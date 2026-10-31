//! Map-object transports (TrinityCore `GAMEOBJECT_TYPE_MAP_OBJ_TRANSPORT`, SharedDefines.h):
//! their GameObjectDisplayInfo file is a WMO root, drawn with the ADT WMO renderer and
//! made solid with its authored MOPY-collidable faces under the object's own node, so
//! game-object floor and wall rays follow the node wherever it is placed.
use std::path::Path;

use glam::Vec3;
use godot::{classes::ConcavePolygonShape3D, prelude::*};
use osso_asset_resolver::CascListfileResolver;

use crate::{
    lighting::TerrainLight,
    wmo::{
        assets::{NativeWmoAsset, read_wmo_with_product},
        scene::build_wmo_node,
    },
};

pub(crate) const GAMEOBJECT_TYPE_MAP_OBJ_TRANSPORT: u8 = 15;

/// MODD set 0 is the WMO's default global set, active in every placement
/// (`wmo/global.rs` adds the placement's own set on top; a game object has none).
const DEFAULT_DOODAD_SETS: [u16; 1] = [0];

/// The WMO root `fdid` and its groups; the local listfile must name it a `.wmo`.
fn read_transport_wmo(
    data_root: &Path,
    fdid: u32,
) -> Result<(NativeWmoAsset, CascListfileResolver), String> {
    let resolver = crate::assets::creature::wmo_resolver(
        data_root,
        game_engine_core::asset_product::AssetProduct::Retail,
        fdid,
    )?;
    let path = resolver
        .resolve_path(fdid)
        .ok_or_else(|| format!("Transport display FDID {fdid} is not in the listfile"))?;
    if !path.ends_with(".wmo") {
        return Err(format!(
            "Transport display FDID {fdid} is {path}, not a WMO root"
        ));
    }
    let asset = read_wmo_with_product(
        &resolver,
        data_root,
        fdid,
        Some(game_engine_core::asset_product::AssetProduct::Retail),
    )?;
    Ok((asset, resolver))
}

/// Every MOPY-collidable triangle of the WMO, flattened, in WMO-local engine axes.
fn collision_faces(asset: &NativeWmoAsset) -> Vec<Vec3> {
    asset
        .groups
        .iter()
        .flat_map(|group| group.collision.collidable_triangles())
        .flatten()
        .collect()
}

/// The drawn WMO of a transport display with its authored collision body attached.
pub(super) fn load_transport_wmo(
    data_root: &Path,
    fdid: u32,
    light: Option<&TerrainLight>,
) -> Result<Gd<Node3D>, String> {
    let (asset, resolver) = read_transport_wmo(data_root, fdid)?;
    let built = build_wmo_node(
        &asset,
        &resolver,
        data_root,
        Some(game_engine_core::asset_product::AssetProduct::Retail),
        &DEFAULT_DOODAD_SETS,
        light,
    )?;
    if !built.batch_errors.is_empty() {
        built.node.free();
        return Err(format!(
            "Transport WMO {fdid}: {}",
            built.batch_errors.join("; ")
        ));
    }
    let faces: PackedVector3Array = collision_faces(&asset)
        .into_iter()
        .map(|corner| Vector3::from_array(corner.to_array()))
        .collect();
    let mut model = built.node;
    if faces.is_empty() {
        model.free();
        return Err(format!("Transport WMO {fdid} has no collidable faces"));
    }
    let mut shape = ConcavePolygonShape3D::new_gd();
    // Rays hit faces from either side, as WMO terrain collision does.
    shape.set_backface_collision_enabled(true);
    shape.set_faces(&faces);
    // The same body game-object M2s carry, so the replicated collision state toggles it.
    crate::terrain::doodad_collision::attach_collision(&mut model, &shape);
    Ok(model)
}

#[cfg(test)]
pub(crate) mod tests {
    use super::*;
    use crate::game_objects::parse_displays;
    use std::path::PathBuf;

    fn data_root() -> PathBuf {
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../data")
    }

    /// Two-sided ray/triangle test returning the hit height of a downward ray.
    fn floor_below(faces: &[Vec3], from: Vec3) -> Option<f32> {
        faces
            .chunks_exact(3)
            .filter_map(|tri| {
                let (a, b, c) = (tri[0], tri[1], tri[2]);
                let normal = (b - a).cross(c - a).normalize_or_zero();
                if normal.y.abs() < shared::ground::min_walkable_normal_y() {
                    return None;
                }
                let t = normal.dot(a - from) / -normal.y;
                let point = from - Vec3::Y * t;
                let inside = [(a, b), (b, c), (c, a)]
                    .iter()
                    .all(|&(p, q)| (q - p).cross(point - p).dot(normal) >= -1e-4);
                (t >= 0.0 && inside).then_some(point.y)
            })
            .reduce(f32::max)
    }

    pub(crate) fn receipt_only_fixture() -> PathBuf {
        use std::sync::atomic::{AtomicUsize, Ordering};
        static NEXT: AtomicUsize = AtomicUsize::new(0);
        let data = std::env::temp_dir().join(format!(
            "wmo-receipt-{}-{}",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        ));
        let key = "d718abcc42604d8efbd35aecfa5bb75b";
        std::fs::create_dir_all(data.join(format!("products/wow/{key}/models"))).unwrap();
        std::fs::create_dir_all(data.join(format!("products/wow/{key}/textures"))).unwrap();
        std::fs::create_dir_all(data.join("cache")).unwrap();
        std::fs::create_dir_all(data.join("db2/active")).unwrap();
        std::fs::write(
            data.join("community-listfile.csv"),
            "307812;world/wmo/northrend/buildings/human/nd_alliancegunship_bg.wmo\n",
        )
        .unwrap();
        std::fs::write(
            data.join("db2/active/GameObjectDisplayInfo.csv"),
            "ID,FileDataID\n9002,307812\n",
        )
        .unwrap();
        let source = data_root();
        let resolver = crate::assets::creature::local_resolver(&source);
        let original = crate::wmo::assets::read_wmo(&resolver, &source, 307812).unwrap();
        let mut assets = Vec::new();
        let mut flat_texture_written = false;
        for (fdid, kind, directory) in [307812, 318242, 318243, 318244, 318245, 318246]
            .into_iter()
            .map(|fdid| (fdid, "wmo", "models"))
            .chain(
                crate::wmo::scene::texture_fdids(&original)
                    .into_iter()
                    .map(|fdid| (fdid, "blp", "textures")),
            )
        {
            let key = if fdid == 318242 || kind == "blp" {
                "11111111111111111111111111111111"
            } else {
                key
            };
            let bytes = std::fs::read(source.join(format!("{directory}/{fdid}.{kind}"))).unwrap();
            if kind == "blp" && !flat_texture_written {
                let path = data.join(format!("textures/{fdid}.blp"));
                std::fs::create_dir_all(path.parent().unwrap()).unwrap();
                std::fs::write(path, &bytes).unwrap();
                flat_texture_written = true;
                continue;
            }
            let path = format!("products/wow/{key}/{directory}/{fdid}.{kind}");
            std::fs::create_dir_all(data.join(&path).parent().unwrap()).unwrap();
            std::fs::write(data.join(&path), &bytes).unwrap();
            assets.push(serde_json::json!({"product":"wow", "build_key":key, "build":"fixture", "fdid":fdid, "kind":kind, "path":path, "bytes":bytes.len(), "sha256":"a".repeat(64), "content_key":"b".repeat(32)}));
        }
        std::fs::write(
            data.join("cache/model-asset-index.json"),
            serde_json::json!({"version":1,"assets":assets}).to_string(),
        )
        .unwrap();
        data
    }

    #[test]
    fn game_object_display_loads_receipt_only_wmo() {
        let data = receipt_only_fixture();
        let displays = parse_displays(
            &std::fs::read_to_string(data.join("db2/active/GameObjectDisplayInfo.csv")).unwrap(),
        )
        .unwrap();
        let (asset, _) = read_transport_wmo(&data, displays[&9002]).unwrap();
        assert_eq!(asset.groups.len(), 5);
        assert!(!collision_faces(&asset).is_empty());
        assert!(!data.join("models/307812.wmo").exists());
        std::fs::remove_dir_all(data).unwrap();
    }

    #[test]
    fn battleground_gunship_displays_name_their_authored_wmo_roots() {
        let text =
            std::fs::read_to_string(data_root().join("db2/active/GameObjectDisplayInfo.csv"))
                .unwrap();
        let displays = parse_displays(&text).unwrap();
        // nd_alliancegunship_bg.wmo and nd_hordegunship_bg.wmo (community listfile).
        assert_eq!(displays.get(&9002), Some(&307812));
        assert_eq!(displays.get(&9001), Some(&307818));
    }

    #[test]
    fn alliance_gunship_draws_its_wmo_groups_and_carries_its_authored_deck() {
        let data = data_root();
        let (asset, _) = read_transport_wmo(&data, 307812).unwrap();
        // nd_alliancegunship_bg_000..004.wmo, FDIDs 318242..318246.
        assert_eq!(asset.groups.len(), 5);
        assert!(
            asset
                .groups
                .iter()
                .filter(|group| group.batches.iter().any(|batch| !batch.indices.is_empty()))
                .count()
                >= 1
        );
        assert!(!crate::wmo::scene::texture_fdids(&asset).is_empty());
        let local = collision_faces(&asset);
        assert_eq!(local.len() % 3, 0);
        // Authored MOPY-collidable deck faces of group 002 (318244): file (0, 0) at
        // z 21.0096 and file (-30, 0) at z 20.6032; local engine axes are [x, z, -y].
        let above = |x: f32| Vec3::new(x, 30.0, 0.0);
        assert!((floor_below(&local, above(0.0)).unwrap() - 21.0096).abs() < 0.01);
        assert!((floor_below(&local, above(-30.0)).unwrap() - 20.6032).abs() < 0.01);
    }

    #[test]
    fn m2_displays_are_not_read_as_transport_wmos() {
        let data = data_root();
        // Goldshire Mailbox display 1907 is the M2 FDID 199999.
        let error = read_transport_wmo(&data, 199999)
            .err()
            .expect("an M2 FDID is not a WMO root");
        assert!(error.contains("199999"), "{error}");
        assert!(error.contains(".m2"), "{error}");
    }

    #[test]
    fn map_object_transports_render_without_an_interaction_cursor() {
        assert!(crate::game_objects::game_object_is_renderable(
            GAMEOBJECT_TYPE_MAP_OBJ_TRANSPORT
        ));
        assert!(
            crate::game_objects::game_object_cursor(GAMEOBJECT_TYPE_MAP_OBJ_TRANSPORT).is_none()
        );
    }
}
