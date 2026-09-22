use super::*;
use std::sync::atomic::{AtomicU64, Ordering};

struct CatalogFixture {
    root: PathBuf,
}

impl CatalogFixture {
    fn new() -> Self {
        static SEQUENCE: AtomicU64 = AtomicU64::new(0);
        let root = std::env::temp_dir().join(format!(
            "customization-catalog-{}-{}",
            std::process::id(),
            SEQUENCE.fetch_add(1, Ordering::Relaxed)
        ));
        std::fs::create_dir(&root).unwrap();
        let fixture = Self { root };
        for (name, contents) in [
            (
                "ChrModel",
                "ID,CharComponentTextureLayoutID,CustomizeScale,CameraDistanceOffset\n1,1,1.1,-0.34\n20,1,1,0\n",
            ),
            (
                "ChrCustomizationOption",
                "Name_lang,ID,ChrModelID,ChrCustomizationCategoryID,OrderIndex,OptionType,Requirement\nEyebrows,890,1,3,10,0,0\nEars,8789,1,23,14,0,0\nJewelry Color,776,20,5,26,1,12\n",
            ),
            (
                "ChrCustomizationCategory",
                "ID,CategoryName_lang,OrderIndex,CustomizeIcon,CustomizeIconSelected\n3,Accessories,3,11985,11984\n23,Ears,7,11991,11990\n5,Markings,5,11987,11986\n",
            ),
            (
                "ChrCustomizationChoice",
                "Name_lang,ID,ChrCustomizationOptionID,ChrCustomizationReqID,OrderIndex,ChrCustomizationVisReqID,SwatchColor_0,SwatchColor_1\nCurved,90001,890,0,2,0,0,0\nStraight,90002,890,0,1,0,0,0\nGold,90003,776,19,0,23,-26091,-16777216\n",
            ),
            (
                "ChrCustomizationElement",
                "ChrCustomizationChoiceID,RelatedChrCustomizationChoiceID,ChrCustomizationGeosetID,ChrCustomizationMaterialID,ChrCustomizationSkinnedModelID,ChrCustomizationBoneSetID,ChrCustomizationCondModelID,ChrCustomizationDisplayInfoID,ChrCustItemGeoModifyID,ChrCustomizationVoiceID,AnimKitID,ParticleColorID,ChrCustGeoComponentLinkID\n90001,0,1,0,0,0,0,0,0,0,0,0,0\n90002,0,0,0,0,0,0,0,0,0,0,0,0\n90003,0,0,1,7,0,0,0,0,0,0,0,0\n",
            ),
            (
                "ChrCustomizationMaterial",
                "ID,ChrModelTextureTargetID,MaterialResourcesID\n1,6,101\n",
            ),
            ("ChrCustomizationGeoset", "ID,GeosetType,GeosetID\n1,32,2\n"),
            (
                "CharHairGeosets",
                "RaceID,SexID,GeosetType,GeosetID,Showscalp\n1,0,0,7,1\n",
            ),
            (
                "TextureFileData",
                "FileDataID,MaterialResourcesID\n1020001,101\n",
            ),
        ] {
            std::fs::write(fixture.root.join(format!("{name}.csv")), contents).unwrap();
        }
        fixture
    }

    fn cache_path(&self) -> PathBuf {
        self.root.join("cache.sqlite")
    }
}

impl Drop for CatalogFixture {
    fn drop(&mut self) {
        if let Err(error) = std::fs::remove_dir_all(&self.root) {
            eprintln!("remove test catalog {}: {error}", self.root.display());
        }
    }
}

#[test]
fn catalog_cache_roundtrip_retains_original_metadata_and_effect_support() {
    let fixture = CatalogFixture::new();
    let cache = import_customization_cache_at(&fixture.root, &fixture.cache_path()).unwrap();
    let raw = load_customization_raw_data_at(&fixture.root, &cache).unwrap();
    let eyebrows = raw.options.iter().find(|option| option.id == 890).unwrap();
    assert_eq!(
        (
            eyebrows.name.as_str(),
            eyebrows.category_id,
            eyebrows.order_index
        ),
        ("Eyebrows", 3, 10)
    );
    let jewelry = raw.options.iter().find(|option| option.id == 776).unwrap();
    assert_eq!(
        (
            jewelry.name.as_str(),
            jewelry.category_id,
            jewelry.order_index,
            jewelry.ui_type,
            jewelry.requirement_id
        ),
        ("Jewelry Color", 5, 26, 1, 12)
    );
    let jewelry_choice = raw
        .choices
        .iter()
        .find(|choice| choice.id == 90003)
        .unwrap();
    assert_eq!(jewelry_choice.swatch_colors, [-26_091, -16_777_216]);
    assert_eq!(jewelry_choice.visibility_requirement_id, 23);
    let category = &raw.categories[&3];
    assert_eq!(
        (
            category.name.as_str(),
            category.order_index,
            category.icon,
            category.selected_icon
        ),
        ("Accessories", 3, 11985, 11984)
    );
    assert_eq!(
        raw.choices
            .iter()
            .find(|choice| choice.id == 90003)
            .unwrap()
            .requirement_id,
        19
    );
    assert!(
        !raw.elements
            .iter()
            .find(|element| element.choice_id == 90002)
            .unwrap()
            .has_unsupported_effects,
        "an authored empty choice is not an unsupported effect"
    );
    assert!(
        raw.elements
            .iter()
            .find(|element| element.choice_id == 90003)
            .unwrap()
            .has_unsupported_effects
    );
    let before = std::fs::metadata(&cache).unwrap().modified().unwrap();
    assert_eq!(
        import_customization_cache_at(&fixture.root, &cache).unwrap(),
        cache
    );
    assert_eq!(
        std::fs::metadata(&cache).unwrap().modified().unwrap(),
        before
    );
}

#[test]
fn catalog_cache_rebuilds_old_schema_without_deleting_the_cache_by_hand() {
    let fixture = CatalogFixture::new();
    let cache = import_customization_cache_at(&fixture.root, &fixture.cache_path()).unwrap();
    for version in [0, 1] {
        let conn = Connection::open(&cache).unwrap();
        conn.execute("UPDATE options SET name = 'stale' WHERE id = 890", [])
            .unwrap();
        conn.pragma_update(None, "user_version", version).unwrap();
        drop(conn);
        import_customization_cache_at(&fixture.root, &cache).unwrap();
        let raw = load_customization_raw_data_at(&fixture.root, &cache).unwrap();
        assert_eq!(
            raw.options
                .iter()
                .find(|option| option.id == 890)
                .unwrap()
                .name,
            "Eyebrows"
        );
        assert_eq!(raw.categories[&3].name, "Accessories");
    }
}

#[test]
fn catalog_loader_rebuilds_previous_catalog_schema_before_reading() {
    let fixture = CatalogFixture::new();
    let cache = import_customization_cache_at(&fixture.root, &fixture.cache_path()).unwrap();
    let conn = Connection::open(&cache).unwrap();
    conn.execute_batch("DROP TABLE options;
        CREATE TABLE options (id INTEGER PRIMARY KEY, name TEXT NOT NULL, chr_model_id INTEGER NOT NULL);
        INSERT INTO options VALUES (890, 'stale', 1);
        PRAGMA user_version = 0;").unwrap();
    drop(conn);
    let raw = load_customization_raw_data_at(&fixture.root, &cache)
        .expect("normal loading must rebuild a previous-schema cache");
    assert_eq!(
        raw.options
            .iter()
            .find(|option| option.id == 890)
            .unwrap()
            .name,
        "Eyebrows"
    );
    assert_eq!(raw.categories[&3].name, "Accessories");
}

#[test]
fn catalog_cache_reads_actual_local_options_into_an_isolated_cache() {
    let fixture = CatalogFixture::new();
    let raw = load_customization_raw_data_at(Path::new("data"), &fixture.cache_path())
        .expect("import actual local customization data into temporary cache");
    let ears = raw.options.iter().find(|option| option.id == 8789).unwrap();
    assert_eq!(
        (
            ears.chr_model_id,
            ears.name.as_str(),
            ears.category_id,
            ears.order_index
        ),
        (1, "Ears", 23, 14)
    );
    let eyebrows = raw.options.iter().find(|option| option.id == 890).unwrap();
    assert_eq!(
        (eyebrows.chr_model_id, eyebrows.name.as_str()),
        (1, "Eyebrows")
    );
    let jewelry = raw.options.iter().find(|option| option.id == 776).unwrap();
    assert_eq!(
        (
            jewelry.chr_model_id,
            jewelry.name.as_str(),
            jewelry.category_id
        ),
        (20, "Jewelry Color", 5)
    );
    let gold = raw.choices.iter().find(|choice| choice.id == 8619).unwrap();
    assert_eq!((gold.option_id, gold.swatch_colors), (776, [-26_091, 0]));
    assert!(
        raw.elements
            .iter()
            .any(|element| element.choice_id == gold.id && element.material_id != 0)
    );
}

#[test]
fn catalog_cache_rejects_invalid_authored_swatch_instead_of_inventing_color() {
    let fixture = CatalogFixture::new();
    let path = fixture.root.join("ChrCustomizationChoice.csv");
    let csv = std::fs::read_to_string(&path)
        .unwrap()
        .replace("-26091", "not-a-color");
    std::fs::write(&path, csv).unwrap();
    let error = import_customization_cache_at(&fixture.root, &fixture.cache_path()).unwrap_err();
    assert!(
        error.contains("invalid signed integer \"not-a-color\""),
        "{error}"
    );
    assert!(error.contains("ChrCustomizationChoice.csv"), "{error}");
}

#[test]
fn catalog_cache_marks_each_unimplemented_element_kind() {
    for column in [
        "ChrCustomizationSkinnedModelID",
        "ChrCustomizationBoneSetID",
        "ChrCustomizationCondModelID",
        "ChrCustomizationDisplayInfoID",
        "ChrCustItemGeoModifyID",
        "ChrCustomizationVoiceID",
        "AnimKitID",
        "ParticleColorID",
        "ChrCustGeoComponentLinkID",
    ] {
        assert!(
            has_unsupported_effects(&[column.into()], &["41".into()]),
            "{column}"
        );
        assert!(
            !has_unsupported_effects(&[column.into()], &["0".into()]),
            "{column}"
        );
    }
    assert!(!has_unsupported_effects(
        &[
            "ChrCustomizationGeosetID".into(),
            "ChrCustomizationMaterialID".into()
        ],
        &["41".into(), "42".into()]
    ));
}
