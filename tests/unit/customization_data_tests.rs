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
            ..Default::default()
        },
        RawOption {
            id: 20,
            name: "Face Shape".into(),
            chr_model_id: 1,
            ..Default::default()
        },
        RawOption {
            id: 30,
            name: "Eyebrows".into(),
            chr_model_id: 1,
            ..Default::default()
        },
        RawOption {
            id: 40,
            name: "Piercings".into(),
            chr_model_id: 1,
            ..Default::default()
        },
        RawOption {
            id: 50,
            name: "Eyebrows".into(),
            chr_model_id: 3,
            ..Default::default()
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
        visibility_requirement_id: 23,
        swatch_colors: [-26_091, 0],
        order_index,
    })
    .collect();
    RawData {
        chr_models: vec![],
        options,
        categories: HashMap::new(),
        choices,
        elements: vec![
            RawElement {
                choice_id: 70_001,
                related_choice_id: 0,
                geoset_id: 1,
                material_id: 1,
                has_unsupported_effects: false,
            },
            RawElement {
                choice_id: 70_001,
                related_choice_id: 70_002,
                geoset_id: 2,
                material_id: 2,
                has_unsupported_effects: false,
            },
            RawElement {
                choice_id: 501,
                related_choice_id: 0,
                geoset_id: 3,
                material_id: 0,
                has_unsupported_effects: false,
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
fn catalog_exposes_every_authored_option_for_the_model() {
    let db = CustomizationDb::from_raw(&full_choice_lookup_fixture());
    let choices_by_option: Vec<Vec<u32>> = db
        .options_for(1, 0)
        .unwrap()
        .iter()
        .map(|option| option.choices.iter().map(|choice| choice.id).collect())
        .collect();
    assert_eq!(
        choices_by_option,
        vec![vec![501, 500], vec![70_001], vec![70_002], vec![70_003]],
        "Face Shape, Eyebrows and Piercings must remain available alongside Hair Style"
    );
    assert_eq!(db.options_for(2, 0).unwrap()[0].choices[0].id, 90_000);
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
    assert_eq!(choice.visibility_requirement_id, 23);
    assert_eq!(choice.swatch_colors, [-26_091, 0]);
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
    assert_eq!(options.len(), 4);
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
fn catalog_retains_authored_names_categories_order_and_requirement_ids() {
    let mut raw = full_choice_lookup_fixture();
    raw.categories = HashMap::from([
        (
            2,
            RawCategory {
                name: "Face".into(),
                order_index: 1,
                icon: 11991,
                selected_icon: 11990,
            },
        ),
        (
            3,
            RawCategory {
                name: "Accessories".into(),
                order_index: 3,
                icon: 11985,
                selected_icon: 11984,
            },
        ),
    ]);
    for option in &mut raw.options {
        option.category_id = 3;
        option.order_index = 10;
    }
    raw.options[0].category_id = 2;
    raw.options[0].order_index = 4;
    raw.options[1].category_id = 2;
    raw.options[1].order_index = 1;
    raw.options[2].name = "Sourcils".into();
    raw.options[2].ui_type = 2;
    raw.options[2].requirement_id = 12;
    let db = CustomizationDb::from_raw(&raw);
    assert_eq!(
        db.options_for(1, 0)
            .unwrap()
            .iter()
            .map(|o| o.id)
            .collect::<Vec<_>>(),
        vec![20, 10, 30, 40]
    );
    let option = db.option_by_id(1, 0, 30).unwrap();
    assert_eq!(option.display_name, "Sourcils");
    assert_eq!(option.option_type, OptionType::Additional(30));
    assert_eq!(
        (
            option.category_id,
            option.category_name.as_str(),
            option.category_order_index,
            option.order_index
        ),
        (3, "Accessories", 3, 10)
    );
    assert_eq!((option.ui_type, option.requirement_id), (2, 12));
    assert_eq!(
        (option.category_icon, option.category_selected_icon),
        (11985, 11984)
    );
    assert!(db.option_by_id(2, 0, 30).is_none());
    assert_eq!(
        db.get_choice(1, 0, OptionType::HairStyle, 0).unwrap().id,
        501
    );
}

#[test]
fn catalog_filtered_choice_ids_names_and_swatches_use_the_same_sequence() {
    let mut raw = full_choice_lookup_fixture();
    raw.options[0].name = "Face".into();
    raw.options[0].chr_model_id = 19;
    raw.choices[0].requirement_id = 142;
    raw.choices[1].requirement_id = 146;
    let mut db = CustomizationDb::from_raw(&raw);
    let option = &mut db.options_by_model.get_mut(&19).unwrap()[0];
    for (choice, color) in option.choices.iter_mut().zip([[17, 29, 43], [59, 71, 83]]) {
        choice.sample_swatch = true;
        choice.swatch_color_cache.set(Some(color)).unwrap();
    }
    let values = |class| {
        db.choices_for_option(10, 0, class, 10)
            .into_iter()
            .map(|choice| {
                (
                    choice.id,
                    choice.display_name.as_str(),
                    choice.swatch_color(),
                )
            })
            .collect::<Vec<_>>()
    };
    assert_eq!(values(1), vec![(500, "Hair second", Some([59, 71, 83]))]);
    assert_eq!(values(12), vec![(501, "Hair first", Some([17, 29, 43]))]);
    assert!(db.choices_for_option(10, 1, 1, 10).is_empty());
    assert!(db.choices_for_option(10, 0, 1, 999_999).is_empty());
}

#[test]
fn catalog_core_aliases_keep_the_original_lowest_id_choice_source() {
    let mut raw = full_choice_lookup_fixture();
    raw.options[0].name = "Beard".into();
    raw.options[0].order_index = 9;
    raw.options[1].name = "Mustache".into();
    raw.options[1].order_index = 1;
    let db = CustomizationDb::from_raw(&raw);
    assert_eq!(
        db.get_choice(1, 0, OptionType::FacialHair, 0).unwrap().id,
        501
    );
    assert_eq!(db.option_by_id(1, 0, 20).unwrap().choices[0].id, 70_001);
}

#[test]
fn catalog_marks_authored_unsupported_effects_without_rejecting_empty_choices() {
    let mut raw = full_choice_lookup_fixture();
    raw.elements[0].has_unsupported_effects = true;
    let db = CustomizationDb::from_raw(&raw);
    assert!(
        db.choice_by_id(1, 0, 70_001)
            .unwrap()
            .has_unsupported_effects
    );
    assert!(
        !db.choice_by_id(1, 0, 70_002)
            .unwrap()
            .has_unsupported_effects
    );
    assert!(!db.choice_by_id(1, 0, 501).unwrap().has_unsupported_effects);
}

#[test]
fn catalog_demon_hunter_filter_does_not_hide_other_races_authored_options() {
    let mut raw = full_choice_lookup_fixture();
    raw.options[0].name = "Eyesight".into();
    raw.options[1].name = "Horns".into();
    raw.options[1].chr_model_id = 21;
    let db = CustomizationDb::from_raw(&raw);
    assert_eq!(
        db.choices_for_option(1, 0, 1, 10).len(),
        2,
        "Human Eyesight remains selectable"
    );
    assert_eq!(
        db.choices_for_option(11, 0, 1, 20).len(),
        1,
        "Draenei Horns remain selectable"
    );
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
