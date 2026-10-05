use game_engine_core::outfit_data::OutfitData;
use game_engine_godot::equipment_appearance_data::*;
use shared::components::{EquipmentAppearance, EquipmentVisualSlot, EquippedAppearanceEntry};
use std::path::{Path, PathBuf};

fn data_dir() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../../data")
}

fn entry(slot: EquipmentVisualSlot, item_id: u32) -> EquippedAppearanceEntry {
    EquippedAppearanceEntry {
        definition_source: Some(shared::item_data::ItemDefinitionSource::Retail),
        slot,
        item_id: Some(item_id),
        display_info_id: None,
        inventory_type: 0,
        hidden: false,
    }
}

fn source_fixture(name: &str) -> PathBuf {
    let root = data_dir()
        .parent()
        .unwrap()
        .join("target")
        .join(format!("equipment-source-{}-{name}", std::process::id()));
    if root.exists() {
        std::fs::remove_dir_all(&root).unwrap();
    }
    let forever = root.join("db2/1.60.1.70205");
    let retail = root.join("db2/12.1.0.69933");
    std::fs::create_dir_all(forever.join("items")).unwrap();
    std::fs::create_dir_all(&retail).unwrap();
    let write = |dir: &Path, name: &str, text: &str| {
        std::fs::write(dir.join(format!("{name}.csv")), text).unwrap();
    };
    let display_header = "ID,ModelResourcesID_0,ModelResourcesID_1,ModelMaterialResourcesID_0,ModelMaterialResourcesID_1,GeosetGroup_0,GeosetGroup_1,GeosetGroup_2,GeosetGroup_3,GeosetGroup_4,GeosetGroup_5,HelmetGeosetVis_0,HelmetGeosetVis_1\n";
    write(
        &root,
        "CharStartOutfit",
        &format!(
            "RaceID,ClassID,SexID,{}\n",
            (0..12)
                .map(|i| format!("ItemID_{i}"))
                .collect::<Vec<_>>()
                .join(",")
        ),
    );
    write(
        &root,
        "ItemModifiedAppearance",
        "ItemID,ItemAppearanceID\n25,10\n",
    );
    write(&root, "ItemAppearance", "ID,ItemDisplayInfoID\n10,1542\n");
    write(
        &root,
        "ItemDisplayInfo",
        &format!("{display_header}1542,7,0,8,0,1,0,0,0,0,0,0,0\n"),
    );
    write(
        &root,
        "ModelFileData",
        "FileDataID,ModelResourcesID\n111,7\n",
    );
    write(
        &root,
        "TextureFileData",
        "FileDataID,UsageType,MaterialResourcesID\n112,0,8\n",
    );
    write(
        &root,
        "ItemDisplayInfoMaterialRes",
        "ItemDisplayInfoID,ComponentSection,MaterialResourcesID\n1542,5,8\n",
    );
    write(
        &forever.join("items"),
        "ItemModifiedAppearance",
        "ItemID,ItemAppearanceID,OrderIndex\n25,10,0\n",
    );
    write(
        &forever.join("items"),
        "ItemAppearance",
        "ID,ItemDisplayInfoID,DefaultIconFileDataID\n10,99,1\n",
    );
    write(
        &forever,
        "ItemDisplayInfo",
        &format!("{display_header}99,7,0,8,0,4,0,0,0,0,0,0,0\n1542,7,0,8,0,4,0,0,0,0,0,0,0\n"),
    );
    write(
        &forever,
        "ModelFileData",
        "FileDataID,ModelResourcesID\n211,7\n",
    );
    write(
        &forever,
        "TextureFileData",
        "FileDataID,UsageType,MaterialResourcesID\n212,0,8\n",
    );
    write(
        &forever,
        "ItemDisplayInfoMaterialRes",
        "ItemDisplayInfoID,ComponentSection,MaterialResourcesID\n99,5,8\n1542,5,8\n",
    );
    write(
        &forever,
        "HelmetGeosetData",
        "HelmetGeosetVisDataID,RaceID,HideGeosetGroup,RaceBitSelection\n",
    );
    for dir in [&retail, &forever] {
        write(
            dir,
            "ComponentModelFileData",
            "ID,GenderIndex,ClassID,RaceID,PositionIndex\n",
        );
        write(
            dir,
            "ComponentTextureFileData",
            "ID,GenderIndex,ClassID,RaceID\n",
        );
        write(
            dir,
            "ChrRaces",
            "ID,MaleTextureFallbackRaceID,MaleTextureFallbackSex,FemaleTextureFallbackRaceID,FemaleTextureFallbackSex,MaleModelFallbackRaceID,MaleModelFallbackSex,FemaleModelFallbackRaceID,FemaleModelFallbackSex\n",
        );
    }
    write(
        &retail,
        "ComponentModelFileData",
        "ID,GenderIndex,ClassID,RaceID,PositionIndex\n211,1,0,1,-1\n",
    );
    write(
        &retail,
        "ComponentTextureFileData",
        "ID,GenderIndex,ClassID,RaceID\n212,1,0,1\n",
    );
    write(
        &forever,
        "ComponentModelFileData",
        "ID,GenderIndex,ClassID,RaceID,PositionIndex\n211,0,0,1,-1\n",
    );
    write(
        &forever,
        "ComponentTextureFileData",
        "ID,GenderIndex,ClassID,RaceID\n212,0,0,1\n",
    );
    std::fs::copy(
        data_dir().join("db2/HelmetGeosetData.db2"),
        root.join("db2/HelmetGeosetData.db2"),
    )
    .unwrap();
    root
}

fn forever_entry(slot: EquipmentVisualSlot) -> EquippedAppearanceEntry {
    EquippedAppearanceEntry {
        definition_source: Some(shared::item_data::ItemDefinitionSource::Forever70205),
        ..entry(slot, 25)
    }
}

#[test]
fn owned_sources_resolve_distinct_displays_and_colliding_resource_groups() {
    let root = source_fixture("collision");
    let catalog = OutfitData::load(&root);
    let resolve = |entry| {
        resolve_equipment_appearance(
            &EquipmentAppearance {
                entries: vec![entry],
            },
            &catalog,
            1,
            0,
        )
        .unwrap()
    };
    let retail = resolve(entry(EquipmentVisualSlot::Legs, 25));
    let forever = resolve(forever_entry(EquipmentVisualSlot::Legs));
    assert_eq!(retail.runtime_models[0].fdid, 111);
    assert_eq!(retail.runtime_models[0].skin_fdids, [112, 0, 0]);
    assert_eq!(retail.outfit.item_textures, [(5, 112)]);
    assert!(retail.outfit.geoset_overrides.contains(&(11, 2)));
    assert_eq!(forever.runtime_models[0].fdid, 211);
    assert_eq!(forever.runtime_models[0].skin_fdids, [212, 0, 0]);
    assert_eq!(forever.outfit.item_textures, [(5, 212)]);
    assert!(forever.outfit.geoset_overrides.contains(&(11, 5)));
    let explicit = resolve(EquippedAppearanceEntry {
        display_info_id: Some(1542),
        ..forever_entry(EquipmentVisualSlot::Legs)
    });
    assert_eq!(explicit.runtime_models, forever.runtime_models);
    assert_eq!(explicit.outfit.item_textures, forever.outfit.item_textures);
    std::fs::remove_dir_all(root).unwrap();
}

#[test]
fn owned_mixed_sources_keep_body_geosets_in_each_display_namespace() {
    let root = source_fixture("mixed-body");
    let entries = vec![
        EquippedAppearanceEntry {
            display_info_id: Some(1542),
            ..forever_entry(EquipmentVisualSlot::Shirt)
        },
        entry(EquipmentVisualSlot::Legs, 25),
    ];
    let result = resolve_equipment_appearance(
        &EquipmentAppearance { entries },
        &OutfitData::load(&root),
        1,
        0,
    )
    .unwrap();
    assert!(result.outfit.geoset_overrides.contains(&(8, 5)));
    assert!(result.outfit.geoset_overrides.contains(&(11, 2)));
    std::fs::remove_dir_all(root).unwrap();
}

#[test]
fn owned_forever_missing_source_never_substitutes_retail() {
    let root = source_fixture("missing-source");
    std::fs::remove_dir_all(root.join("db2/1.60.1.70205")).unwrap();
    let result = resolve_equipment_appearance(
        &EquipmentAppearance {
            entries: vec![forever_entry(EquipmentVisualSlot::MainHand)],
        },
        &OutfitData::load(&root),
        1,
        0,
    );
    let error = result.unwrap_err();
    assert!(error.contains("1.60.1.70205"), "{error}");
    std::fs::remove_dir_all(root).unwrap();
}

#[test]
fn owned_item_requires_paired_definition_source_but_display_only_remains_legacy() {
    let root = source_fixture("pairing");
    let catalog = OutfitData::load(&root);
    let unpaired = EquippedAppearanceEntry {
        definition_source: None,
        ..entry(EquipmentVisualSlot::Legs, 25)
    };
    let error = resolve_equipment_appearance(
        &EquipmentAppearance {
            entries: vec![unpaired],
        },
        &catalog,
        1,
        0,
    )
    .unwrap_err();
    assert!(error.contains("definition source"), "{error}");
    let display_only = EquippedAppearanceEntry {
        definition_source: None,
        item_id: None,
        display_info_id: Some(1542),
        ..entry(EquipmentVisualSlot::Legs, 25)
    };
    let result = resolve_equipment_appearance(
        &EquipmentAppearance {
            entries: vec![display_only],
        },
        &catalog,
        1,
        0,
    )
    .unwrap();
    assert_eq!(result.runtime_models[0].fdid, 111);
    assert_eq!(result.outfit.item_textures, [(5, 112)]);
    std::fs::remove_dir_all(root).unwrap();
}

#[test]
fn owned_retail_missing_material_never_borrows_forever_group() {
    let root = source_fixture("missing-retail-material");
    std::fs::write(
        root.join("TextureFileData.csv"),
        "FileDataID,UsageType,MaterialResourcesID\n",
    )
    .unwrap();
    let result = resolve_equipment_appearance(
        &EquipmentAppearance {
            entries: vec![entry(EquipmentVisualSlot::Legs, 25)],
        },
        &OutfitData::load(&root),
        1,
        0,
    );
    let error = result.unwrap_err();
    assert!(error.contains("material resource 8"), "{error}");
    std::fs::remove_dir_all(root).unwrap();
}

#[test]
fn owned_forever_missing_item_never_substitutes_retail() {
    let root = source_fixture("missing-item");
    std::fs::write(
        root.join("db2/1.60.1.70205/items/ItemModifiedAppearance.csv"),
        "ItemID,ItemAppearanceID,OrderIndex\n",
    )
    .unwrap();
    let result = resolve_equipment_appearance(
        &EquipmentAppearance {
            entries: vec![forever_entry(EquipmentVisualSlot::MainHand)],
        },
        &OutfitData::load(&root),
        1,
        0,
    );
    assert!(
        result.is_err(),
        "missing Forever item rendered Retail: {result:?}"
    );
    std::fs::remove_dir_all(root).unwrap();
}

#[test]
fn owned_forever_missing_material_never_substitutes_retail_group() {
    let root = source_fixture("missing-material");
    std::fs::write(
        root.join("db2/1.60.1.70205/TextureFileData.csv"),
        "FileDataID,UsageType,MaterialResourcesID\n",
    )
    .unwrap();
    let result = resolve_equipment_appearance(
        &EquipmentAppearance {
            entries: vec![forever_entry(EquipmentVisualSlot::Legs)],
        },
        &OutfitData::load(&root),
        1,
        0,
    );
    let error = result.unwrap_err();
    assert!(error.contains("material resource 8"), "{error}");
    std::fs::remove_dir_all(root).unwrap();
}
