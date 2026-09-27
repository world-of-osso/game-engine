use super::catalog::CreationSceneCatalog;
use crate::char_create_data::RACES;
use std::path::Path;

#[test]
fn production_catalog_covers_current_roster_and_authored_scenes() {
    let path = Path::new(env!("CARGO_MANIFEST_DIR")).join("data/ChrRaces.csv");
    let catalog = CreationSceneCatalog::load(&path).unwrap();
    for race in RACES {
        assert!(
            catalog.lookup(race.id).is_ok(),
            "{} ({})",
            race.name,
            race.id
        );
    }
    for (race, fdid) in [
        (1, 623712),
        (2, 623714),
        (24, 623716),
        (25, 623712),
        (26, 623714),
    ] {
        assert_eq!(catalog.lookup(race).unwrap(), fdid, "race {race}");
    }
}

#[test]
fn missing_columns_fail() {
    let err = parse_catalog("ID,FactionID\n1,1\n").unwrap_err();
    assert!(err.contains("CreateScreenFileDataID"), "{err}");
    let err = parse_catalog("CreateScreenFileDataID,FactionID\n623712,1\n").unwrap_err();
    assert!(err.contains("ID"), "{err}");
}

#[test]
fn invalid_ids_and_truncated_rows_fail() {
    for csv in [
        "ID,CreateScreenFileDataID\nnope,623712\n",
        "ID,CreateScreenFileDataID\n256,623712\n",
        "ID,CreateScreenFileDataID\n1,nope\n",
        "ID,CreateScreenFileDataID\n1,-1\n",
        "ID,CreateScreenFileDataID\n1\n",
    ] {
        assert!(parse_catalog(csv).is_err(), "accepted {csv}");
    }
}

#[test]
fn zero_scene_rows_are_skipped_without_guessing_for_absent_races() {
    let catalog = parse_catalog("CreateScreenFileDataID,ID\n0,12\n623716,24\n623712,25\n").unwrap();
    assert_eq!(catalog.lookup(24).unwrap(), 623716);
    assert_eq!(catalog.lookup(25).unwrap(), 623712);
    assert!(catalog.lookup(12).is_err());
    assert!(catalog.lookup(99).is_err());
}

fn parse_catalog(csv: &str) -> Result<CreationSceneCatalog, String> {
    CreationSceneCatalog::parse(csv.as_bytes(), Path::new("fixture.csv"))
}
