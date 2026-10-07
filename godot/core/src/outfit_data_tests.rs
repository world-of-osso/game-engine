use std::path::Path;

use crate::outfit_data::OutfitData;

mod baked_display {
    use super::*;

    fn write_catalog(label: &str) -> std::path::PathBuf {
        let root = Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../../target")
            .join(format!("baked-display-{}-{label}", std::process::id()));
        let gear = root.join("db2/1.60.1.70205");
        std::fs::create_dir_all(gear.join("items")).unwrap();
        for (name, content) in [
            (
                "items/ItemModifiedAppearance.csv",
                "ItemID,ItemAppearanceID\n",
            ),
            ("items/ItemAppearance.csv", "ID,ItemDisplayInfoID\n"),
            (
                "ItemDisplayInfo.csv",
                "ID,ModelResourcesID_0,ModelResourcesID_1,ModelMaterialResourcesID_0,ModelMaterialResourcesID_1,GeosetGroup_0,GeosetGroup_1,GeosetGroup_2,GeosetGroup_3,GeosetGroup_4,GeosetGroup_5,HelmetGeosetVis_0,HelmetGeosetVis_1\n10,100,0,200,0,0,0,0,0,0,0,0,0\n11,101,0,200,0,0,0,0,0,0,0,0,0\n12,100,0,201,0,0,0,0,0,0,0,0,0\n13,100,0,200,0,0,0,0,0,0,0,0,0\n",
            ),
            (
                "ItemDisplayInfoMaterialRes.csv",
                "ItemDisplayInfoID,ComponentSection,MaterialResourcesID\n10,4,300\n13,4,301\n",
            ),
            (
                "ModelFileData.csv",
                "FileDataID,ModelResourcesID\n1000,100\n1001,100\n",
            ),
            (
                "TextureFileData.csv",
                "FileDataID,UsageType,MaterialResourcesID\n2000,0,200\n2001,0,200\n3000,0,301\n",
            ),
            (
                "ComponentModelFileData.csv",
                "ID,GenderIndex,ClassID,RaceID,PositionIndex\n1000,0,0,1,-1\n1001,1,0,1,-1\n",
            ),
            (
                "ComponentTextureFileData.csv",
                "ID,GenderIndex,ClassID,RaceID\n2000,0,0,1\n2001,1,0,1\n3000,2,0,1\n",
            ),
            (
                "ChrRaces.csv",
                "ID,MaleTextureFallbackRaceID,MaleTextureFallbackSex,FemaleTextureFallbackRaceID,FemaleTextureFallbackSex,MaleModelFallbackRaceID,MaleModelFallbackSex,FemaleModelFallbackRaceID,FemaleModelFallbackSex\n",
            ),
            (
                "HelmetGeosetData.csv",
                "HelmetGeosetVisDataID,RaceID,HideGeosetGroup,RaceBitSelection\n",
            ),
        ] {
            std::fs::write(gear.join(name), content).unwrap();
        }
        root
    }

    #[test]
    fn single_model_ignores_body_components_but_requires_model_resources() {
        let root = write_catalog("single-model");
        let catalog = OutfitData::load(&root);
        let outfit = catalog.load_owned_forever_70205().unwrap();
        for sex in [0, 1] {
            assert_eq!(
                outfit.try_resolve_runtime_model(10, 1, sex).unwrap(),
                Some((1000 + u32::from(sex), [2000 + u32::from(sex), 0, 0]))
            );
        }
        for (display, error) in [
            (11, "missing ModelFileData model resource 101"),
            (12, "missing TextureFileData material resource 201"),
        ] {
            assert_eq!(
                outfit.try_resolve_runtime_model(display, 1, 0).unwrap_err(),
                error
            );
        }
        assert_eq!(outfit.try_resolve_runtime_model(99, 1, 0).unwrap(), None);
        std::fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn columns_ignore_body_components_but_require_models_and_model_materials() {
        let root = write_catalog("columns");
        let catalog = OutfitData::load(&root);
        let outfit = catalog.load_owned_forever_70205().unwrap();
        for sex in [0, 1] {
            let baked = outfit
                .try_load_baked_display_info(10, 1, sex)
                .unwrap()
                .unwrap();
            assert_eq!(baked.model_fdids, [(100, 1000 + u32::from(sex))]);
            assert_eq!(baked.item_textures, []);
            assert_eq!(baked.geoset_overrides, []);
            assert_eq!(
                outfit.try_resolve_column_models(10, 1, sex).unwrap(),
                [(0, 1000 + u32::from(sex), [2000 + u32::from(sex), 0, 0])]
            );
        }
        assert_eq!(
            outfit.try_resolve_display_info(10, 1, 0).unwrap_err(),
            "missing TextureFileData material resource 300"
        );
        for (display, error) in [
            (11, "missing ModelFileData model resource 101"),
            (12, "missing TextureFileData material resource 201"),
        ] {
            assert_eq!(
                outfit.try_resolve_column_models(display, 1, 0).unwrap_err(),
                error
            );
            assert_eq!(
                outfit
                    .try_load_baked_display_info(display, 1, 0)
                    .unwrap_err(),
                error
            );
            assert_eq!(
                outfit.try_resolve_display_info(display, 1, 0).unwrap_err(),
                error
            );
        }
        assert_eq!(
            outfit
                .try_resolve_display_info(13, 1, 0)
                .unwrap()
                .unwrap()
                .item_textures,
            [(4, 3000)]
        );
        let regular = outfit.try_resolve_display_info(13, 1, 0).unwrap().unwrap();
        let baked = outfit
            .try_load_baked_display_info(13, 1, 0)
            .unwrap()
            .unwrap();
        assert_eq!(baked.model_fdids, regular.model_fdids);
        assert_eq!(baked.geoset_overrides, regular.geoset_overrides);
        assert_eq!(baked.item_textures, []);
        assert_eq!(
            outfit
                .try_resolve_display_info(13, 1, 0)
                .unwrap()
                .unwrap()
                .item_textures,
            [(4, 3000)]
        );
        assert!(
            outfit
                .try_load_baked_display_info(99, 1, 0)
                .unwrap()
                .is_none()
        );
        assert_eq!(outfit.try_resolve_column_models(99, 1, 0).unwrap(), []);
        std::fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn owned_retail_model_replacements_stay_isolated_from_forever() {
        let root = write_catalog("owned-replacements");
        let forever = root.join("db2/1.60.1.70205");
        let components = root.join("db2/12.1.0.69933");
        std::fs::create_dir_all(&components).unwrap();
        for name in [
            "ItemModifiedAppearance.csv",
            "ItemAppearance.csv",
            "ItemDisplayInfo.csv",
            "ItemDisplayInfoMaterialRes.csv",
            "ModelFileData.csv",
        ] {
            let source = if name == "ItemModifiedAppearance.csv" || name == "ItemAppearance.csv" {
                forever.join("items").join(name)
            } else {
                forever.join(name)
            };
            std::fs::copy(source, root.join(name)).unwrap();
        }
        for name in [
            "ComponentModelFileData.csv",
            "ComponentTextureFileData.csv",
            "ChrRaces.csv",
        ] {
            std::fs::copy(forever.join(name), components.join(name)).unwrap();
        }
        // Empty WDC5 helmet table; no external assets or shared cache needed.
        let mut helmet = vec![0; 356];
        helmet[..4].copy_from_slice(b"WDC5");
        helmet[176..180].copy_from_slice(&4u32.to_le_bytes());
        std::fs::write(root.join("db2/HelmetGeosetData.db2"), helmet).unwrap();
        std::fs::write(
            root.join("TextureFileData.csv"),
            "FileDataID,UsageType,MaterialResourcesID\n4000,0,200\n4001,0,201\n",
        )
        .unwrap();
        std::fs::write(
            root.join("ItemDisplayInfoModelMatRes.csv"),
            "ItemDisplayInfoID,ModelIndex,TextureType,MaterialResourcesID\n10,0,2,200\n10,0,3,201\n10,1,2,201\n",
        )
        .unwrap();
        let catalog = OutfitData::load(&root);
        let retail = catalog.load_owned_retail().unwrap();
        assert_eq!(
            retail.try_resolve_column_models(10, 1, 0).unwrap(),
            [(0, 1000, [4000, 0, 0])]
        );
        assert_eq!(
            retail.resolve_model_texture_fdids(10, 0, 1, 0).unwrap(),
            [(2, 4000), (3, 4001)]
        );
        assert_eq!(
            retail.resolve_model_texture_fdids(10, 1, 1, 0).unwrap(),
            [(2, 4001)]
        );
        let owned_forever = catalog.load_owned_forever_70205().unwrap();
        assert_eq!(
            owned_forever.try_resolve_column_models(10, 1, 0).unwrap(),
            [(0, 1000, [2000, 0, 0])]
        );
        for column in [0, 1] {
            assert_eq!(
                owned_forever
                    .resolve_model_texture_fdids(10, column, 1, 0)
                    .unwrap(),
                []
            );
        }
        std::fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn ailee_735014_source_components_are_not_column_dependencies() {
        let data = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../data");
        let catalog = OutfitData::load(&data);
        let outfit = catalog.load_owned_forever_70205().unwrap();
        assert_eq!(
            outfit.try_resolve_display_info(735014, 95, 1).unwrap_err(),
            "missing TextureFileData material resource 1102747"
        );
        assert_eq!(outfit.try_resolve_column_models(735014, 95, 1).unwrap(), []);
        let baked = outfit
            .try_load_baked_display_info(735014, 95, 1)
            .unwrap()
            .unwrap();
        assert_eq!(baked.model_fdids, []);
        assert_eq!(baked.item_textures, []);
        assert_eq!(baked.geoset_overrides, []);
        assert_eq!(
            outfit.try_resolve_display_info(735014, 95, 1).unwrap_err(),
            "missing TextureFileData material resource 1102747"
        );
    }
}

#[test]
fn forever_npc_gear_resolves_zephras_shoulders_and_preserves_retail() {
    let data = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../data");
    let forever = data.join("db2/1.60.1.70205");
    let mut extra = 0;
    crate::csv_util::read_numeric_rows(
        &forever.join("CreatureDisplayInfo.csv"),
        ["ID", "ExtendedDisplayInfoID"],
        |[id, value]| {
            if id == 136967 {
                extra = value;
            }
        },
    )
    .unwrap();
    assert_eq!(extra, 162977);
    let mut shoulder = 0;
    crate::csv_util::read_numeric_rows(
        &forever.join("NPCModelItemSlotDisplayInfo.csv"),
        ["NpcModelID", "ItemSlot", "ItemDisplayInfoID"],
        |[id, slot, value]| {
            if id == extra && slot == 1 {
                shoulder = value;
            }
        },
    )
    .unwrap();
    assert_eq!(shoulder, 734891);
    let outfit = OutfitData::load(&data);
    let resolved = outfit
        .try_resolve_display_info(shoulder as u32, 4, 0)
        .unwrap();
    assert!(resolved.is_some(), "Forever shoulder display missing");
    let resolved = resolved.unwrap();
    assert!(resolved.model_fdids.contains(&(84883, 7579617)));
    assert_eq!(
        outfit.display_material_texture_fdids(shoulder as u32, 4, 0),
        [7731197, 7731197]
    );
    assert_eq!(
        outfit.resolve_shoulder_runtime_model(shoulder as u32, 0, 4, 0),
        Some((7579617, [7731197, 0, 0]))
    );
    assert_eq!(
        outfit.resolve_shoulder_runtime_model(shoulder as u32, 1, 4, 0),
        Some((7579618, [7731197, 0, 0]))
    );
    selected_human_warrior_items_resolve_original_displays_and_resources();
}

#[test]
fn forever_npc_gear_reports_declared_missing_resources() {
    let source = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../data")
        .canonicalize()
        .unwrap();
    let fixture = source
        .parent()
        .unwrap()
        .join("target")
        .join(format!("forever-missing-resources-{}", std::process::id()));
    if fixture.exists() {
        std::fs::remove_dir_all(&fixture).unwrap();
    }
    std::fs::create_dir_all(fixture.join("db2/1.60.1.70205")).unwrap();
    for name in [
        "CharStartOutfit",
        "ItemModifiedAppearance",
        "ItemAppearance",
        "ItemDisplayInfo",
        "TextureFileData",
        "ItemDisplayInfoMaterialRes",
        "ModelFileData",
    ] {
        std::os::unix::fs::symlink(
            source.join(format!("{name}.csv")),
            fixture.join(format!("{name}.csv")),
        )
        .unwrap();
    }
    std::os::unix::fs::symlink(
        source.join("db2/12.1.0.69933"),
        fixture.join("db2/12.1.0.69933"),
    )
    .unwrap();
    std::os::unix::fs::symlink(
        source.join("db2/HelmetGeosetData.db2"),
        fixture.join("db2/HelmetGeosetData.db2"),
    )
    .unwrap();
    let dir = fixture.join("db2/1.60.1.70205");
    for name in [
        "ChrRaces",
        "ComponentModelFileData",
        "ComponentTextureFileData",
    ] {
        std::os::unix::fs::symlink(
            source.join("db2/1.60.1.70205").join(format!("{name}.csv")),
            dir.join(format!("{name}.csv")),
        )
        .unwrap();
    }
    let header = "ID,ModelResourcesID_0,ModelResourcesID_1,ModelMaterialResourcesID_0,ModelMaterialResourcesID_1,GeosetGroup_0,GeosetGroup_1,GeosetGroup_2,GeosetGroup_3,GeosetGroup_4,GeosetGroup_5,HelmetGeosetVis_0,HelmetGeosetVis_1\n";
    std::fs::write(dir.join("ItemDisplayInfo.csv"), format!("{header}2000000000,2000000001,0,0,0,0,0,0,0,0,0,0,0\n2000000002,0,0,2000000003,0,0,0,0,0,0,0,0,0\n2000000004,0,0,0,0,0,0,0,0,0,0,0,0\n")).unwrap();
    std::fs::write(
        dir.join("ModelFileData.csv"),
        "FileDataID,ModelResourcesID\n",
    )
    .unwrap();
    std::fs::write(
        dir.join("TextureFileData.csv"),
        "FileDataID,UsageType,MaterialResourcesID\n",
    )
    .unwrap();
    std::fs::write(
        dir.join("ItemDisplayInfoMaterialRes.csv"),
        "ItemDisplayInfoID,ComponentSection,MaterialResourcesID\n2000000004,5,2000000005\n",
    )
    .unwrap();
    let outfit = OutfitData::load(&fixture);
    for (display, resource) in [
        (2000000000, "2000000001"),
        (2000000002, "2000000003"),
        (2000000004, "2000000005"),
    ] {
        let error = outfit.try_resolve_display_info(display, 4, 0).unwrap_err();
        assert!(error.contains(resource), "{error}");
    }
    std::fs::remove_dir_all(fixture).unwrap();
}

#[test]
fn forever_npc_gear_cache_keeps_all_retail_rows_and_removes_absent_overlay() {
    use rusqlite::Connection;
    use std::collections::HashSet;
    let source = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../data")
        .canonicalize()
        .unwrap();
    let fixture = source
        .parent()
        .unwrap()
        .join("target")
        .join(format!("forever-cache-preservation-{}", std::process::id()));
    if fixture.exists() {
        std::fs::remove_dir_all(&fixture).unwrap();
    }
    std::fs::create_dir_all(fixture.join("db2")).unwrap();
    for name in [
        "CharStartOutfit",
        "ItemModifiedAppearance",
        "ItemAppearance",
        "ItemDisplayInfo",
        "TextureFileData",
        "ItemDisplayInfoMaterialRes",
        "ModelFileData",
    ] {
        std::os::unix::fs::symlink(
            source.join(format!("{name}.csv")),
            fixture.join(format!("{name}.csv")),
        )
        .unwrap();
    }
    let snapshot = |path: &Path| {
        let conn = Connection::open(path).unwrap();
        [
            "display_info",
            "display_materials",
            "material_textures",
            "model_to_fdid",
        ]
        .map(|table| {
            let mut stmt = conn.prepare(&format!("SELECT * FROM {table}")).unwrap();
            let columns = stmt.column_count();
            stmt.query_map([], |row| {
                (0..columns)
                    .map(|i| row.get::<_, i64>(i))
                    .collect::<Result<Vec<_>, _>>()
            })
            .unwrap()
            .collect::<Result<HashSet<_>, _>>()
            .unwrap()
        })
    };
    let cache = crate::outfit_catalog_db::import_outfit_links_cache(&fixture).unwrap();
    let retail = snapshot(&cache);
    let overlay = fixture.join("db2/1.60.1.70205");
    std::os::unix::fs::symlink(source.join("db2/1.60.1.70205"), &overlay).unwrap();
    crate::outfit_catalog_db::import_outfit_links_cache(&fixture).unwrap();
    let merged = snapshot(&cache);
    for (retail, merged) in retail.iter().zip(&merged) {
        assert!(retail.is_subset(merged));
        let retail_keys = retail.iter().map(|row| row[0]).collect::<HashSet<_>>();
        for row in merged.iter().filter(|row| retail_keys.contains(&row[0])) {
            assert!(
                retail.contains(row),
                "collision changed a Retail resource group"
            );
        }
    }
    assert!(merged[0].len() > retail[0].len());
    std::fs::remove_file(overlay).unwrap();
    crate::outfit_catalog_db::import_outfit_links_cache(&fixture).unwrap();
    assert_eq!(snapshot(&cache), retail);
    std::fs::remove_dir_all(fixture).unwrap();
}

fn catalog() -> OutfitData {
    let data_root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../data");
    OutfitData::load(&data_root)
}

#[test]
fn selected_human_warrior_items_resolve_original_displays_and_resources() {
    let catalog = catalog();
    for (item, display) in [
        (25, 1542),
        (38, 5729),
        (39, 6050),
        (40, 6051),
        (2362, 18730),
    ] {
        assert_eq!(catalog.resolve_item_display_id(item).unwrap(), display);
    }
    // Recruit's Pants: `leather_a_05yellow_pant_lu_m` 157713 for a man,
    // `..._lu_f` 157712 (listed first in TextureFileData) for a woman.
    let outfit = catalog.try_resolve_outfit(1, 1, 0).unwrap();
    assert!(outfit.item_textures.contains(&(5, 157713)));
    assert!(!outfit.item_textures.contains(&(5, 157712)));
    assert!(outfit.model_fdids.contains(&(16810, 148132)));
    let pants = |sex| {
        catalog
            .try_resolve_display_info(6050, 1, sex)
            .unwrap()
            .unwrap()
            .item_textures
    };
    assert_eq!(pants(0), [(5, 157713), (6, 155104)]);
    assert_eq!(pants(1), [(5, 157712), (6, 155103)]);
    assert_eq!(
        catalog.try_resolve_runtime_model(1542, 1, 0).unwrap(),
        Some((148132, [148134, 0, 0]))
    );
    assert_eq!(
        catalog.try_resolve_runtime_model(18730, 1, 0).unwrap(),
        Some((143001, [142735, 0, 0]))
    );
}

#[test]
fn equipment_everforged_shoulders_resolve_model_column_texture_types() {
    let catalog = catalog();
    let display = catalog.resolve_item_display_id(222436).unwrap();
    assert_eq!(display, 697601);
    for side in [0, 1] {
        let column = catalog.shoulder_model_column(display, side).unwrap();
        assert_eq!(column, side);
        assert_eq!(
            catalog
                .resolve_model_texture_fdids(display, column, 1, 0)
                .unwrap(),
            [(2, 5647905), (3, 5665215)]
        );
    }
}

#[test]
fn concurrent_first_imports_share_one_complete_local_catalog() {
    use std::sync::{Arc, Barrier};

    let source = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../data");
    let fixture = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../target")
        .join(format!("outfit-import-{}", std::process::id()));
    if fixture.exists() {
        std::fs::remove_dir_all(&fixture).unwrap();
    }
    std::fs::create_dir_all(&fixture).unwrap();
    for name in [
        "CharStartOutfit.csv",
        "ItemModifiedAppearance.csv",
        "ItemAppearance.csv",
        "ItemDisplayInfo.csv",
        "TextureFileData.csv",
        "ItemDisplayInfoMaterialRes.csv",
        "ModelFileData.csv",
    ] {
        std::os::unix::fs::symlink(
            source.join(name).canonicalize().unwrap(),
            fixture.join(name),
        )
        .unwrap();
    }
    let barrier = Arc::new(Barrier::new(2));
    std::thread::scope(|scope| {
        let workers = (0..2)
            .map(|_| {
                let barrier = Arc::clone(&barrier);
                let fixture = &fixture;
                scope.spawn(move || {
                    barrier.wait();
                    crate::outfit_catalog_db::import_outfit_links_cache(fixture).unwrap();
                    let ids = crate::outfit_catalog_db::resolve_cached_outfit_display_ids(
                        fixture, 1, 1, 0,
                    )
                    .unwrap();
                    assert!(ids.contains(&1542));
                })
            })
            .collect::<Vec<_>>();
        for worker in workers {
            worker.join().unwrap();
        }
    });
    let cache = fixture.join("cache/outfit_links-v3.sqlite");
    let before = std::fs::metadata(&cache).unwrap().modified().unwrap();
    let alias = fixture.join("..").join(fixture.file_name().unwrap());
    crate::outfit_catalog_db::import_outfit_links_cache(&alias).unwrap();
    let after = std::fs::metadata(&cache).unwrap().modified().unwrap();
    assert_eq!(
        before, after,
        "equivalent data roots must reuse the same cache"
    );
    std::fs::remove_dir_all(&fixture).unwrap();
}

#[test]
fn missing_data_root_is_an_error_not_an_empty_outfit() {
    let catalog = OutfitData::load(Path::new("/nonexistent/outfit-data-root"));
    assert!(catalog.try_resolve_outfit(1, 1, 0).is_err());
}

/// Item models resolve from the DB2 tables alone (`ModelFileData` maps a model resource to
/// its FDIDs and local CASC extracts by FDID): the 143 MB community listfile is not read.
#[test]
fn runtime_models_resolve_without_the_community_listfile() {
    let source = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../data")
        .canonicalize()
        .unwrap();
    let fixture = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../target")
        .join(format!("outfit-no-listfile-{}", std::process::id()));
    if fixture.exists() {
        std::fs::remove_dir_all(&fixture).unwrap();
    }
    std::fs::create_dir_all(&fixture).unwrap();
    for entry in std::fs::read_dir(&source).unwrap() {
        let name = entry.unwrap().file_name();
        if name != "community-listfile.csv" {
            std::os::unix::fs::symlink(source.join(&name), fixture.join(&name)).unwrap();
        }
    }
    let catalog = OutfitData::load(&fixture);
    let sword = catalog.try_resolve_runtime_model(1542, 1, 0);
    let mace = catalog.try_resolve_runtime_model(18730, 1, 0);
    std::fs::remove_dir_all(&fixture).unwrap();
    assert_eq!(sword.unwrap(), Some((148132, [148134, 0, 0])));
    assert_eq!(mace.unwrap(), Some((143001, [142735, 0, 0])));
}
