use super::*;

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
    let db = CustomizationDb::load(Path::new("data"));
    let count = db.choice_count(1, 0, OptionType::SkinColor);
    assert!(count > 0, "Human Male skin colors: {count}");
    let count = db.choice_count(1, 0, OptionType::HairStyle);
    assert!(count > 0, "Human Male hair styles: {count}");
}

#[test]
fn human_male_skin_has_materials() {
    let db = CustomizationDb::load(Path::new("data"));
    let choice = db.get_choice(1, 0, OptionType::SkinColor, 0).unwrap();
    assert!(
        !choice.materials.is_empty(),
        "Skin should have materials: {choice:?}"
    );
}

#[test]
fn human_male_hair_style_has_display_name() {
    let db = CustomizationDb::load(Path::new("data"));

    assert_eq!(db.choice_name(1, 0, OptionType::HairStyle, 0), Some("Bald"));
}

#[test]
fn blood_elf_face_choices_are_filtered_by_class() {
    let db = CustomizationDb::load(Path::new("data"));

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
    let db = CustomizationDb::load(Path::new("data"));

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
    let db = CustomizationDb::load(Path::new("data"));

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
    let db = CustomizationDb::load(Path::new("data"));
    let presentation = db.presentation_for(1, 0);

    assert!((presentation.customize_scale - 1.1).abs() < 0.001);
    assert!((presentation.camera_distance_offset - (-0.34)).abs() < 0.001);
}

#[test]
fn human_male_scalp_fallback_hair_geoset_comes_from_char_hair_geosets() {
    let db = CustomizationDb::load(Path::new("data"));
    assert_eq!(db.scalp_fallback_hair_geoset(1, 0), Some(0));
}

#[test]
fn troll_male_scalp_fallback_hair_geoset_uses_first_showscalp_geoset() {
    let db = CustomizationDb::load(Path::new("data"));
    assert_eq!(db.scalp_fallback_hair_geoset(8, 0), Some(8));
}

#[test]
#[ignore]
fn dump_human_male_eye_color_choices() {
    let db = CustomizationDb::load(Path::new("data"));
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
