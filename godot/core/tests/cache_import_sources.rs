use game_engine_core::{char_texture_cache, creature_display_cache, customization_cache};
use rusqlite::Connection;
use std::path::PathBuf;
use std::sync::atomic::{AtomicUsize, Ordering};

struct Fixture(PathBuf);
impl Fixture {
    fn new() -> Self {
        static NEXT: AtomicUsize = AtomicUsize::new(0);
        let root = std::env::temp_dir().join(format!(
            "cache-import-sources-{}-{}",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        ));
        std::fs::create_dir_all(root.join("db2/active")).unwrap();
        Self(root)
    }
    fn write(&self, name: &str, root: &str, retail: &str) {
        std::fs::write(self.0.join(name), root).unwrap();
        std::fs::write(self.0.join("db2/active").join(name), retail).unwrap();
    }
}
impl Drop for Fixture {
    fn drop(&mut self) {
        std::fs::remove_dir_all(&self.0).unwrap();
    }
}

#[test]
fn creature_import_follows_active_export_after_patch() {
    let fixture = Fixture::new();
    let base = fixture.0.join("db2");
    std::fs::remove_dir_all(base.join("active")).unwrap();
    for (build, display, fdid) in [("12.1.0.69933", 4, 9001), ("12.1.5.70340", 5, 9002)] {
        let source = base.join(build);
        std::fs::create_dir_all(&source).unwrap();
        std::fs::write(source.join("CreatureDisplayInfo.csv"), format!("ID,ModelID,CreatureModelScale,PetInstanceScale,CreatureModelAlpha,TextureVariationFileDataID_0,TextureVariationFileDataID_1,TextureVariationFileDataID_2,TextureVariationFileDataID_3\n{display},7,1,1,255,0,0,0,0\n")).unwrap();
        std::fs::write(
            source.join("CreatureModelData.csv"),
            format!("ID,FileDataID,ModelScale,CreatureGeosetDataID\n7,{fdid},1,0\n"),
        )
        .unwrap();
    }
    let active = base.join("active");
    for (build, expected) in [("12.1.0.69933", 9001), ("12.1.5.70340", 9002)] {
        if active.exists() {
            std::fs::remove_file(&active).unwrap();
        }
        std::os::unix::fs::symlink(build, &active).unwrap();
        let cache = creature_display_cache::import_creature_display_cache(&fixture.0).unwrap();
        let conn = Connection::open(cache).unwrap();
        let actual: u32 = conn
            .query_row("SELECT model_fdid FROM creature_displays", [], |r| r.get(0))
            .unwrap();
        assert_eq!(actual, expected, "active export {build}");
    }
}

#[test]
fn outfit_import_uses_active_gear_without_a_legacy_starter_table() {
    let fixture = Fixture::new();
    let source = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../data/db2/active");
    for name in [
        "ItemModifiedAppearance",
        "ItemAppearance",
        "ItemDisplayInfo",
        "TextureFileData",
        "ItemDisplayInfoMaterialRes",
        "ModelFileData",
    ] {
        std::os::unix::fs::symlink(
            source.join(format!("{name}.csv")).canonicalize().unwrap(),
            fixture.0.join("db2/active").join(format!("{name}.csv")),
        )
        .unwrap();
    }
    let path = game_engine_core::outfit_catalog_db::import_outfit_links_cache(&fixture.0).unwrap();
    let conn = Connection::open(path).unwrap();
    let display: u32 = conn.query_row("SELECT iam.display_info_id FROM item_modified_appearance_map ima JOIN item_appearance_map iam ON iam.appearance_id=ima.appearance_id WHERE ima.item_id=25", [], |r| r.get(0)).unwrap();
    assert_eq!(display, 1542);
    let model: u32 = conn.query_row("SELECT file_data_id FROM model_to_fdid WHERE model_resource_id=16810 ORDER BY model_order LIMIT 1", [], |r| r.get(0)).unwrap();
    assert_eq!(model, 148132);
}

#[test]
fn creature_import_uses_pinned_rows_and_invalidates_root_source_receipts() {
    let fixture = Fixture::new();
    let header = "ID,ModelID,CreatureModelScale,PetInstanceScale,CreatureModelAlpha,TextureVariationFileDataID_0,TextureVariationFileDataID_1,TextureVariationFileDataID_2,TextureVariationFileDataID_3\n";
    fixture.write(
        "CreatureDisplayInfo.csv",
        &format!("{header}4,7,1,1,255,11,0,0,0\n"),
        &format!("{header}145204,7,2,0.7,255,22,0,0,0\n145205,7,1,1,255,23,0,0,0\n"),
    );
    fixture.write(
        "CreatureModelData.csv",
        "ID,FileDataID,ModelScale,CreatureGeosetDataID\n7,9001,1,0\n",
        "ID,FileDataID,ModelScale,CreatureGeosetDataID\n7,9002,1.25,0\n",
    );
    let path = creature_display_cache::import_creature_display_cache(&fixture.0).unwrap();
    let conn = Connection::open(&path).unwrap();
    let rows: Vec<(u32, u32, u32, u32)> = conn.prepare("SELECT display_id, model_fdid, scale_milli, pet_instance_scale_milli FROM creature_displays ORDER BY display_id").unwrap().query_map([], |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?, r.get(3)?))).unwrap().map(Result::unwrap).collect();
    assert_eq!(
        rows,
        vec![(145204, 9002, 2500, 700), (145205, 9002, 1250, 1000)]
    );
    // Emulate a cache produced by the old root-path importer, even with matching mtimes.
    conn.execute(
        "UPDATE source_files SET path = replace(path, 'db2/active/', '')",
        [],
    )
    .unwrap();
    conn.execute(
        "DELETE FROM creature_displays WHERE display_id = 145205",
        [],
    )
    .unwrap();
    drop(conn);
    creature_display_cache::import_creature_display_cache(&fixture.0).unwrap();
    let conn = Connection::open(&path).unwrap();
    assert_eq!(
        conn.query_row("SELECT count(*) FROM creature_displays", [], |r| r
            .get::<_, u32>(0))
            .unwrap(),
        2
    );
    drop(conn);
    // A missing pinned input must fail even when a usable cache and root CSV remain.
    std::fs::remove_file(fixture.0.join("db2/active/CreatureModelData.csv")).unwrap();
    let error = creature_display_cache::import_creature_display_cache(&fixture.0).unwrap_err();
    assert!(
        error.contains("db2/active/CreatureModelData.csv"),
        "{error}"
    );
}

#[test]
fn texture_import_ignores_root_rows_and_invalidates_root_source_receipts() {
    let fixture = Fixture::new();
    for (name, header, row) in [
        (
            "ChrModelTextureLayer.csv",
            "TextureType,Layer,BlendMode,TextureSectionTypeBitMask,ChrModelTextureTargetID_0,CharComponentTextureLayoutsID\n",
            "1,0,0,1,6,103\n",
        ),
        (
            "CharComponentTextureSections.csv",
            "CharComponentTextureLayoutID,SectionType,X,Y,Width,Height\n",
            "103,2,16,32,128,256\n",
        ),
        (
            "CharComponentTextureLayouts.csv",
            "ID,Width,Height\n",
            "103,2048,1024\n",
        ),
        (
            "ChrModelMaterial.csv",
            "CharComponentTextureLayoutsID,TextureType,Width,Height\n",
            "103,1,2048,1024\n",
        ),
    ] {
        fixture.write(name, header, &format!("{header}{row}"));
    }
    let path = char_texture_cache::import_char_texture_cache(&fixture.0).unwrap();
    let conn = Connection::open(&path).unwrap();
    let size: (u32, u32) = conn
        .query_row(
            "SELECT width, height FROM layouts WHERE id = 103",
            [],
            |r| Ok((r.get(0)?, r.get(1)?)),
        )
        .unwrap();
    assert_eq!(size, (2048, 1024));
    conn.execute(
        "UPDATE source_files SET source = replace(source, 'db2/active/', '')",
        [],
    )
    .unwrap();
    conn.execute("DELETE FROM layouts", []).unwrap();
    drop(conn);
    char_texture_cache::import_char_texture_cache(&fixture.0).unwrap();
    let conn = Connection::open(&path).unwrap();
    assert_eq!(
        conn.query_row("SELECT count(*) FROM layouts", [], |r| r.get::<_, u32>(0))
            .unwrap(),
        1
    );
}

#[test]
fn importers_reject_root_only_csvs_without_creating_caches() {
    let fixture = Fixture::new();
    for name in [
        "ChrModel",
        "ChrCustomizationOption",
        "ChrCustomizationChoice",
        "ChrCustomizationElement",
        "ChrCustomizationMaterial",
        "ChrCustomizationGeoset",
        "CharHairGeosets",
        "ChrCustomizationCategory",
        "ChrCustomizationSkinnedModel",
        "TextureFileData",
        "ChrRaceXChrModel",
        "ChrRaces",
        "ChrModelTextureLayer",
        "CharComponentTextureSections",
        "CharComponentTextureLayouts",
        "ChrModelMaterial",
        "ItemModifiedAppearance",
        "ItemAppearance",
        "ItemDisplayInfo",
        "ItemDisplayInfoMaterialRes",
        "ModelFileData",
    ] {
        std::fs::write(fixture.0.join(format!("{name}.csv")), "ID\n1\n").unwrap();
    }
    for importer in [
        customization_cache::import_customization_cache,
        char_texture_cache::import_char_texture_cache,
        game_engine_core::outfit_catalog_db::import_outfit_links_cache,
    ] {
        let error = importer(&fixture.0).unwrap_err();
        assert!(error.contains("db2/active/"), "{error}");
    }
    assert!(!fixture.0.join("cache").exists());
}
