use game_engine_ui_model::raid_info::*;
use shared::protocol::{InstanceLockInfo, RaidDifficultySet, SetRaidDifficulty};

#[test]
fn retail_primary_and_legacy_size_selection_preserves_heroic_and_disables_mythic_size() {
    let selected = RaidDifficultySet {
        difficulty_id: 14,
        legacy_difficulty_id: 4,
    };
    assert_eq!(
        raid_selection_requests(selected, 15, false),
        vec![
            SetRaidDifficulty {
                difficulty_id: 15,
                legacy: false
            },
            SetRaidDifficulty {
                difficulty_id: 6,
                legacy: true
            }
        ]
    );
    assert_eq!(
        raid_selection_requests(
            RaidDifficultySet {
                difficulty_id: 15,
                legacy_difficulty_id: 6
            },
            3,
            true
        ),
        vec![SetRaidDifficulty {
            difficulty_id: 5,
            legacy: true
        }]
    );
    assert_eq!(
        raid_selection_requests(selected, 16, false),
        vec![SetRaidDifficulty {
            difficulty_id: 16,
            legacy: false
        }]
    );
    assert!(
        raid_selection_requests(
            RaidDifficultySet {
                difficulty_id: 16,
                legacy_difficulty_id: 4
            },
            3,
            true
        )
        .is_empty()
    );
}
#[test]
fn raid_info_retail_scrollbox_geometry_and_scroll_reaches_eighth_saved_instance() {
    use ui_toolkit::{
        atlas::{ActiveSkin, set_thread_skin},
        layout_values::Val,
        registry::FrameRegistry,
        screen::{Screen, SharedContext},
    };
    game_engine_ui_model::paths::set_data_root(
        std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../data"),
    )
    .unwrap();
    for skin in [ActiveSkin::Modern, ActiveSkin::Forever] {
        set_thread_skin(skin);
        let rows = [409, 469, 533, 249, 532, 565, 544, 580]
            .into_iter()
            .enumerate()
            .map(|(index, map_id)| SavedInstanceRow {
                lock: InstanceLockInfo {
                    map_id,
                    difficulty_id: 9,
                    instance_id: index as u32 + 1,
                    time_remaining_secs: 4000,
                    completed_mask: 0,
                    locked: true,
                    extended: false,
                },
                name: format!("Saved instance {map_id}"),
                difficulty: "Raid".into(),
                extend_disabled: false,
            })
            .collect();
        let state = RaidInfoState {
            visible: true,
            rows,
            ..Default::default()
        };
        let mut shared = SharedContext::new();
        shared.insert(state);
        shared.insert(skin);
        let mut registry = FrameRegistry::new(1920.0, 1080.0);
        let mut screen = Screen::new(game_engine_ui_model::raid_info::screen);
        screen.sync(&shared, &mut registry);
        let list = registry
            .get(registry.get_by_name("RaidInfoScrollBox").unwrap())
            .unwrap();
        assert_eq!((list.width.value(), list.height.value()), (292.0, 157.0));
        assert_eq!(
            (list.position.left, list.position.top),
            (Val::Px(16.0), Val::Px(48.0))
        );
        assert!(registry.scroll_lists.scroll_to("RaidInfoScrollBox", 10_000));
        screen.sync(&shared, &mut registry);
        assert_eq!(
            registry
                .scroll_lists
                .get("RaidInfoScrollBox")
                .unwrap()
                .first_row,
            163
        );
        let child = registry
            .get(registry.get_by_name("RaidInfoScrollChild").unwrap())
            .unwrap();
        assert_eq!(child.position.top, Val::Px(-163.0));
        let last = registry
            .get(registry.get_by_name("RaidInfoRow7").unwrap())
            .unwrap();
        assert_eq!(last.position.top, Val::Px(280.0));
        assert_eq!(last.onclick.as_deref(), Some("raid_info:select:7"));
    }
}

#[test]
fn saved_instance_selection_extends_unextends_and_reactivates_without_unsaving_active_lock() {
    let lock = InstanceLockInfo {
        map_id: 409,
        difficulty_id: 9,
        instance_id: 8,
        time_remaining_secs: 4000,
        completed_mask: 1,
        locked: true,
        extended: false,
    };
    let mut state = RaidInfoState {
        visible: true,
        rows: vec![SavedInstanceRow {
            lock,
            name: "Molten Core".into(),
            difficulty: "40 Player".into(),
            extend_disabled: false,
        }],
        ..Default::default()
    };
    assert!(state.action("raid_info:extend").is_none());
    state.action("raid_info:select:0");
    assert_eq!(state.extend_label(), "Extend Raid Lock");
    assert!(state.action("raid_info:extend").unwrap().extend);
    state.rows[0].lock.extended = true;
    assert_eq!(state.extend_label(), "Cancel Raid Lock Extension");
    assert!(!state.action("raid_info:extend").unwrap().extend);
    state.rows[0].lock.extended = false;
    state.rows[0].lock.locked = false;
    assert_eq!(state.extend_label(), "Reactivate Raid Lock");
    state.rows[0].extend_disabled = true;
    assert!(state.action("raid_info:extend").is_none());
    assert_eq!(state.rows[0].lock.completed_mask, 1);
}
#[test]
fn catalog_reads_authentic_raid_names_difficulty_and_extension_flag_and_selects_fourth_save() {
    let db2 =
        std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../data/db2/12.1.0.69933");
    let catalog = RaidCatalog::load(&db2).unwrap();
    let lock = InstanceLockInfo {
        map_id: 409,
        difficulty_id: 9,
        instance_id: 8,
        time_remaining_secs: 4000,
        completed_mask: 1,
        locked: true,
        extended: false,
    };
    let saves = [
        lock,
        InstanceLockInfo {
            map_id: 533,
            difficulty_id: 3,
            instance_id: 9,
            ..lock
        },
        InstanceLockInfo {
            map_id: 249,
            difficulty_id: 3,
            instance_id: 10,
            ..lock
        },
        InstanceLockInfo {
            map_id: 670,
            difficulty_id: 2,
            instance_id: 11,
            completed_mask: 1 << 3,
            ..lock
        },
    ];
    let rows = catalog.rows(&saves, 60).unwrap();
    assert_eq!(rows[0].name, "Molten Core");
    assert_eq!(rows[0].difficulty, "40 Player");
    assert_eq!(rows[0].lock.time_remaining_secs, 3940);
    assert!(!rows[0].extend_disabled);
    assert!(!catalog.rows(&[lock], 4000).unwrap()[0].lock.locked);
    assert!(!snapshot_reached_expiry(&[lock], 3999));
    assert!(snapshot_reached_expiry(&[lock], 4000));
    assert!(!snapshot_reached_expiry(
        &[InstanceLockInfo {
            time_remaining_secs: 0,
            ..lock
        }],
        4000
    ));
    let mut state = RaidInfoState {
        rows,
        ..Default::default()
    };

    state.action("raid_info:select:3");
    assert_eq!(state.selected_row().unwrap().lock.instance_id, 11);
    assert_eq!(state.selected_row().unwrap().lock.map_id, 670);
}
