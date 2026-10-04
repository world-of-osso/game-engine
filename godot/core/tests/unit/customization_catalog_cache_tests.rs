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
                "ChrCustomizationChoiceID,RelatedChrCustomizationChoiceID,ChrCustomizationGeosetID,ChrCustomizationMaterialID,ChrCustomizationSkinnedModelID,ChrCustomizationBoneSetID,ChrCustomizationCondModelID,ChrCustomizationDisplayInfoID,ChrCustItemGeoModifyID,ChrCustomizationVoiceID,AnimKitID,ParticleColorID,ChrCustGeoComponentLinkID\n90001,0,1,0,0,0,0,0,0,0,0,0,0\n90001,0,0,0,7,0,0,0,0,0,0,0,0\n90002,0,0,0,0,0,0,0,0,0,0,0,0\n90003,0,0,1,0,7,0,0,0,0,0,0,0\n",
            ),
            (
                "ChrCustomizationMaterial",
                "ID,ChrModelTextureTargetID,MaterialResourcesID\n1,6,101\n",
            ),
            ("ChrCustomizationGeoset", "ID,GeosetType,GeosetID\n1,32,2\n"),
            (
                "ChrCustomizationSkinnedModel",
                "ID,CollectionsFileDataID,GeosetType,GeosetID,Modifier,Flags\n7,7760205,25,1,-1,0\n",
            ),
            (
                "CharHairGeosets",
                "RaceID,SexID,GeosetType,GeosetID,Showscalp\n1,0,0,7,1\n",
            ),
            (
                "ChrRaceXChrModel",
                "ID,ChrRacesID,ChrModelID,Sex,AllowedTransmogSlots\n1,1,1,0,0\n2,1,2,1,0\n",
            ),
            ("ChrRaces", "ID,UnalteredVisualRaceID\n1,0\n"),
            (
                "TextureFileData",
                "FileDataID,UsageType,MaterialResourcesID\n1020001,0,101\n1020002,2,101\n",
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
fn skyborne_imported_forever_catalog_and_retail_share_the_player_path() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../data");
    let db = crate::npc_appearance_assets::load_customization_db(&root).unwrap();
    let compositor = crate::npc_appearance_assets::load_compositor(&root).unwrap();
    assert_eq!(db.chr_model_id(1, 0), Some(1));
    assert_eq!(db.layout_id(1, 0), Some(103));
    for race in [95, 96] {
        let class = if race == 95 { 8 } else { 7 };
        for (sex, model, layout, count, skin) in [(0, 218, 201, 18, 9021), (1, 219, 202, 19, 9034)]
        {
            assert_eq!(db.chr_model_id(race, sex), Some(model));
            assert_eq!(db.layout_id(race, sex), Some(layout));
            assert_eq!(db.options_for(race, sex).unwrap().len(), count);
            let choices = db.offered_choices(race, sex, class, skin);
            assert!(!choices.is_empty(), "race {race} sex {sex} skin choices");
            assert!(choices.iter().any(|choice| !choice.materials.is_empty()));
            let canvas = compositor.layout(layout).unwrap();
            assert_eq!((canvas.width, canvas.height), (2048, 1024));
        }
    }
}

#[test]
fn skyborne_compositor_uses_forever_layouts_without_replacing_retail() {
    let fixture = CatalogFixture::new();
    let forever = fixture.root.join("db2/1.60.1.70205");
    std::fs::create_dir_all(&forever).unwrap();
    for (dir, ids) in [(&fixture.root, &[1][..]), (&forever, &[1, 201, 202][..])] {
        let mut layouts = String::from("ID,Width,Height\n");
        let mut sections =
            String::from("CharComponentTextureLayoutID,SectionType,X,Y,Width,Height\n");
        let mut layers = String::from(
            "TextureType,Layer,BlendMode,TextureSectionTypeBitMask,ChrModelTextureTargetID_0,CharComponentTextureLayoutsID\n",
        );
        let mut sizes = String::from("CharComponentTextureLayoutsID,TextureType,Width,Height\n");
        for id in ids {
            let width = if *id == 1 {
                if dir == &forever { 4 } else { 2 }
            } else {
                2048
            };
            let height = if *id == 1 { 1 } else { 1024 };
            layouts.push_str(&format!("{id},{width},{height}\n"));
            sections.push_str(&format!("{id},0,0,0,{width},{height}\n"));
            layers.push_str(&format!("1,0,0,1,6,{id}\n"));
            sizes.push_str(&format!("{id},1,{width},{height}\n"));
        }
        for (name, csv) in [
            ("CharComponentTextureLayouts", layouts),
            ("CharComponentTextureSections", sections),
            ("ChrModelTextureLayer", layers),
            ("ChrModelMaterial", sizes),
        ] {
            std::fs::write(dir.join(format!("{name}.csv")), csv).unwrap();
        }
    }
    crate::char_texture_cache::import_char_texture_cache(&fixture.root).unwrap();
    let compositor = crate::npc_appearance_assets::load_compositor(&fixture.root).unwrap();
    for (id, dimensions) in [(1, (2, 1)), (201, (2048, 1024)), (202, (2048, 1024))] {
        let (pixels, width, height) = compositor
            .composite_with(&[(6, 99)], id, |_| Some((vec![9, 18, 27, 255], 1, 1)))
            .unwrap();
        assert_eq!((width, height), dimensions);
        assert!(
            pixels
                .chunks_exact(4)
                .all(|pixel| pixel == [9, 18, 27, 255])
        );
    }
}

#[test]
fn skyborne_catalog_overlays_models_effects_and_colliding_requirements() {
    let retail = CatalogFixture::new();
    let source = CatalogFixture::new();
    let forever = retail.root.join("db2/1.60.1.70205");
    std::fs::create_dir_all(&forever).unwrap();
    for entry in std::fs::read_dir(&source.root).unwrap() {
        let entry = entry.unwrap();
        std::fs::copy(entry.path(), forever.join(entry.file_name())).unwrap();
    }
    for (name, contents) in [
        (
            "ChrModel",
            "ID,CharComponentTextureLayoutID,CustomizeScale,CameraDistanceOffset\n218,201,1,0\n219,202,1,0\n",
        ),
        (
            "ChrRaceXChrModel",
            "ChrRacesID,ChrModelID,Sex\n95,218,0\n95,219,1\n96,218,0\n96,219,1\n1,218,0\n",
        ),
        ("ChrRaces", "ID,UnalteredVisualRaceID\n95,0\n96,0\n"),
        (
            "ChrCustomizationOption",
            "Name_lang,ID,ChrModelID,ChrCustomizationCategoryID,OrderIndex,OptionType,Requirement\nSkin Color,500,218,3,0,0,12\nSkin Color,501,219,3,0,0,12\nHair Style,502,218,3,1,0,0\n",
        ),
        (
            "ChrCustomizationChoice",
            "Name_lang,ID,ChrCustomizationOptionID,ChrCustomizationReqID,OrderIndex,ChrCustomizationVisReqID,SwatchColor_0,SwatchColor_1\nBlue,95000,500,19,0,0,0,0\nBlue,95001,501,20,0,0,0,0\nLong,95002,502,0,0,0,0,0\n",
        ),
        (
            "ChrCustomizationElement",
            "ChrCustomizationChoiceID,RelatedChrCustomizationChoiceID,ChrCustomizationGeosetID,ChrCustomizationMaterialID,ChrCustomizationSkinnedModelID,ChrCustomizationBoneSetID,ChrCustomizationCondModelID,ChrCustomizationDisplayInfoID,ChrCustItemGeoModifyID,ChrCustomizationVoiceID,AnimKitID,ParticleColorID,ChrCustGeoComponentLinkID\n95000,0,1,1,0,0,0,0,0,0,0,0,0\n95001,0,1,1,0,0,0,0,0,0,0,0,0\n",
        ),
        (
            "CharHairGeosets",
            "RaceID,SexID,GeosetType,GeosetID,Showscalp\n95,0,0,7,1\n",
        ),
    ] {
        std::fs::write(forever.join(format!("{name}.csv")), contents).unwrap();
    }
    let header = "ID,ReqType,ClassMask,RaceMasks_0,RaceMasks_1,ReqAchievementID,ReqQuestID,ReqItemModifiedAppearanceID\n";
    std::fs::write(
        retail.root.join("ChrCustomizationReq.csv"),
        format!("{header}12,1,0,1,0,0,0,0\n19,2,0,0,0,0,0,0\n"),
    )
    .unwrap();
    std::fs::write(
        forever.join("ChrCustomizationReq.csv"),
        format!("{header}12,1,0,0,3,0,0,0\n19,1,128,0,3,0,0,0\n20,1,128,0,3,0,0,0\n"),
    )
    .unwrap();
    for dir in [&retail.root, &forever] {
        std::fs::write(
            dir.join("ChrCustomizationReqChoice.csv"),
            "ChrCustomizationReqID,ChrCustomizationChoiceID\n",
        )
        .unwrap();
    }
    std::fs::write(
        forever.join("ChrCustomizationReqChoice.csv"),
        "ChrCustomizationReqID,ChrCustomizationChoiceID\n19,95002\n",
    )
    .unwrap();
    import_customization_cache(&retail.root).unwrap();
    let db = crate::npc_appearance_assets::load_customization_db(&retail.root).unwrap();
    assert_eq!(db.chr_model_id(1, 0), Some(1));
    assert_eq!(db.options_for(1, 0).unwrap()[0].id, 890);
    for (race, sex, model, layout, option) in [
        (95, 0, 218, 201, 500),
        (95, 1, 219, 202, 501),
        (96, 0, 218, 201, 500),
        (96, 1, 219, 202, 501),
    ] {
        assert_eq!(db.chr_model_id(race, sex), Some(model));
        assert_eq!(db.layout_id(race, sex), Some(layout));
        let choices = db.offered_choices(race, sex, 8, option);
        assert_eq!(choices.len(), 1);
        assert_eq!(choices[0].materials[0].1, 1020001);
        assert_eq!(choices[0].geosets[0], (32, 2));
        assert!(
            db.offered_choices(race, sex, 1, option).is_empty(),
            "Mage-only fixture requirement"
        );
        if sex == 0 {
            let required = db.required_choices(choices[0]);
            assert_eq!(required.len(), 1);
            assert_eq!(required[0].option_id, 502);
            assert_eq!(required[0].choice_ids, [95002]);
        }
    }
    assert!(db.offered_choices(1, 0, 8, 500).is_empty());
    assert!(!db.offered_choices(1, 0, 8, 890).is_empty());
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
    assert_eq!(
        raw.texture_fdids[&101], 1020001,
        "a material resource resolves to its UsageType 0 texture"
    );
    let skinned = raw
        .elements
        .iter()
        .find(|element| element.skinned_model_id != 0)
        .unwrap();
    assert_eq!((skinned.choice_id, skinned.skinned_model_id), (90001, 7));
    assert!(!skinned.has_unsupported_effects, "skinned models render");
    let row = &raw.skinned_models[&7];
    assert_eq!(
        (row.collection_fdid, row.geoset_type, row.geoset_id),
        (7760205, 25, 1)
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
            "ChrCustomizationMaterialID".into(),
            "ChrCustomizationSkinnedModelID".into()
        ],
        &["41".into(), "42".into(), "43".into()]
    ));
}
