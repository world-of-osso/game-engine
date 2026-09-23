use super::*;

#[test]
fn real_catalog_selects_authored_valid_name_for_race_and_sex() {
    let catalog = NameCatalog::load(std::path::Path::new("data/NameGen.csv")).unwrap();
    let source = std::fs::read_to_string("data/NameGen.csv").unwrap();
    let authored: Vec<_> = source
        .lines()
        .skip(1)
        .filter_map(|line| {
            let columns: Vec<_> = line.split(',').collect();
            (columns.len() == 4 && columns[2] == "1" && columns[3] == "0").then_some(columns[1])
        })
        .collect();
    let name = catalog.pick_name(1, 0, "", 17).unwrap();
    assert!(authored.contains(&name));
    assert!(name.len() <= 12);
    assert!(name.len() >= 2);
}

#[test]
fn every_selectable_race_and_body_type_has_authored_names() {
    let catalog = NameCatalog::load(std::path::Path::new("data/NameGen.csv")).unwrap();
    for race in game_engine::char_create_data::RACES {
        for sex in [0, 1] {
            assert!(
                catalog.has_names(race.id, sex),
                "race {} sex {sex}",
                race.id
            );
        }
    }
}

#[test]
fn pandaren_alias_uses_only_neutral_pandaren_names() {
    let catalog = NameCatalog::load(std::path::Path::new("data/NameGen.csv")).unwrap();
    let neutral = catalog.pick_name(24, 1, "", 123).unwrap();
    assert_eq!(catalog.pick_name(25, 1, "", 123), Some(neutral));
    assert_eq!(catalog.pick_name(26, 1, "", 123), Some(neutral));
    assert!(catalog.pick_name(99, 1, "", 123).is_none());
}

#[test]
fn repeated_selection_changes_name_and_does_not_change_sex() {
    let catalog = NameCatalog::load(std::path::Path::new("data/NameGen.csv")).unwrap();
    let previous = catalog.pick_name(3, 1, "", 123).unwrap();
    let next = catalog.pick_name(3, 1, previous, 123).unwrap();
    assert_ne!(previous, next);
    assert!(catalog.pick_name(3, 0, next, 123).is_some());
}

#[test]
fn missing_or_malformed_catalog_fails_explicitly() {
    assert!(NameCatalog::load(std::path::Path::new("data/not-a-name-catalog.csv")).is_err());
    assert!(NameCatalog::parse("ID,Name,RaceID,Sex\n1,Wrong,broken,0\n").is_err());
    assert!(NameCatalog::parse("wrong,headers\n").is_err());
    let short =
        NameCatalog::parse("ID,Name,RaceID,Sex\n1,A,1,0\n2,Twelveletters,1,0\n3,Valid,1,0\n")
            .unwrap();
    assert_eq!(short.pick_name(1, 0, "", 0), Some("Valid"));
    assert_eq!(short.pick_name(1, 0, "Valid", 0), Some("Valid"));
    assert!(short.pick_name(1, 1, "", 0).is_none());
}
