use std::path::Path;

use crate::outfit_data::OutfitData;

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
        [7731197]
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
