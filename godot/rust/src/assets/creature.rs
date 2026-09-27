//! Local-only creature model companions and textures for the native M2 loader.
use std::{
    collections::BTreeSet,
    fs,
    path::{Path, PathBuf},
};

use game_engine_core::{creature_display_data::CreatureDisplay, m2};
use godot::{classes::Node3D, prelude::*};
use osso_asset_resolver::{AssetResolverConfig, CascListfileResolver};

use super::{appearance::PreparedAppearance, load_model_node_with_appearance};

pub(crate) fn load_creature_model(
    data_root: &Path,
    cache_root: &Path,
    display: &CreatureDisplay,
    appearance: Option<&PreparedAppearance>,
) -> Result<(Gd<Node3D>, PackedInt32Array), String> {
    let resolver = local_resolver(data_root, cache_root);
    let path = cache_model_files(&resolver, data_root, display.model_fdid)?;
    cache_model_textures(&resolver, data_root, &display.skin_fdids, &path)?;
    load_model_node_with_appearance(
        &GString::from(path.to_string_lossy().as_ref()),
        &display.skin_fdids,
        appearance,
    )
}

pub(super) fn local_resolver(data_root: &Path, cache_root: &Path) -> CascListfileResolver {
    CascListfileResolver::new(
        AssetResolverConfig::new()
            .with_data_root(data_root)
            .with_shared_data_root(data_root)
            .with_cache_root(cache_root),
    )
}

pub(super) fn cache_model_files(
    resolver: &CascListfileResolver,
    data_root: &Path,
    model_fdid: u32,
) -> Result<PathBuf, String> {
    let models = data_root.join("models");
    let model_path = models.join(format!("{model_fdid}.m2"));
    let path = cache_required(resolver, model_fdid, &model_path)?;
    let bytes = fs::read(&path).map_err(|error| format!("{}: {error}", path.display()))?;
    let references = m2::parse_asset_references(&bytes)
        .map_err(|error| format!("{}: {error}", path.display()))?;
    let skin_fdid = references
        .skin_fdids
        .first()
        .copied()
        .ok_or_else(|| format!("{}: no primary SFID", path.display()))?;
    cache_required(
        resolver,
        skin_fdid,
        &models.join(format!("{model_fdid}00.skin")),
    )?;
    if let Some(skeleton_fdid) = references.skeleton_fdid {
        cache_required(
            resolver,
            skeleton_fdid,
            &models.join(format!("{model_fdid}.skel")),
        )?;
    }
    Ok(path)
}

fn cache_required(
    resolver: &CascListfileResolver,
    fdid: u32,
    destination: &Path,
) -> Result<PathBuf, String> {
    resolver.ensure_cached(fdid, destination).ok_or_else(|| {
        format!(
            "Failed to cache local CASC FDID {fdid} at {}",
            destination.display()
        )
    })
}

pub(super) fn cache_model_textures(
    resolver: &CascListfileResolver,
    data_root: &Path,
    skin_fdids: &[u32; 3],
    model_path: &Path,
) -> Result<(), String> {
    let model =
        fs::read(model_path).map_err(|error| format!("{}: {error}", model_path.display()))?;
    let stem = model_path.with_extension("");
    let skin_path = PathBuf::from(format!("{}00.skin", stem.display()));
    let skin = fs::read(&skin_path).map_err(|error| format!("{}: {error}", skin_path.display()))?;
    let skeleton_path = stem.with_extension("skel");
    let skeleton = if skeleton_path.exists() {
        Some(
            fs::read(&skeleton_path)
                .map_err(|error| format!("{}: {error}", skeleton_path.display()))?,
        )
    } else {
        None
    };
    let parsed = m2::parse_model_with_skeleton(&model, &skin, skeleton.as_deref())?;
    let textures = creature_texture_fdids(resolver, &parsed, skin_fdids)?;
    for fdid in textures {
        let path = data_root.join("textures").join(format!("{fdid}.blp"));
        // Missing textures remain the native material loader's reported missing FDIDs.
        resolver.ensure_cached(fdid, &path);
    }
    Ok(())
}

fn creature_texture_fdids(
    resolver: &CascListfileResolver,
    model: &m2::Model,
    slots: &[u32; 3],
) -> Result<BTreeSet<u32>, String> {
    let batches =
        m2::resolve_render_batches(model, slots, false, |fdid| resolver.resolve_path(fdid))?;
    let mut textures = BTreeSet::from_iter(slots.iter().copied().filter(|fdid| *fdid != 0));
    for batch in batches {
        textures.extend(batch.texture_fdid);
        textures.extend(batch.texture_2_fdid);
        textures.extend(batch.extra_texture_fdids);
        textures.extend(batch.overlays.into_iter().map(|overlay| overlay.fdid));
    }
    Ok(textures)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    fn cached_data_root() -> PathBuf {
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../data")
    }

    #[test]
    fn acquires_cached_creature_skin_from_sfid_not_creature_texture_slots() {
        let data_root = cached_data_root();
        let resolver = local_resolver(&data_root, &data_root.join("cache"));
        let display = CreatureDisplay {
            model_fdid: 126278,
            skin_fdids: [126280, 0, 0],
            scale_milli: 1000,
        };
        let path = cache_model_files(&resolver, &data_root, display.model_fdid).unwrap();
        assert_eq!(path, data_root.join("models/126278.m2"));
        let model = std::fs::read(&path).unwrap();
        let skin = std::fs::read(data_root.join("models/12627800.skin")).unwrap();
        assert!(
            !game_engine_core::m2::parse_model(&model, &skin)
                .unwrap()
                .submeshes
                .is_empty()
        );
    }

    #[test]
    fn collects_authored_batch_textures_and_explicit_creature_slots() {
        let data_root = cached_data_root();
        let resolver = local_resolver(&data_root, &data_root.join("cache"));
        let model = std::fs::read(data_root.join("models/126278.m2")).unwrap();
        let skin = std::fs::read(data_root.join("models/12627800.skin")).unwrap();
        let parsed = m2::parse_model(&model, &skin).unwrap();
        let textures = creature_texture_fdids(&resolver, &parsed, &[987654321, 0, 0]).unwrap();
        assert!(textures.contains(&987654321));
        assert!(textures.contains(&126280));
        assert!(!textures.contains(&0));
    }

    #[test]
    fn acquires_cached_external_skeleton_from_skid() {
        let data_root = cached_data_root();
        let resolver = local_resolver(&data_root, &data_root.join("cache"));
        let display = CreatureDisplay {
            model_fdid: 1011653,
            skin_fdids: [0; 3],
            scale_milli: 1000,
        };
        let path = cache_model_files(&resolver, &data_root, display.model_fdid).unwrap();
        let model = std::fs::read(path).unwrap();
        let skin = std::fs::read(data_root.join("models/101165300.skin")).unwrap();
        let skel = std::fs::read(data_root.join("models/1011653.skel")).unwrap();
        assert!(
            !game_engine_core::m2::parse_model_with_skeleton(&model, &skin, Some(&skel))
                .unwrap()
                .bones
                .is_empty()
        );
    }
}
