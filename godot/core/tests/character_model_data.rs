use game_engine_core::character_model_data::race_model_wow_path;

#[test]
fn roster_race_and_sex_select_authored_models() {
    for (race, sex, expected) in [
        (1, 0, "character/human/male/humanmale_hd.m2"),
        (1, 1, "character/human/female/humanfemale_hd.m2"),
        (10, 0, "character/bloodelf/male/bloodelfmale_hd.m2"),
        (10, 1, "character/bloodelf/female/bloodelffemale_hd.m2"),
        (27, 0, "character/nightborne/male/nightbornemale.m2"),
        (27, 1, "character/nightborne/female/nightbornefemale.m2"),
        (36, 0, "character/orc/male/orcmale_hd.m2"),
    ] {
        assert_eq!(race_model_wow_path(race, sex), Some(expected));
    }
}

#[test]
fn unknown_race_or_sex_has_no_authored_model() {
    assert_eq!(race_model_wow_path(99, 0), None);
    assert_eq!(race_model_wow_path(1, 2), None);
}
