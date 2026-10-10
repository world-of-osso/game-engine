//! Complete root/group WMO payload acquisition through the shared local CASC resolver.

use std::{
    collections::HashSet,
    fs,
    path::{Path, PathBuf},
    sync::Arc,
};

use game_engine_core::{adt, asset::wmo_format::fog::WmoFogVolume, wmo};
use osso_asset_resolver::CascListfileResolver;
use shared::ground::WmoGroupCollision;

use super::doodad_light::{DoodadLight, doodad_light};

/// A MODD doodad a WMO placement draws, and its light.
pub(crate) type LitDoodad = (wmo::WmoDoodad, DoodadLight);

pub(crate) struct NativeWmoAsset {
    pub root_fdid: u32,
    pub root: wmo::WmoRootData,
    pub groups: Vec<NativeWmoGroup>,
}

impl NativeWmoAsset {
    /// The MODD doodads a placement with active `doodad_sets` draws, with their light.
    pub fn doodads(&self, doodad_sets: &[u16]) -> Vec<LitDoodad> {
        wmo::placed_doodads(
            &self.root,
            self.groups.iter().map(|group| {
                (
                    group.index as u16,
                    group.group.geometry.doodad_refs.as_slice(),
                )
            }),
            doodad_sets,
        )
        .into_iter()
        .map(|doodad| {
            let light = doodad_light(self, &doodad, doodad_sets);
            (doodad, light)
        })
        .collect()
    }

    pub fn shadow_groups(&self) -> ShadowGroups {
        ShadowGroups(
            self.groups
                .iter()
                .filter(|group| group_casts_shadow(group.group.header.flags))
                .map(|group| group.index as u16)
                .collect(),
        )
    }
}

/// Whether a group with MOGP `flags` casts directional shadows: EXTERIOR (0x8) or
/// EXTERIOR_LIT (0x40). Retail collects only those groups, and the MODR doodads they
/// reference, into its shadow maps (solarityclient
/// `terrain_frame/world_model/shadow.rs` `prepare_shadow_draws`, `group.flags() & 0x48`).
pub(crate) fn group_casts_shadow(flags: u32) -> bool {
    flags & 0x48 != 0
}

/// The groups of one WMO that cast directional shadows.
#[derive(Default)]
pub(crate) struct ShadowGroups(HashSet<u16>);

impl ShadowGroups {
    /// Whether a doodad referenced by `groups` (their MODR) casts: one of them does.
    pub fn casts(&self, groups: &[u16]) -> bool {
        groups.iter().any(|group| self.0.contains(group))
    }
}

pub(crate) struct NativeWmoGroup {
    pub index: u32,
    pub group: wmo::Group,
    pub batches: Vec<wmo::WmoMeshBatch>,
    pub collision: Arc<WmoGroupCollision>,
}

/// The asset's MFOG records with each loaded group's MOGP fog references and portals.
pub(crate) fn wmo_fog_volume(asset: &NativeWmoAsset) -> WmoFogVolume {
    WmoFogVolume::new(
        &asset.root,
        asset
            .groups
            .iter()
            .map(|group| (group.index as usize, &group.group.header)),
    )
}

pub(crate) fn read_placement(
    resolver: &CascListfileResolver,
    data_root: &Path,
    placement: &adt::WmoPlacement,
) -> Result<NativeWmoAsset, String> {
    read_wmo(
        resolver,
        data_root,
        resolve_placement_fdid(resolver, placement)?,
    )
}

/// WMO root `root_fdid` and its groups, extracted from local CASC and parsed; no engine
/// calls, so a worker can read it.
pub(crate) fn read_wmo(
    resolver: &CascListfileResolver,
    data_root: &Path,
    root_fdid: u32,
) -> Result<NativeWmoAsset, String> {
    let root = read_root(resolver, data_root, root_fdid)?;
    let group_fdids = resolve_group_fdids(resolver, root_fdid, &root)?;
    let mut groups = Vec::with_capacity(group_fdids.len());
    for (index, fdid) in group_fdids.into_iter().enumerate() {
        groups.push(read_group(
            resolver, data_root, root_fdid, &root, index, fdid,
        )?);
    }
    Ok(NativeWmoAsset {
        root_fdid,
        root,
        groups,
    })
}

pub(crate) fn resolve_placement_fdid(
    resolver: &CascListfileResolver,
    placement: &adt::WmoPlacement,
) -> Result<u32, String> {
    match placement.fdid {
        Some(fdid) => Ok(fdid),
        None => {
            let path = placement
                .path
                .as_deref()
                .ok_or("WMO placement has no FDID or path")?;
            resolver
                .lookup_path(path)
                .ok_or_else(|| format!("WMO root {path} not in listfile"))
        }
    }
}

fn read_root(
    resolver: &CascListfileResolver,
    data_root: &Path,
    root_fdid: u32,
) -> Result<wmo::WmoRootData, String> {
    let io = crate::profile::span(|| "phase.asset_io.wmo_root".to_owned());
    let root_path = read_required_wmo(resolver, data_root, root_fdid, "root")?;
    let root_bytes = fs::read(&root_path)
        .map_err(|error| format!("WMO root {}: {error}", root_path.display()))?;
    drop(io);
    let _parse = crate::profile::span(|| "phase.wmo_parse.root".to_owned());
    wmo::parse_root(&root_bytes)
        .map_err(|error| format!("WMO root {}: {error}", root_path.display()))
}

fn read_group(
    resolver: &CascListfileResolver,
    data_root: &Path,
    root_fdid: u32,
    root: &wmo::WmoRootData,
    index: usize,
    fdid: u32,
) -> Result<NativeWmoGroup, String> {
    let context = format!("group {index} of root FDID {root_fdid}");
    let io = crate::profile::span(|| "phase.asset_io.wmo_group".to_owned());
    let path = read_required_wmo(resolver, data_root, fdid, &context)?;
    let bytes =
        fs::read(&path).map_err(|error| format!("WMO {context} {}: {error}", path.display()))?;
    drop(io);
    let parse = crate::profile::span(|| "phase.wmo_parse.group".to_owned());
    let group = wmo::parse_group(&bytes)
        .map_err(|error| format!("WMO {context} {}: {error}", path.display()))?;
    let collision = Arc::new(
        WmoGroupCollision::parse(&bytes)
            .map_err(|error| format!("WMO {context} {} collision: {error}", path.display()))?,
    );
    drop(parse);
    let _mesh = crate::profile::span(|| "phase.mesh_build.wmo_worker".to_owned());
    let batches = group.batches(Some(root));
    Ok(NativeWmoGroup {
        index: index as u32,
        group,
        batches,
        collision,
    })
}

fn resolve_group_fdids(
    resolver: &CascListfileResolver,
    root_fdid: u32,
    root: &wmo::WmoRootData,
) -> Result<Vec<u32>, String> {
    if root.group_file_data_ids.len() >= root.n_groups as usize {
        return root.group_file_data_ids[..root.n_groups as usize]
            .iter()
            .enumerate()
            .map(|(index, &fdid)| {
                (fdid != 0)
                    .then_some(fdid)
                    .ok_or_else(|| format!("WMO root FDID {root_fdid} group {index}: GFID is zero"))
            })
            .collect();
    }
    let root_path = resolver.resolve_path(root_fdid).ok_or_else(|| {
        format!("WMO root FDID {root_fdid}: no complete GFID and not in listfile")
    })?;
    let base = root_path.trim_end_matches(".wmo");
    (0..root.n_groups)
        .map(|index| {
            let group_path = format!("{base}_{index:03}.wmo");
            resolver.lookup_path(&group_path).ok_or_else(|| {
                format!("WMO root FDID {root_fdid} group {index}: {group_path} not in listfile")
            })
        })
        .collect()
}

fn read_required_wmo(
    resolver: &CascListfileResolver,
    data_root: &Path,
    fdid: u32,
    context: &str,
) -> Result<PathBuf, String> {
    let cache_path = data_root.join("models").join(format!("{fdid}.wmo"));
    resolver
        .ensure_cached_checked(fdid, &cache_path)
        .map_err(|error| format!("WMO {context}: {error}"))
}

#[cfg(test)]
mod tests {
    use std::{
        fs,
        path::{Path, PathBuf},
    };

    use game_engine_core::adt::WmoPlacement;
    use osso_asset_resolver::{AssetResolverConfig, CascListfileResolver};

    use super::{read_placement, resolve_group_fdids};

    fn placement(fdid: Option<u32>, path: Option<&str>) -> WmoPlacement {
        WmoPlacement {
            name_id: 0,
            unique_id: 0,
            position: [0.0; 3],
            rotation: [0.0; 3],
            extents_min: [0.0; 3],
            extents_max: [0.0; 3],
            flags: 0,
            doodad_set: 0,
            name_set: 0,
            scale: 1.0,
            fdid,
            path: path.map(str::to_owned),
        }
    }

    fn cached_data_root() -> PathBuf {
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../data")
    }

    fn resolver(data_root: &Path) -> CascListfileResolver {
        CascListfileResolver::new(
            AssetResolverConfig::new()
                .with_data_root(data_root)
                .with_shared_data_root(data_root),
        )
    }

    fn temporary_data_root() -> PathBuf {
        std::env::temp_dir().join(format!(
            "native-wmo-assets-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ))
    }

    #[test]
    fn loads_all_abbey_groups_from_path_placement() {
        let data_root = cached_data_root();
        let asset = read_placement(
            &resolver(&data_root),
            &data_root,
            &placement(
                None,
                Some("world/wmo/azeroth/buildings/nsabbey/nsabbey.wmo"),
            ),
        )
        .expect("authored Abbey WMO");
        assert_eq!(asset.root_fdid, 107_074);
        assert_eq!(asset.root.n_groups, 13);
        assert_eq!(asset.groups.len(), 13);
        for (index, group) in asset.groups.iter().enumerate() {
            assert_eq!(group.index as usize, index);
            assert!(!group.group.geometry.vertices.is_empty());
            assert!(!group.batches.is_empty());
            assert!(group.batches.iter().any(|batch| !batch.indices.is_empty()));
        }
    }

    #[test]
    fn loads_stockade_gfid_groups_from_fdid_placement() {
        let data_root = cached_data_root();
        let asset = read_placement(
            &resolver(&data_root),
            &data_root,
            &placement(Some(108_631), None),
        )
        .expect("authored Stockade WMO");
        assert_eq!(asset.root_fdid, 108_631);
        assert_eq!(asset.root.n_groups, 27);
        assert_eq!(asset.root.group_file_data_ids.len(), 81);
        assert_eq!(asset.groups.len(), 27);
        for (index, group) in asset.groups.iter().enumerate() {
            assert_eq!(group.index as usize, index);
            assert!(!group.group.geometry.vertices.is_empty());
            assert!(!group.batches.is_empty());
        }
        assert_eq!(asset.root.group_file_data_ids[26], 2_058_163);
    }

    #[test]
    fn legacy_root_without_gfid_loads_all_named_groups() {
        let data_root = cached_data_root();
        let temp = temporary_data_root();
        let models = temp.join("models");
        fs::create_dir_all(&models).unwrap();
        let mut root = fs::read(data_root.join("models/107074.wmo")).unwrap();
        let tag = root.windows(4).position(|bytes| bytes == b"DIFG").unwrap();
        root[tag..tag + 4].copy_from_slice(b"XXXX");
        fs::write(models.join("107074.wmo"), root).unwrap();
        let stem = "world/wmo/azeroth/buildings/nsabbey/nsabbey";
        let mut listfile = format!("107074;{stem}.wmo\n");
        for index in 0..13 {
            let fdid = 107_075 + index;
            listfile.push_str(&format!("{fdid};{stem}_{index:03}.wmo\n"));
            fs::copy(
                data_root.join(format!("models/{fdid}.wmo")),
                models.join(format!("{fdid}.wmo")),
            )
            .unwrap();
        }
        fs::write(temp.join("community-listfile.csv"), listfile).unwrap();
        let asset = read_placement(&resolver(&temp), &temp, &placement(Some(107_074), None))
            .expect("legacy authored group names");
        assert_eq!(asset.groups.len(), 13);
        assert_eq!(
            resolve_group_fdids(&resolver(&temp), 107_074, &asset.root).unwrap(),
            (107_075..=107_087).collect::<Vec<_>>()
        );
        fs::remove_dir_all(temp).unwrap();
    }

    #[test]
    fn missing_gfid_slot_rejects_incomplete_root() {
        let temp = temporary_data_root();
        let models = temp.join("models");
        fs::create_dir_all(&models).unwrap();
        let mut root = fs::read(cached_data_root().join("models/107074.wmo")).unwrap();
        let tag = root.windows(4).position(|bytes| bytes == b"DIFG").unwrap();
        root[tag + 8..tag + 12].copy_from_slice(&0_u32.to_le_bytes());
        fs::write(models.join("107074.wmo"), root).unwrap();
        let error = read_placement(&resolver(&temp), &temp, &placement(Some(107_074), None))
            .err()
            .expect("missing group FDID");
        assert!(error.contains("group 0"), "{error}");
        assert!(error.contains("GFID is zero"), "{error}");
        fs::remove_dir_all(temp).unwrap();
    }

    #[test]
    fn unresolved_root_path_is_an_error() {
        let data_root = cached_data_root();
        let error = read_placement(
            &resolver(&data_root),
            &data_root,
            &placement(None, Some("world/wmo/nonexistent/missing.wmo")),
        )
        .err()
        .expect("unresolved root");
        assert!(
            error.contains("world/wmo/nonexistent/missing.wmo"),
            "{error}"
        );
    }

    #[test]
    fn corrupt_root_and_required_group_are_contextual_errors() {
        let temp = temporary_data_root();
        let models = temp.join("models");
        fs::create_dir_all(&models).unwrap();
        let root_path = models.join("107074.wmo");
        fs::write(&root_path, b"broken").unwrap();
        let resolver = resolver(&temp);
        let abbey = placement(Some(107_074), None);
        let error = read_placement(&resolver, &temp, &abbey)
            .err()
            .expect("corrupt root");
        assert!(error.contains("107074.wmo"), "{error}");

        fs::copy(cached_data_root().join("models/107074.wmo"), &root_path).unwrap();
        fs::write(models.join("107075.wmo"), b"broken").unwrap();
        let error = read_placement(&resolver, &temp, &abbey)
            .err()
            .expect("corrupt required group");
        assert!(error.contains("group 0"), "{error}");
        assert!(error.contains("107075.wmo"), "{error}");
        fs::remove_dir_all(&temp).unwrap();
    }
}
