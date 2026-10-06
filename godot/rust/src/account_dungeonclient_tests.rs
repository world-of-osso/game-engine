use super::*;
use shared::protocol::{
    AchievementCatalogEntry, AchievementCatalogPage, AchievementCategoryEntry,
    AchievementCriterionLine, AchievementStateUpdate, DungeonEncounterProgress, DungeonProgress,
    QueryAchievementCatalog,
};

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
        vec![QueryAchievementCatalog::Categories { after_id: 0 }]
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
                    earned: false,
                    earned_at: None,
                    progress_supported: true,
                    criteria: vec![AchievementCriterionLine {
                        tree_id: 1,
                        order_index: 0,
                        criteria_id: 3666,
                        description: "Hogger defeated".into(),
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
        vec![QueryAchievementCatalog::Category {
            category_id: 14808,
            after_id: 0
        }]
    );
    assert!(
        matches!(output.as_slice(),[AccountEvent::Achievement(update)] if update.completed.as_ref()==Some(&toast))
    );
    let mut shared = ui_toolkit::screen::SharedContext::new();
    shared.insert(toast);
    let mut registry = ui_toolkit::registry::FrameRegistry::new(1920.0, 1080.0);
    ui_toolkit::screen::Screen::new(game_engine_ui_model::achievements::achievement_toast_screen)
        .sync(&shared, &mut registry);
    let name = registry
        .get(registry.get_by_name("AchievementToastName").unwrap())
        .unwrap();
    assert!(
        matches!(&name.widget_data,Some(ui_toolkit::frame::WidgetData::FontString(text)) if text.text=="Stormwind Stockade")
    );
}
