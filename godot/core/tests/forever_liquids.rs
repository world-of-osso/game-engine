use game_engine_core::liquid_data::{LiquidCatalog, LiquidFamily, LiquidShader, MapLiquidCatalog};
use std::{fs, os::unix::fs::PermissionsExt, path::PathBuf};

fn data_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../data")
}

#[test]
fn forever_liquids_resolve_observed_objects_and_preserve_every_retail_material() {
    let data = data_root();
    let catalogs = MapLiquidCatalog::read(&data).unwrap();
    let retail = LiquidCatalog::read(&data.join("db2/active")).unwrap();
    let forever = LiquidCatalog::read(&data.join("db2/1.60.1.70205")).unwrap();
    for (kind, object) in [(1251, 18420), (1279, 18563), (1279, 18615), (1279, 21229)] {
        let actual = catalogs.liquid_material(2991, kind, object).unwrap();
        assert_eq!(actual, forever.liquid_material(kind, object).unwrap());
        assert_eq!(actual.liquid_type, u32::from(kind));
        assert!(!actual.texture_slots[2].is_empty());
        assert!(!actual.texture_slots[3].is_empty());
        // Unknown Retail ids retry Retail type8 on the WMO path only, never the
        // colliding Forever row; the terrain path has no retry.
        // The retry lands on Retail's shared type 8; its vertex format comes from
        // LiquidMaterial, because the object key is above 42.
        let retried = catalogs.wmo_material(0, kind, object).unwrap();
        assert_eq!(retried.liquid_type, 8);
        assert_eq!(retried.material_id, 2);
        assert_eq!(retried.shader, LiquidShader::Magma);
        assert_eq!(
            catalogs.liquid_material(0, kind, object).unwrap_err(),
            format!("LiquidType {kind} has no DB2 row")
        );
    }
    let csv = fs::read_to_string(data.join("db2/active/LiquidType.csv")).unwrap();
    for row in game_engine_core::csv_util::parse_csv_records(&csv)
        .into_iter()
        .skip(1)
    {
        let id: u16 = row[0].parse().unwrap();
        let expected = retail.liquid_material(id, 0);
        assert_eq!(
            catalogs.liquid_material(0, id, 0),
            expected,
            "Retail LiquidType {id}"
        );
    }
    // An authored object id with no row resolves to the layer's own type and no flow, the
    // same rule the ocean object 42 needs; only the type lookup can still fail.
    let authored = catalogs.liquid_material(2991, 1251, 65535).unwrap();
    assert_eq!(authored.liquid_type, 1251);
    assert_eq!(authored.flow_speed, 0.0);
    assert_eq!(
        catalogs.wmo_material(2991, 65535, 0).unwrap(),
        forever.liquid_material(8, 0).unwrap()
    );
    assert_eq!(
        catalogs.liquid_material(2991, 65535, 0).unwrap_err(),
        "Forever map 2991: LiquidType 65535 has no DB2 row"
    );
}

#[test]
fn pbr_material_renders_its_own_six_textures_and_38_floats() {
    let data = data_root();
    let mut maps = MapLiquidCatalog::read(&data).unwrap();
    maps.set_pbr_water(true);
    let forever = LiquidCatalog::read(&data.join("db2/1.60.1.70205")).unwrap();
    for (kind, object) in [(1251, 18420), (1279, 21229)] {
        let source = maps.liquid_material(2991, kind, object).unwrap();
        assert_eq!(source.floats.len(), 38);
        assert!((source.floats[23] + 0.146).abs() < 0.000_001);
        let rendered = maps
            .render_material(
                game_engine_core::liquid_data::LiquidFamily::Terrain,
                2991,
                kind,
                object,
            )
            .unwrap();
        assert_eq!(rendered, source);
        assert_eq!(rendered.liquid_type, u32::from(kind));
        assert_eq!(rendered.material_id, 130);
        assert_eq!(
            rendered.floats,
            forever.liquid_material(kind, object).unwrap().floats
        );
        let slots: Vec<u32> = rendered.texture_slots.iter().flatten().copied().collect();
        assert_eq!(slots.len(), 6);
        assert_eq!(slots[0], 7475833);
        assert_eq!(slots[1], 7475835);
        assert_eq!(slots[2], 7475834);
        assert_eq!(slots[3], 7475836);
        assert_eq!(slots[4], 7660147);
        assert_eq!(slots[5], 7480794);
        if kind == 1251 {
            assert_eq!(
                rendered.pbr_layer_transforms(),
                [[0.4, 0.9, 7.85], [50.0, 1.0, 0.951], [60.0, 0.774, 26.0]]
            );
            assert_eq!(rendered.pbr_foam(), ([2.0, 0.015, 1.2, 12.0], 12.0));
            let batch = rendered.pbr_material_batch();
            assert!((batch[0] - (1.0 - 179.0 / 255.0)).abs() < 0.000_001);
            assert!((batch[1] - (1.0 - 219.0 / 255.0)).abs() < 0.000_001);
            assert!((batch[2] - (1.0 - 204.0 / 255.0)).abs() < 0.000_001);
            assert_eq!(batch[3], 0.4);
            assert_eq!(&batch[4..8], &[50.0, 1.0, 0.951, 60.0]);
            assert!((batch[8] - 251.0 / 255.0).abs() < 0.000_001);
            assert!((batch[9] - 223.0 / 255.0).abs() < 0.000_001);
            assert!((batch[10] - 118.0 / 255.0).abs() < 0.000_001);
            assert_eq!(batch[11], 7.85);
            assert!((batch[12] - 195.0 / 255.0).abs() < 0.000_001);
            assert_eq!(batch[15], 0.9);
            // Float[7..25] ascending with EF2's two overrides.
            assert_eq!(batch[16], 0.774);
            assert_eq!(batch[24], 1.2);
            assert_eq!(batch[25], 0.02);
        }
    }
    let retail = maps.liquid_material(0, 947, 42).unwrap();
    assert_eq!(
        maps.render_material(
            game_engine_core::liquid_data::LiquidFamily::Terrain,
            0,
            947,
            42
        )
        .unwrap(),
        retail
    );
}

#[test]
fn default_material130_keeps_the_liquidtype5_fallback() {
    let data = data_root();
    let maps = MapLiquidCatalog::read(&data).unwrap();
    assert!(!maps.pbr_water());
    let forever = LiquidCatalog::read(&data.join("db2/1.60.1.70205")).unwrap();
    let legacy = forever.liquid_material(5, 0).unwrap();
    for (kind, object) in [(1251, 18420), (1279, 21229)] {
        let source = maps.liquid_material(2991, kind, object).unwrap();
        let rendered = maps
            .render_material(
                game_engine_core::liquid_data::LiquidFamily::Terrain,
                2991,
                kind,
                object,
            )
            .unwrap();
        assert_ne!(rendered, source);
        assert_eq!(rendered.liquid_type, u32::from(kind));
        assert_eq!(rendered.material_id, 130);
        assert_eq!(
            rendered.shader,
            game_engine_core::liquid_data::LiquidShader::Water
        );
        assert_eq!(rendered.lvf, source.lvf);
        assert_eq!(rendered.floats, legacy.floats);
        assert_eq!(rendered.depth_coefficients, legacy.depth_coefficients);
        assert_eq!(rendered.depth_curve, legacy.depth_curve);
        assert_eq!(rendered.texture_slots, legacy.texture_slots);
        assert_eq!(rendered.texture_slots[2], [463849]);
        assert_eq!(rendered.texture_slots[3], [317230]);
    }
}

#[test]
fn forever_liquids_are_isolated_from_colliding_retail_ids_and_missing_tables() {
    let data = data_root();
    let scratch = std::env::temp_dir().join(format!("forever-liquids-{}", std::process::id()));
    let retail_dir = scratch.join("db2/active");
    let forever_dir = scratch.join("db2/1.60.1.70205");
    fs::create_dir_all(&retail_dir).unwrap();
    fs::create_dir_all(&forever_dir).unwrap();
    for table in [
        "LiquidType",
        "LiquidMaterial",
        "LiquidObject",
        "LiquidTypeXTexture",
    ] {
        let source = data.join(format!("db2/active/{table}.csv"));
        fs::copy(&source, retail_dir.join(format!("{table}.csv"))).unwrap();
        fs::copy(&source, forever_dir.join(format!("{table}.csv"))).unwrap();
    }
    let header = "ID,Directory,MapName_lang,WdtFileDataID\n";
    fs::write(
        retail_dir.join("Map.csv"),
        format!("{header}0,Azeroth,Retail,1\n"),
    )
    .unwrap();
    fs::write(
        forever_dir.join("Map.csv"),
        format!(
            "{header}0,Other,Collision,2\n2991,2991,Zephras,3\n2992,Azeroth,DirectoryCollision,4\n"
        ),
    )
    .unwrap();
    let objects = "ID,FlowDirection,FlowSpeed,LiquidTypeID\n427,1,2,5\n";
    let object_file = forever_dir.join("LiquidObject.csv");
    // fs::copy preserves the read-only source mode; only the scratch copy is edited.
    let mut permissions = fs::metadata(&object_file).unwrap().permissions();
    permissions.set_mode(permissions.mode() | 0o200);
    fs::set_permissions(&object_file, permissions).unwrap();
    fs::write(object_file, objects).unwrap();
    let catalog = MapLiquidCatalog::read(&scratch).unwrap();
    assert_eq!(catalog.liquid_material(0, 5, 427).unwrap().flow_speed, 0.0);
    assert_eq!(
        catalog.liquid_material(2992, 5, 427).unwrap().flow_speed,
        0.0
    );
    assert_eq!(
        catalog.liquid_material(2991, 5, 427).unwrap().flow_speed,
        2.0
    );
    fs::remove_file(forever_dir.join("LiquidType.csv")).unwrap();
    let catalog = MapLiquidCatalog::read(&scratch).unwrap();
    assert_eq!(catalog.liquid_material(0, 5, 427).unwrap().flow_speed, 0.0);
    assert!(
        catalog
            .liquid_material(2991, 5, 427)
            .unwrap_err()
            .contains("LiquidType.csv")
    );
    fs::remove_dir_all(scratch).unwrap();
}

/// RF3: the `_depth` art is slot 4 canonically. Two Forever rows store the rim and depth
/// paths reversed (1250 ocean and 1296); the catalog finds the swap by comparing each row's
/// slot-3/slot-4 pair with the modal pair, because our Forever export carries no path table.
#[test]
fn material130_depth_art_slot_follows_the_row_order() {
    let forever = LiquidCatalog::read(&data_root().join("db2/1.60.1.70205")).unwrap();
    let canonical = forever.liquid_material(1240, 0).unwrap();
    assert_eq!(canonical.pbr_depth_slot, 4);
    assert_eq!(canonical.pbr_mask_slots(), [0, 1, 2, 3, 4, 5]);
    assert_eq!(canonical.texture_slots[3], [7475836]);
    assert_eq!(canonical.texture_slots[4], [7660147]);

    // PBRWater - Generic - Ocean stores the depth art in slot 3 and the rim in slot 4.
    let ocean = forever.liquid_material(1250, 0).unwrap();
    assert_eq!(ocean.pbr_depth_slot, 3);
    assert_eq!(ocean.pbr_mask_slots(), [0, 1, 2, 4, 3, 5]);
    assert_eq!(ocean.texture_slots[3], [7660147]);
    assert_eq!(ocean.texture_slots[4], [7475836]);
    // Darkspear Islands - Ocean is the second swapped row.
    assert_eq!(forever.liquid_material(1296, 0).unwrap().pbr_depth_slot, 3);

    // A non-Material130 row keeps the canonical slot, which it never binds.
    assert_eq!(forever.liquid_material(5, 0).unwrap().pbr_depth_slot, 4);
}

/// The ocean object 42 has no `LiquidObject` row, and dropping it removed entire seas (73
/// layers on map 2959 alone). It resolves to the layer's own LiquidType 2 with no flow and the
/// ocean vertex format, which is what the Retail path and WebWowViewerCpp do.
#[test]
fn forever_ocean_object_42_takes_the_layer_liquid_type() {
    let maps = MapLiquidCatalog::read(&data_root()).unwrap();
    let material = maps.liquid_material(2959, 2, 42).unwrap();
    assert_eq!(material.liquid_type, 2);
    assert_eq!(material.lvf, 2);
    assert_eq!(material.flow_speed, 0.0);
    assert_eq!(material.flow_direction, 0.0);
    assert_eq!(material.shader, LiquidShader::Water);
    assert!(maps.liquid_material(2959, 2, 42).is_ok());
}

/// MH2O's `LiquidObject` is a hybrid: below 42 the value is the vertex format itself, exactly
/// 42 is a sentinel that is never looked up, and above 42 a miss yields a zero-flow record.
#[test]
fn liquid_object_is_a_hybrid_of_format_key_and_sentinel() {
    let data = data_root();
    let forever = LiquidCatalog::read(&data.join("db2/1.60.1.70205")).unwrap();
    let retail = LiquidCatalog::read(&data.join("db2/active")).unwrap();
    // Ocean layers store the sentinel with LiquidType 2 and take the hard-coded format 2.
    let ocean = forever.surface(LiquidFamily::Terrain, 2, 42).unwrap();
    assert_eq!(ocean.liquid_type, 2);
    assert_eq!(ocean.lvf, 2);
    // Below 42 the value is the format, with no table lookup.
    let literal = retail.surface(LiquidFamily::Terrain, 8, 1).unwrap();
    assert_eq!(literal.lvf, 1);
    assert_eq!(literal.liquid_type, 8);
    // Above 42 a missing key keeps the layer's type and zeroes the flow.
    let missing = forever.surface(LiquidFamily::Terrain, 1251, 65535).unwrap();
    assert_eq!(missing.liquid_type, 1251);
    assert_eq!(missing.flow_speed, 0.0);
    // A real key still supplies its own type and flow.
    let keyed = forever.surface(LiquidFamily::Terrain, 0, 18420).unwrap();
    assert_eq!(keyed.liquid_type, 1251);
    assert_eq!(keyed.flow_speed, 0.0);
}
