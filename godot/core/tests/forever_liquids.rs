use game_engine_core::liquid_data::{LiquidCatalog, MapLiquidCatalog};
use std::{fs, path::PathBuf};

fn data_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../data")
}

#[test]
fn forever_liquids_resolve_observed_objects_and_preserve_every_retail_material() {
    let data = data_root();
    let catalogs = MapLiquidCatalog::read(&data).unwrap();
    let retail = LiquidCatalog::read(&data.join("db2/12.1.0.69933")).unwrap();
    let forever = LiquidCatalog::read(&data.join("db2/1.60.1.70205")).unwrap();
    for (kind, object) in [(1251, 18420), (1279, 18563), (1279, 18615), (1279, 21229)] {
        let actual = catalogs.liquid_material(2991, kind, object).unwrap();
        assert_eq!(actual, forever.liquid_material(kind, object).unwrap());
        assert_eq!(actual.liquid_type, u32::from(kind));
        assert!(!actual.texture_slots[2].is_empty());
        assert!(!actual.texture_slots[3].is_empty());
        assert!(catalogs.liquid_material(0, kind, object).is_err());
    }
    let csv = fs::read_to_string(data.join("db2/12.1.0.69933/LiquidType.csv")).unwrap();
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
    assert!(
        catalogs
            .liquid_material(2991, 1251, 65535)
            .unwrap_err()
            .contains("Forever LiquidObject 65535")
    );
    assert!(
        catalogs
            .liquid_material(2991, 65535, 0)
            .unwrap_err()
            .contains("LiquidType 65535")
    );
}

#[test]
fn forever_liquids_are_isolated_from_colliding_retail_ids_and_missing_tables() {
    let data = data_root();
    let scratch = std::env::temp_dir().join(format!("forever-liquids-{}", std::process::id()));
    let retail_dir = scratch.join("db2/12.1.0.69933");
    let forever_dir = scratch.join("db2/1.60.1.70205");
    fs::create_dir_all(&retail_dir).unwrap();
    fs::create_dir_all(&forever_dir).unwrap();
    for table in [
        "LiquidType",
        "LiquidMaterial",
        "LiquidObject",
        "LiquidTypeXTexture",
    ] {
        let source = data.join(format!("db2/12.1.0.69933/{table}.csv"));
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
    fs::write(forever_dir.join("LiquidObject.csv"), objects).unwrap();
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
