use super::*;

fn load_test_db() -> CustomizationDb {
    crate::customization_cache::import_customization_cache(Path::new("data"))
        .expect("import customization cache");
    CustomizationDb::load(Path::new("data"))
}

fn full_choice_lookup_fixture() -> RawData {
    let options = vec![
        RawOption {
            id: 10,
            name: "Hair Style".into(),
            chr_model_id: 1,
        },
        RawOption {
            id: 20,
            name: "Face Shape".into(),
            chr_model_id: 1,
        },
        RawOption {
            id: 30,
            name: "Eyebrows".into(),
            chr_model_id: 1,
        },
        RawOption {
            id: 40,
            name: "Piercings".into(),
            chr_model_id: 1,
        },
        RawOption {
            id: 50,
            name: "Eyebrows".into(),
            chr_model_id: 3,
        },
    ];
    let choices = [
        (500, 10, "Hair second", 2),
        (501, 10, "Hair first", 1),
        (70_001, 20, "Face shape", 0),
        (70_002, 30, "Eyebrows", 0),
        (70_003, 40, "Piercing", 0),
        (90_000, 50, "Orc eyebrows", 0),
    ]
    .into_iter()
    .map(|(id, option_id, name, order_index)| RawChoice {
        id,
        option_id,
        name: name.into(),
        requirement_id: 19,
        order_index,
    })
    .collect();
    RawData {
        chr_models: vec![],
        options,
        choices,
        elements: vec![
            RawElement {
                choice_id: 70_001,
                related_choice_id: 0,
                geoset_id: 1,
                material_id: 1,
            },
            RawElement {
                choice_id: 70_001,
                related_choice_id: 70_002,
                geoset_id: 2,
                material_id: 2,
            },
            RawElement {
                choice_id: 501,
                related_choice_id: 0,
                geoset_id: 3,
                material_id: 0,
            },
        ],
        materials: HashMap::from([
            (
                1,
                RawMaterial {
                    texture_target_id: 6,
                    material_resources_id: 101,
                },
            ),
            (
                2,
                RawMaterial {
                    texture_target_id: 19,
                    material_resources_id: 102,
                },
            ),
        ]),
        geosets: HashMap::from([
            (
                1,
                RawGeoset {
                    geoset_type: 32,
                    geoset_id: 2,
                },
            ),
            (
                2,
                RawGeoset {
                    geoset_type: 34,
                    geoset_id: 4,
                },
            ),
            (
                3,
                RawGeoset {
                    geoset_type: 0,
                    geoset_id: 7,
                },
            ),
        ]),
        hair_geosets: HashMap::from([((1, 0, 7), true)]),
        texture_fdids: HashMap::from([(101, 1_020_001), (102, 1_020_002)]),
    }
}

#[test]
fn full_choice_lookup_resolves_unrecognized_options_and_related_elements() {
    let db = CustomizationDb::from_raw(&full_choice_lookup_fixture());
    for id in [70_001, 70_002, 70_003] {
        assert_eq!(db.choice_by_id(1, 0, id).map(|choice| choice.id), Some(id));
    }
    let choice = db.choice_by_id(1, 0, 70_001).unwrap();
    assert_eq!(choice.display_name, "Face shape");
    assert_eq!(choice.requirement_id, 19);
    assert_eq!(choice.materials, vec![(6, 1_020_001)]);
    assert_eq!(choice.geosets, vec![(32, 2)]);
    let related_materials: Vec<_> = choice
        .related_materials
        .iter()
        .map(|m| (m.related_choice_id, m.target_id, m.fdid))
        .collect();
    assert_eq!(related_materials, vec![(70_002, 19, 1_020_002)]);
    let related_geosets: Vec<_> = choice
        .related_geosets
        .iter()
        .map(|g| (g.related_choice_id, g.geoset_type, g.geoset_id))
        .collect();
    assert_eq!(related_geosets, vec![(70_002, 34, 4)]);
    assert!(!choice.shows_scalp);
}

#[test]
fn full_choice_lookup_is_scoped_to_race_and_sex() {
    let db = CustomizationDb::from_raw(&full_choice_lookup_fixture());
    assert!(db.choice_by_id(2, 0, 70_001).is_none());
    assert!(db.choice_by_id(1, 1, 70_001).is_none());
    assert!(db.choice_by_id(0, 0, 70_001).is_none());
    assert!(db.choice_by_id(1, 0, 999_999).is_none());
    assert_eq!(
        db.choice_by_id(2, 0, 90_000).map(|choice| choice.id),
        Some(90_000)
    );
    assert!(db.choice_by_id(1, 2, 90_000).is_none());
}

#[test]
fn full_choice_lookup_preserves_ui_choices_and_hair_scalp_semantics() {
    let db = CustomizationDb::from_raw(&full_choice_lookup_fixture());
    let options = db.options_for(1, 0).unwrap();
    assert_eq!(options.len(), 1);
    assert_eq!(options[0].option_type, OptionType::HairStyle);
    assert_eq!(db.choice_count(1, 0, OptionType::HairStyle), 2);
    assert_eq!(
        options[0]
            .choices
            .iter()
            .map(|choice| choice.id)
            .collect::<Vec<_>>(),
        vec![501, 500]
    );
    assert_eq!(
        db.get_choice(1, 0, OptionType::HairStyle, 0).unwrap().id,
        501
    );
    assert!(db.choice_by_id(1, 0, 501).unwrap().shows_scalp);
    assert!(!db.choice_by_id(1, 0, 500).unwrap().shows_scalp);
}

#[test]
fn chr_model_id_human() {
    assert_eq!(race_sex_to_chr_model_id(1, 0), Some(1));
    assert_eq!(race_sex_to_chr_model_id(1, 1), Some(2));
}

#[test]
fn chr_model_id_draenei() {
    assert_eq!(race_sex_to_chr_model_id(11, 0), Some(21));
    assert_eq!(race_sex_to_chr_model_id(11, 1), Some(22));
}

#[test]
fn load_customization_db() {
    let db = load_test_db();
    let count = db.choice_count(1, 0, OptionType::SkinColor);
    assert!(count > 0, "Human Male skin colors: {count}");
    let count = db.choice_count(1, 0, OptionType::HairStyle);
    assert!(count > 0, "Human Male hair styles: {count}");
}

#[test]
fn human_male_skin_has_materials() {
    let db = load_test_db();
    let choice = db.get_choice(1, 0, OptionType::SkinColor, 0).unwrap();
    assert!(
        !choice.materials.is_empty(),
        "Skin should have materials: {choice:?}"
    );
}

#[test]
fn human_male_hair_style_has_display_name() {
    let db = load_test_db();

    assert_eq!(db.choice_name(1, 0, OptionType::HairStyle, 0), Some("Bald"));
}

#[test]
fn blood_elf_face_choices_are_filtered_by_class() {
    let db = load_test_db();

    let warrior_faces = db.choice_count_for_class(10, 0, 1, OptionType::Face);
    let demon_hunter_faces = db.choice_count_for_class(10, 0, 12, OptionType::Face);

    assert_eq!(warrior_faces, 10);
    assert_eq!(demon_hunter_faces, 6);
    assert_eq!(
        db.get_choice_for_class(10, 0, 1, OptionType::Face, 0)
            .unwrap()
            .requirement_id,
        142
    );
    assert_eq!(
        db.get_choice_for_class(10, 0, 12, OptionType::Face, 0)
            .unwrap()
            .requirement_id,
        146
    );
}

#[test]
fn blood_elf_blindfold_choices_are_demon_hunter_only() {
    let db = load_test_db();

    assert_eq!(
        db.choice_count_for_class(10, 0, 1, OptionType::Blindfold),
        0
    );
    assert_eq!(
        db.choice_count_for_class(10, 0, 12, OptionType::Blindfold),
        12
    );
}

#[test]
fn blood_elf_horns_and_eyesight_are_demon_hunter_only() {
    let db = load_test_db();

    assert_eq!(db.choice_count_for_class(10, 0, 1, OptionType::Horns), 0);
    assert_eq!(db.choice_count_for_class(10, 0, 12, OptionType::Horns), 7);
    assert_eq!(db.choice_count_for_class(10, 0, 1, OptionType::Eyesight), 0);
    assert_eq!(
        db.choice_count_for_class(10, 0, 12, OptionType::Eyesight),
        4
    );
    assert_eq!(db.choice_count_for_class(10, 0, 1, OptionType::EyeStyle), 0);
    assert_eq!(
        db.choice_count_for_class(10, 0, 12, OptionType::EyeStyle),
        3
    );
}

#[test]
fn human_male_presentation_matches_chr_model_csv() {
    let db = load_test_db();
    let presentation = db.presentation_for(1, 0);

    assert!((presentation.customize_scale - 1.1).abs() < 0.001);
    assert!((presentation.camera_distance_offset - (-0.34)).abs() < 0.001);
}

#[test]
fn human_male_scalp_fallback_hair_geoset_comes_from_char_hair_geosets() {
    let db = load_test_db();
    assert_eq!(db.scalp_fallback_hair_geoset(1, 0), Some(0));
}

#[test]
fn troll_male_scalp_fallback_hair_geoset_uses_first_showscalp_geoset() {
    let db = load_test_db();
    assert_eq!(db.scalp_fallback_hair_geoset(8, 0), Some(8));
}

#[test]
#[ignore]
fn dump_human_male_eye_color_choices() {
    let db = load_test_db();
    let count = db.choice_count_for_class(1, 0, 1, OptionType::EyeColor);
    println!("human male eye color count={count}");
    for idx in 0..count {
        let choice = db
            .get_choice_for_class(1, 0, 1, OptionType::EyeColor, idx)
            .unwrap();
        println!(
            "idx={idx} id={} name={} mats={:?} related={:?}",
            choice.id,
            choice.display_name,
            choice.materials,
            choice
                .related_materials
                .iter()
                .map(|m| (m.related_choice_id, m.target_id, m.fdid))
                .collect::<Vec<_>>()
        );
    }
}

#[test]
#[ignore]
fn dump_human_male_face_and_eye_materials() {
    let db = CustomizationDb::load(Path::new("data"));
    let face = db
        .get_choice_for_class(1, 0, 1, OptionType::Face, 3)
        .unwrap();
    let eye = db
        .get_choice_for_class(1, 0, 1, OptionType::EyeColor, 0)
        .unwrap();
    println!(
        "face idx=3 id={} name={} mats={:?} related={:?}",
        face.id,
        face.display_name,
        face.materials,
        face.related_materials
            .iter()
            .map(|m| (m.related_choice_id, m.target_id, m.fdid))
            .collect::<Vec<_>>()
    );
    println!(
        "eye idx=0 id={} name={} mats={:?} related={:?}",
        eye.id,
        eye.display_name,
        eye.materials,
        eye.related_materials
            .iter()
            .map(|m| (m.related_choice_id, m.target_id, m.fdid))
            .collect::<Vec<_>>()
    );
}
