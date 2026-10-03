use std::path::PathBuf;
use std::sync::OnceLock;

use shared::protocol::InstanceLockInfo;

use super::*;

/// `JournalInstance` 238 "The Stockade" (Map 34); its `JournalInstanceEntrance` row 95.
const STOCKADE: u32 = 238;
const STOCKADE_MAP: u32 = 34;
const STOCKADE_ENTRANCE: [f32; 3] = [-8761.85, 848.557, 87.8052];
/// `JournalInstance` 63 "Deadmines" (Map 36): Normal, Heroic and Timewalking 24; Heroic
/// adds Vanessa VanCleef to Normal's five bosses.
const DEADMINES: u32 = 63;

fn catalog() -> &'static EntranceCatalog {
    static CATALOG: OnceLock<EntranceCatalog> = OnceLock::new();
    CATALOG.get_or_init(|| {
        let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        let dir = [
            root.join("data/db2/12.1.0.69933"),
            root.join("../../data/db2/12.1.0.69933"),
        ]
        .into_iter()
        .find(|dir| dir.join(JOURNAL_INSTANCE_ENTRANCE_CSV).exists())
        .expect("JournalInstanceEntrance.csv export (scripts/export_db2_csv.py)");
        EntranceCatalog::load(&dir).unwrap()
    })
}

/// A point `distance` yards east of the Stockade entrance, 5 yd above it.
fn near_stockade(distance: f32) -> [f32; 3] {
    let [x, y, z] = STOCKADE_ENTRANCE;
    [x, y + distance, z + 5.0]
}

fn lock(map_id: u32, difficulty_id: u32, completed_mask: u32, locked: bool) -> InstanceLockInfo {
    InstanceLockInfo {
        map_id,
        difficulty_id,
        instance_id: 1,
        time_remaining_secs: 3600,
        completed_mask,
        locked,
        extended: false,
    }
}

#[test]
fn stockade_journal_instance_and_entrance_resolve_from_the_tables() {
    let catalog = catalog();
    let journal = catalog.journal_instance(STOCKADE).unwrap();
    assert_eq!(journal.name, "The Stockade");
    assert_eq!(journal.map_id, STOCKADE_MAP);
    let entrances: Vec<_> = catalog
        .entrances()
        .iter()
        .filter(|entrance| entrance.journal_instance_id == STOCKADE)
        .collect();
    assert_eq!(entrances.len(), 1);
    assert_eq!(entrances[0].map_id, 0);
    for (axis, expected) in entrances[0].position.iter().zip(STOCKADE_ENTRANCE) {
        assert!(
            (axis - expected).abs() < 0.01,
            "{:?}",
            entrances[0].position
        );
    }
}

#[test]
fn raid_entrances_are_not_offered() {
    // Molten Core's entrance (JournalInstance 741, raid map 409) is on Blackrock Mountain.
    assert!(
        catalog()
            .entrances()
            .iter()
            .all(|entrance| entrance.journal_instance_id != 741)
    );
}

#[test]
fn the_bar_shows_within_31_yards_on_the_entrance_map_only() {
    let catalog = catalog();
    let at_30 = catalog.nearest_entrance(0, near_stockade(30.0)).unwrap();
    assert_eq!(at_30.journal_instance_id, STOCKADE);
    // Height is ignored: 2D distance, as Plumber's.
    assert!((at_30.distance - 30.0).abs() < 0.01, "{}", at_30.distance);
    assert!(at_30.shows_bar());
    let at_32 = catalog.nearest_entrance(0, near_stockade(32.0)).unwrap();
    assert!(!at_32.shows_bar());
    assert!(
        catalog
            .nearest_entrance(STOCKADE_MAP, near_stockade(0.0))
            .is_none()
    );
    // Kalimdor has entrances, none at Stormwind's coordinates.
    let kalimdor = catalog.nearest_entrance(1, near_stockade(0.0)).unwrap();
    assert!(!kalimdor.shows_bar(), "{kalimdor:?}");
}

#[test]
fn proximity_is_rechecked_twice_a_second_within_60_yards() {
    let catalog = catalog();
    let near = catalog.nearest_entrance(0, near_stockade(59.0)).unwrap();
    assert_eq!(near.recheck_secs(), 0.5);
    let far = catalog.nearest_entrance(0, near_stockade(61.0)).unwrap();
    assert_eq!(far.recheck_secs(), 1.0);
}

#[test]
fn stockade_offers_normal_with_three_bosses() {
    let selector = catalog().selector(STOCKADE, Some(1), &[]).unwrap();
    assert_eq!(selector.title, "The Stockade");
    assert_eq!(selector.map_id, STOCKADE_MAP);
    assert_eq!(
        selector.choices,
        vec![DifficultyChoice {
            difficulty_id: 1,
            label: "(5) Normal".into(),
            killed: 0,
            total: 3,
        }]
    );
    assert_eq!(selector.selected, Some(1));
    assert_eq!(selector.choices[0].progress_text(), "(0/3)");
    assert_eq!(selector.choices[0].progress_color(), PROGRESS_OPEN_RGBA);
}

#[test]
fn a_difficulty_the_dungeon_lacks_selects_nothing() {
    let selector = catalog().selector(STOCKADE, Some(23), &[]).unwrap();
    assert_eq!(selector.selected, None);
}

#[test]
fn deadmines_lists_normal_and_heroic_but_not_timewalking() {
    let selector = catalog().selector(DEADMINES, Some(2), &[]).unwrap();
    let labels: Vec<_> = selector
        .choices
        .iter()
        .map(|choice| (choice.label.as_str(), choice.total))
        .collect();
    assert_eq!(labels, [("(5) Normal", 5), ("(5) Heroic", 6)]);
    assert_eq!(selector.selected, Some(2));
}

#[test]
fn killed_bosses_come_from_the_locked_lock_of_that_difficulty() {
    // Hogger bit 3 and Lord Overheat bit 4 on the Stockade Normal lock.
    let locks = [lock(STOCKADE_MAP, 1, (1 << 3) | (1 << 4), true)];
    let selector = catalog().selector(STOCKADE, Some(1), &locks).unwrap();
    assert_eq!(selector.choices[0].killed, 2);
    assert_eq!(selector.choices[0].progress_text(), "(2/3)");
    let expired = [lock(STOCKADE_MAP, 1, 0b111000, false)];
    let selector = catalog().selector(STOCKADE, Some(1), &expired).unwrap();
    assert_eq!(selector.choices[0].killed, 0);
    let other_difficulty = [lock(STOCKADE_MAP, 2, 0b111000, true)];
    let selector = catalog()
        .selector(STOCKADE, Some(1), &other_difficulty)
        .unwrap();
    assert_eq!(selector.choices[0].killed, 0);
}

#[test]
fn a_cleared_difficulty_shows_its_count_in_red() {
    let locks = [lock(STOCKADE_MAP, 1, 0b111000, true)];
    let selector = catalog().selector(STOCKADE, Some(1), &locks).unwrap();
    assert_eq!(selector.choices[0].progress_text(), "(3/3)");
    assert_eq!(selector.choices[0].progress_color(), PROGRESS_CLEARED_RGBA);
}
