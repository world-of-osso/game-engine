use super::*;
use shared::protocol::{
    AchievementCatalogEntry, AchievementCatalogPage, AchievementCategoryEntry,
    AchievementCriterionLine, AchievementScope, AchievementStateUpdate, DungeonEncounterProgress,
    DungeonProgress, QueryAchievementCatalog,
};

#[test]
fn follower_queue_native_account_retains_server_rejection_and_match_state() {
    let mut account = account();
    let mut output = Vec::new();
    for error in [Some("follower map not imported".to_owned()), None] {
        use shared::protocol::{
            GroupRoleSnapshot as Role, LfgMatchFoundSnapshot, LfgMatchMemberSnapshot,
        };
        let match_found = error.is_none().then(|| LfgMatchFoundSnapshot {
            dungeon_id: 2436,
            dungeon_name: "Ruby Life Pools".into(),
            assigned_role: Role::Damage,
            members: [
                ("Alice", Role::Damage),
                ("Captain Garrick", Role::Tank),
                ("Crenna Earth-Daughter", Role::Healer),
                ("Meredy Huntswell", Role::Damage),
                ("Shuja Grimaxe", Role::Damage),
            ]
            .into_iter()
            .map(|(name, role)| LfgMatchMemberSnapshot {
                name: name.into(),
                role,
            })
            .collect(),
        });
        let update = shared::protocol::LfgStateUpdate {
            snapshot: Some(shared::protocol::LfgSnapshot {
                queued: false,
                selected_role: Some(shared::protocol::GroupRoleSnapshot::Damage),
                dungeon_ids: vec![2436],
                queue_size: 0,
                average_wait_secs: 0,
                in_demand_roles: vec![],
                role_check: None,
                match_found,
            }),
            message: None,
            error,
        };
        account
            .dispatch_message(ProtocolMessage::for_tests(update.clone()), &mut output)
            .unwrap();
        assert_eq!(account.lfg, Some(update));
    }
}

#[test]
fn scenarios_account_reduces_entry_completion_and_clears_before_transfer_snapshots() {
    use shared::protocol::{ScenarioProgress, ScenarioStage};
    let mut account = account();
    let mut output = Vec::new();
    let update = ScenarioProgress {
        map_id: 90,
        instance_id: 17,
        difficulty_id: 1,
        scenario: Some(ScenarioStage {
            scenario_id: 649,
            name: "Gnomeregan".into(),
            kind: 3,
            flags: 10,
            step_id: 1176,
            stage: 1,
            num_stages: 1,
            title: "Gnomeregan".into(),
            description: "Defeat Mekgineer Thermaplugg.".into(),
            completed: false,
            criteria: vec![],
        }),
    };
    account
        .dispatch_message(ProtocolMessage::for_tests(update.clone()), &mut output)
        .unwrap();
    assert_eq!(account.scenario_objectives.block.as_ref().unwrap().stage, 1);
    let mut completed = update.clone();
    let stage = completed.scenario.as_mut().unwrap();
    stage.stage = 2;
    stage.step_id = 0;
    stage.completed = true;
    account
        .dispatch_message(ProtocolMessage::for_tests(completed), &mut output)
        .unwrap();
    assert!(
        account
            .scenario_objectives
            .block
            .as_ref()
            .unwrap()
            .completed
    );
    account
        .dispatch_message(
            ProtocolMessage::for_tests(NewWorld {
                map_id: 0,
                map_directory: "Azeroth".into(),
                position: [0.; 3],
                facing: 0.,
            }),
            &mut output,
        )
        .unwrap();
    assert!(account.scenario_objectives.block.is_none());
    account
        .dispatch_message(ProtocolMessage::for_tests(update), &mut output)
        .unwrap();
    assert!(account.scenario_objectives.block.is_none());
}

fn account() -> Account {
    Account::new(PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../data"))
}
#[test]
fn dungeonclient_account_dispatches_stockade_and_rejects_old_copy_after_transfer() {
    let mut account = account();
    let mut output = Vec::new();
    account.dungeon_objectives.begin_map(34);
    for defeated in 0..=3 {
        let snapshot = DungeonProgress {
            map_id: 34,
            instance_id: 17,
            difficulty_id: 1,
            encounters: ["Randolph Moloch", "Lord Overheat", "Hogger"]
                .into_iter()
                .enumerate()
                .map(|(i, name)| DungeonEncounterProgress {
                    encounter_id: i as u32 + 1,
                    name: name.into(),
                    defeated: i < defeated,
                    optional: None,
                    flags: 0,
                })
                .collect(),
        };
        account
            .dispatch_message(ProtocolMessage::for_tests(snapshot), &mut output)
            .unwrap();
        let block = account.dungeon_objectives.block.as_ref().unwrap();
        assert_eq!(block.name, "Stormwind Stockade");
        assert_eq!(block.bosses.len(), 3);
        for (i, boss) in block.bosses.iter().enumerate() {
            assert_eq!(boss.style == game_engine_ui_model::objective_tracker_component::ObjectiveLineStyle::Completed, i < defeated);
        }
    }
    account
        .dispatch_message(
            ProtocolMessage::for_tests(NewWorld {
                map_id: 0,
                map_directory: "Azeroth".into(),
                position: [0.0; 3],
                facing: 0.0,
            }),
            &mut output,
        )
        .unwrap();
    assert!(account.dungeon_objectives.block.is_none());
    account
        .dispatch_message(
            ProtocolMessage::for_tests(DungeonProgress {
                map_id: 34,
                instance_id: 17,
                difficulty_id: 1,
                encounters: vec![DungeonEncounterProgress {
                    encounter_id: 1,
                    name: "Hogger".into(),
                    defeated: true,
                    optional: None,
                    flags: 0,
                }],
            }),
            &mut output,
        )
        .unwrap();
    assert!(account.dungeon_objectives.block.is_none());
    account
        .dispatch_message(
            ProtocolMessage::for_tests(DungeonProgress {
                map_id: 0,
                instance_id: 0,
                difficulty_id: 1,
                encounters: vec![],
            }),
            &mut output,
        )
        .unwrap();
    assert!(account.dungeon_objectives.block.is_none());
}
#[test]
fn dungeonclient_account_catalog_continuations_and_live_toast_reach_host() {
    let mut account = account();
    let mut output = Vec::new();
    assert_eq!(
        account.achievements.action("micro:AchievementMicroButton"),
        vec![
            QueryAchievementCatalog::Categories { after_id: 0 },
            QueryAchievementCatalog::Summary {
                scope: AchievementScope::All
            }
        ]
    );
    account
        .dispatch_message(
            ProtocolMessage::for_tests(AchievementCatalogPage::Categories {
                categories: vec![AchievementCategoryEntry {
                    category_id: 14808,
                    parent_id: -1,
                    order_index: 0,
                    name: "Classic".into(),
                }],
                next_id: Some(14808),
            }),
            &mut output,
        )
        .unwrap();
    assert_eq!(
        std::mem::take(&mut account.achievement_requests),
        vec![QueryAchievementCatalog::Categories { after_id: 14808 }]
    );
    account.achievements.action("achievement:category:14808");
    account
        .dispatch_message(
            ProtocolMessage::for_tests(AchievementCatalogPage::Category {
                category_id: 14808,
                achievements: vec![AchievementCatalogEntry {
                    achievement_id: 633,
                    name: "Stormwind Stockade".into(),
                    description: "Defeat Hogger".into(),
                    points: 10,
                    icon_fdid: 136363,
                    flags: 0,
                    statistic_value: None,
                    earned: false,
                    earned_at: None,
                    progress_supported: true,
                    criteria: vec![AchievementCriterionLine {
                        tree_id: 1,
                        order_index: 0,
                        criteria_id: 3666,
                        description: "Hogger defeated".into(),
                        flags: 0,
                        current: 0,
                        required: 1,
                        completed: false,
                        progress_supported: true,
                    }],
                    next_criteria_id: None,
                }],
                next_id: None,
            }),
            &mut output,
        )
        .unwrap();
    assert_eq!(account.achievements.selected().unwrap().achievement_id, 633);
    let toast = shared::protocol::AchievementToastSnapshot {
        achievement_id: 633,
        name: "Stormwind Stockade".into(),
        points: 10,
        icon_fdid: 134163,
    };
    account
        .dispatch_message(
            ProtocolMessage::for_tests(AchievementStateUpdate {
                snapshot: None,
                completed: Some(toast.clone()),
                message: None,
                error: None,
            }),
            &mut output,
        )
        .unwrap();
    assert_eq!(
        account.achievement_requests,
        vec![
            QueryAchievementCatalog::Summary {
                scope: AchievementScope::All
            },
            QueryAchievementCatalog::Category {
                category_id: 14808,
                after_id: 0
            }
        ]
    );
    assert!(
        matches!(output.as_slice(),[AccountEvent::Achievement(update)] if update.completed.as_ref()==Some(&toast))
    );
    let mut shared = ui_toolkit::screen::SharedContext::new();
    shared.insert(toast);
    let mut registry = ui_toolkit::registry::FrameRegistry::new(1920.0, 1080.0);
    ui_toolkit::screen::Screen::new(
        game_engine_ui_achievements::achievements::achievement_toast_screen,
    )
    .sync(&shared, &mut registry);
    let name = registry
        .get(registry.get_by_name("AchievementToastName").unwrap())
        .unwrap();
    assert!(
        matches!(&name.widget_data,Some(ui_toolkit::frame::WidgetData::FontString(text)) if text.text=="Stormwind Stockade")
    );
}

#[test]
fn ready_check_difficulty_accepts_authoritative_instance_and_continent_updates() {
    let mut account = account();
    let catalog = game_engine_ui_model::ready_check_difficulty::ReadyCheckDifficulties::load(
        &std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../data/db2/active"),
    )
    .unwrap();
    account.group.is_raid = true;
    account
        .dispatch_message(
            ProtocolMessage::for_tests(shared::protocol::RaidDifficultySet {
                difficulty_id: 14,
                legacy_difficulty_id: 6,
            }),
            &mut Vec::new(),
        )
        .unwrap();
    account
        .dispatch_message(
            ProtocolMessage::for_tests(shared::protocol::DungeonDifficultySet { difficulty_id: 1 }),
            &mut Vec::new(),
        )
        .unwrap();
    for (map_id, difficulty_id, expected) in [
        (631, 3, Some("10 Player")),
        (0, 0, None),
        (631, 5, Some("10 Player (Heroic)")),
    ] {
        let result = account.dispatch_message(
            ProtocolMessage::for_tests(shared::protocol::WorldServerInfo {
                map_id,
                difficulty_id,
            }),
            &mut Vec::new(),
        );
        assert!(
            result.is_ok(),
            "authoritative copy difficulty rejected: {result:?}"
        );
        assert_eq!(
            account.world_server_info,
            Some(shared::protocol::WorldServerInfo {
                map_id,
                difficulty_id
            })
        );
        assert_eq!(
            account
                .ready_check_difficulty_name(&catalog)
                .unwrap()
                .as_deref(),
            expected
        );
    }
    account
        .dispatch_message(
            ProtocolMessage::for_tests(shared::protocol::NewWorld {
                map_id: 0,
                map_directory: "azeroth".into(),
                position: [0.0; 3],
                facing: 0.0,
            }),
            &mut Vec::new(),
        )
        .unwrap();
    assert_eq!(
        account.world_server_info, None,
        "transfer must discard previous copy difficulty"
    );
    assert_eq!(account.ready_check_difficulty_name(&catalog).unwrap(), None);
    // Out of instance: primary raid selection, never the legacy selection.
    account.group.is_raid = false;
    assert_eq!(account.ready_check_difficulty_name(&catalog).unwrap(), None);
    account.group.is_raid = true;
    account
        .dispatch_message(
            ProtocolMessage::for_tests(shared::protocol::RaidDifficultySet {
                difficulty_id: 4,
                legacy_difficulty_id: 6,
            }),
            &mut Vec::new(),
        )
        .unwrap();
    assert_eq!(
        account
            .ready_check_difficulty_name(&catalog)
            .unwrap()
            .as_deref(),
        Some("25 Player")
    );
}

#[test]
fn raid_entry_account_accepts_primary_and_legacy_selection_from_server() {
    let mut account = account();
    let result = account.dispatch_message(
        ProtocolMessage::for_tests(shared::protocol::RaidDifficultySet {
            difficulty_id: 15,
            legacy_difficulty_id: 6,
        }),
        &mut Vec::new(),
    );
    assert!(result.is_ok(), "raid selection rejected: {result:?}");
}

#[test]
fn raid_entry_terrain_refresh_cannot_replace_the_world_selected_by_new_world() {
    assert!(super::validate_terrain_refresh(Some("moltencore"), "azeroth").is_err());
    assert!(super::validate_terrain_refresh(Some("azeroth"), "moltencore").is_err());
    assert!(super::validate_terrain_refresh(Some("moltencore"), "moltencore").is_ok());
    assert!(super::validate_terrain_refresh(None, "azeroth").is_ok());
}

#[test]
fn follower_group_finder_account_dispatches_catalog_and_authoritative_state() {
    let mut account = account();
    let mut output = Vec::new();
    account
        .dispatch_message(
            ProtocolMessage::for_tests(shared::protocol::FollowerDungeons {
                entries: vec![shared::protocol::FollowerDungeonEntry {
                    id: 2436,
                    name: "Ruby Life Pools".into(),
                    map_id: 2521,
                }],
            }),
            &mut output,
        )
        .unwrap();
    assert_eq!(
        output.len(),
        1,
        "catalog must reach the native Group Finder"
    );
    output.clear();
    let update = shared::protocol::LfgStateUpdate {
        snapshot: None,
        message: None,
        error: None,
    };
    account
        .dispatch_message(ProtocolMessage::for_tests(update.clone()), &mut output)
        .unwrap();
    assert_eq!(account.lfg, Some(update));
    assert_eq!(
        output.len(),
        1,
        "LFG state must reach both the native window and IPC status"
    );
}
