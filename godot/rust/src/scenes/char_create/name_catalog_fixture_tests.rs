use super::*;

fn with_catalog_fixture(test: impl FnOnce(&Path)) {
    static NEXT_FIXTURE: std::sync::atomic::AtomicUsize = std::sync::atomic::AtomicUsize::new(0);
    let sequence = NEXT_FIXTURE.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
    let root = std::env::temp_dir().join(format!("name-catalog-{}-{sequence}", std::process::id()));
    let overlay = root.join("db2/1.60.1.70205");
    std::fs::create_dir_all(&overlay).unwrap();
    std::fs::write(
        root.join("NameGen.csv"),
        "ID,Name,RaceID,Sex\n1,Anduin,1,0\n2,Jaina,1,1\n3,Chen,24,0\n",
    )
    .unwrap();
    std::fs::write(
        overlay.join("NameGen.csv"),
        concat!(
            "ID,Name,RaceID,Sex,NameType\n",
            "24985,Faladiel,95,0,0\n24987,Alaan,95,0,0\n",
            "24999,Arven,95,1,0\n25001,Arsa,95,1,0\n",
            "24986,Faladiel,96,0,0\n24988,Alaan,96,0,0\n",
            "25000,Arven,96,1,0\n25002,Arsa,96,1,0\n",
            "30000,Sunstrider,95,0,1\n30001,Sunstrider,95,1,1\n",
            "30002,Sunstrider,96,0,1\n30003,Sunstrider,96,1,1\n",
            "30004,A,95,0,0\n30005,TooLongForName,95,1,0\n",
            "30006,Not-a-name,96,0,0\n30007,Arvén,96,1,0\n",
            "30008,Replacement,1,0,0\n30009,Alias,24,0,0\n",
        ),
    )
    .unwrap();
    test(&root);
    std::fs::remove_dir_all(root).unwrap();
}

#[test]
fn forever_fixture_selects_only_valid_authored_first_names_for_all_four_groups() {
    with_catalog_fixture(|root| {
        let catalog = NameCatalog::load(&root.join("NameGen.csv")).unwrap();
        for (race, sex, expected) in [
            (95, 0, ["Faladiel", "Alaan"]),
            (95, 1, ["Arven", "Arsa"]),
            (96, 0, ["Faladiel", "Alaan"]),
            (96, 1, ["Arven", "Arsa"]),
        ] {
            assert!(catalog.has_names(race, sex), "race {race} sex {sex}");
            let mut selected = std::collections::HashSet::new();
            for seed in 0..32 {
                let name = catalog.pick_name(race, sex, "", seed).unwrap();
                assert!(
                    expected.contains(&name),
                    "unusable or fabricated name: {name}"
                );
                selected.insert(name);
                let next = catalog.pick_name(race, sex, name, seed).unwrap();
                assert_ne!(next, name);
                assert!(expected.contains(&next));
            }
            assert_eq!(selected.len(), 2);
        }
        assert!(catalog.pick_name(97, 0, "", 0).is_none());
    });
}

#[test]
fn forever_fixture_preserves_retail_names_and_pandaren_aliases() {
    with_catalog_fixture(|root| {
        let catalog = NameCatalog::load(&root.join("NameGen.csv")).unwrap();
        for seed in 0..32 {
            assert_eq!(catalog.pick_name(1, 0, "", seed), Some("Anduin"));
            assert_eq!(catalog.pick_name(1, 1, "", seed), Some("Jaina"));
            for race in [24, 25, 26] {
                assert_eq!(catalog.pick_name(race, 0, "", seed), Some("Chen"));
            }
        }
    });
}

#[test]
fn forever_fixture_missing_or_malformed_export_fails_explicitly() {
    with_catalog_fixture(|root| {
        let path = root.join("db2/1.60.1.70205/NameGen.csv");
        std::fs::remove_file(&path).unwrap();
        assert!(NameCatalog::load(&root.join("NameGen.csv")).is_err());
        for csv in [
            "ID,Name,RaceID,Sex\n1,Faladiel,95,0\n",
            "ID,Name,RaceID,Sex,NameType\n1,Faladiel,95,0,broken\n",
            "ID,Name,RaceID,Sex,NameType\n1,Sunstrider,95,0,1\n",
        ] {
            std::fs::write(&path, csv).unwrap();
            assert!(NameCatalog::load(&root.join("NameGen.csv")).is_err());
        }
    });
}
