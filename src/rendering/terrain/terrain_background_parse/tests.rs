use super::*;

struct FixtureDirectory(PathBuf);

impl FixtureDirectory {
    fn create(label: &str) -> Self {
        let path = std::env::temp_dir().join(format!(
            "game-engine-terrain-isolation-{label}-{}",
            std::process::id()
        ));
        std::fs::create_dir(&path).expect("isolated terrain fixture directory");
        Self(path)
    }
}

impl Drop for FixtureDirectory {
    fn drop(&mut self) {
        if let Err(error) = std::fs::remove_dir_all(&self.0) {
            eprintln!(
                "Failed to remove terrain fixture {}: {error}",
                self.0.display()
            );
        }
    }
}

#[test]
fn no_terrain_objects_preserves_terrain_without_companion_preloads() {
    let directory = FixtureDirectory::create("objects");
    let root = directory.0.join("azeroth_31_48.adt");
    std::fs::copy("data/terrain/777827.adt", &root).expect("cached terrain fixture");
    let mut objects = Vec::new();
    append_chunk(&mut objects, b"FDDM", &[0; 36]);
    append_chunk(&mut objects, b"FDOM", &[0; 64]);
    std::fs::write(directory.0.join("azeroth_31_48_obj0.adt"), objects)
        .expect("object companion fixture");

    let enabled = build_parsed_tile(
        "azeroth".into(),
        31,
        48,
        root.clone(),
        DoodadLod::Full,
        true,
        true,
    )
    .expect("terrain with object companion");
    let placements = enabled.obj_data.as_ref().expect("object companion loaded");
    assert_eq!(placements.doodads.len(), 1);
    assert_eq!(placements.wmos.len(), 1);
    assert_eq!(enabled.preloaded_doodads.len(), 1);
    assert_eq!(enabled.preloaded_wmos.len(), 1);
    assert!(enabled.preloaded_doodads[0].is_none());
    assert!(enabled.preloaded_wmos[0].is_none());

    let disabled = build_parsed_tile("azeroth".into(), 31, 48, root, DoodadLod::Full, false, true)
        .expect("terrain without object loading");
    assert!(disabled.obj_data.is_none());
    assert!(disabled.preloaded_doodads.is_empty());
    assert!(disabled.preloaded_wmos.is_empty());
    assert_eq!(disabled.adt_data.chunks.len(), 256);
    assert_eq!(
        disabled.adt_data.height_grids.len(),
        enabled.adt_data.height_grids.len()
    );
    assert_eq!(
        disabled.adt_data.chunk_positions,
        enabled.adt_data.chunk_positions
    );
    assert_eq!(
        disabled.adt_data.water.is_some(),
        enabled.adt_data.water.is_some()
    );
    assert_eq!(
        disabled.adt_data.chunks[0]
            .mesh
            .attribute(Mesh::ATTRIBUTE_POSITION)
            .unwrap()
            .as_float3(),
        enabled.adt_data.chunks[0]
            .mesh
            .attribute(Mesh::ATTRIBUTE_POSITION)
            .unwrap()
            .as_float3(),
    );
}

#[test]
fn water_skybox_isolation_omits_water_but_preserves_terrain() {
    let directory = FixtureDirectory::create("water");
    let root = directory.0.join("azeroth_31_48.adt");
    std::fs::copy("data/terrain/777827.adt", &root).expect("cached water-bearing terrain");
    let enabled = build_parsed_tile(
        "azeroth".into(),
        31,
        48,
        root.clone(),
        DoodadLod::Full,
        false,
        true,
    )
    .expect("normal water-bearing terrain");
    assert!(enabled.adt_data.water.is_some());

    let disabled = build_parsed_tile(
        "azeroth".into(),
        31,
        48,
        root,
        DoodadLod::Full,
        false,
        false,
    )
    .expect("terrain with water disabled");
    assert!(disabled.adt_data.water.is_none());
    assert_eq!(disabled.adt_data.chunks.len(), 256);
    assert_eq!(disabled.adt_data.height_grids.len(), 256);
    assert_eq!(
        disabled.adt_data.chunk_positions,
        enabled.adt_data.chunk_positions
    );
    assert_eq!(
        disabled.adt_data.center_surface,
        enabled.adt_data.center_surface
    );
}

fn append_chunk(output: &mut Vec<u8>, tag: &[u8; 4], payload: &[u8]) {
    output.extend_from_slice(tag);
    output.extend_from_slice(&(payload.len() as u32).to_le_bytes());
    output.extend_from_slice(payload);
}
