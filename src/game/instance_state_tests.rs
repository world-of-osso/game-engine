use super::*;

fn catalog() -> InstanceCatalog {
    InstanceCatalog::load().expect("Difficulty.csv and Map.csv in data/db2/12.1.0.69933")
}

#[test]
fn the_catalog_names_retail_difficulties_and_maps() {
    let catalog = catalog();
    assert_eq!(
        MENU_DUNGEON_DIFFICULTIES.map(|id| catalog.difficulty_name(id).to_owned()),
        ["Normal", "Heroic", "Mythic"]
    );
    assert_eq!(catalog.map_name(670), "Grim Batol");
    assert_eq!(catalog.map_name(34), "Stormwind Stockade");
    assert_eq!(
        difficulty_changed_text(&catalog, 2),
        "Dungeon Difficulty set to Heroic."
    );
}

#[test]
fn the_instance_banner_follows_difficulty_flags() {
    let catalog = catalog();
    let banner = |id| catalog.difficulties[&id].banner_texture();
    // Normal 0x4, Heroic 0x5 (heroic-style lockouts), Mythic 0x85 (display mythic).
    assert_eq!(banner(1), BannerTexture::Normal);
    assert_eq!(banner(2), BannerTexture::Heroic);
    assert_eq!(banner(23), BannerTexture::Mythic);
}

#[test]
fn saved_instance_times_read_like_seconds_to_time() {
    assert_eq!(seconds_to_time(3 * 3600 + 5 * 60 + 9), "3 Hr 5 Min");
    assert_eq!(seconds_to_time(86_400 + 3600), "1 Day 1 Hr");
    assert_eq!(
        seconds_to_time(2 * 86_400 + 7 * 3600 + 60),
        "2 Days 7 Hr 1 Min"
    );
    assert_eq!(seconds_to_time(59), "");
}

#[test]
fn difficulty_selection_is_disabled_inside_an_instance_and_for_group_members() {
    let mut state = InstanceState {
        current_map: Some((0, 0)),
        ..Default::default()
    };
    assert!(dungeon_difficulty_enabled(&state, false, false));
    assert!(dungeon_difficulty_enabled(&state, true, true));
    assert!(!dungeon_difficulty_enabled(&state, true, false));
    state.current_map = Some((670, 2));
    assert!(!dungeon_difficulty_enabled(&state, false, false));
}

#[test]
fn a_lock_counts_down_from_when_the_list_arrived() {
    let lock = InstanceLockInfo {
        map_id: 670,
        difficulty_id: 2,
        instance_id: 3,
        time_remaining_secs: 10_800,
        completed_mask: 1 << 3,
        locked: true,
        extended: false,
    };
    let state = InstanceState {
        saved: vec![lock],
        saved_at: 100.0,
        ..Default::default()
    };
    assert_eq!(state.time_remaining(&lock, 160.0), 10_740);
}
