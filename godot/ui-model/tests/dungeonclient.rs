use game_engine_ui_model::achievements::{AchievementWindow, achievement_screen};
use game_engine_ui_model::dungeon_progress::DungeonObjectives;
use game_engine_ui_model::objective_tracker_component::{
    ObjectiveTrackerState, objective_tracker_screen,
};
use shared::protocol::*;
use ui_toolkit::atlas::{ActiveSkin, set_thread_skin};
use ui_toolkit::frame::WidgetData;
use ui_toolkit::registry::FrameRegistry;
use ui_toolkit::screen::{Screen, SharedContext};

fn assets(skin: ActiveSkin) {
    game_engine_ui_model::paths::set_data_root(
        std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../data"),
    )
    .unwrap();
    set_thread_skin(skin);
}
fn text(registry: &FrameRegistry, name: &str) -> String {
    let frame = registry
        .get(registry.get_by_name(name).expect(name))
        .unwrap();
    match frame.widget_data.as_ref().unwrap() {
        WidgetData::FontString(value) => value.text.clone(),
        other => panic!("{name}: {other:?}"),
    }
}
fn tracker(objectives: &DungeonObjectives, skin: ActiveSkin) -> FrameRegistry {
    let mut state = ObjectiveTrackerState::default();
    state.dungeon = objectives.block.clone();
    let mut shared = SharedContext::new();
    shared.insert(skin);
    shared.insert(state);
    let mut registry = FrameRegistry::new(1920.0, 1080.0);
    Screen::new(objective_tracker_screen).sync(&shared, &mut registry);
    registry
}
#[test]
fn dungeonclient_stockade_checks_three_bosses_in_order_then_clears_both_skins() {
    for skin in [ActiveSkin::Modern, ActiveSkin::Forever] {
        assets(skin);
        let mut objectives = DungeonObjectives::default();
        objectives.begin_map(34);
        let names = ["Randolph Moloch", "Lord Overheat", "Hogger"];
        for killed in 0..=3 {
            assert!(
                objectives.apply(
                    DungeonProgress {
                        map_id: 34,
                        instance_id: 7,
                        difficulty_id: 1,
                        encounters: names
                            .iter()
                            .enumerate()
                            .map(|(i, name)| DungeonEncounterProgress {
                                encounter_id: i as u32 + 1,
                                name: (*name).into(),
                                defeated: i < killed,
                                optional: None,
                                flags: 0,
                            })
                            .collect(),
                    },
                    "The Stockade".into()
                )
            );
            let registry = tracker(&objectives, skin);
            assert_eq!(
                text(&registry, "DungeonObjectiveTrackerHeaderText"),
                "The Stockade"
            );
            for (i, name) in names.iter().enumerate() {
                assert_eq!(text(&registry, &format!("DungeonBoss{i}Text")), *name);
                assert_eq!(
                    registry
                        .get_by_name(&format!("DungeonBoss{i}Check"))
                        .is_some(),
                    i < killed
                );
            }
        }
        assert!(!objectives.apply(
            DungeonProgress {
                map_id: 34,
                instance_id: 8,
                difficulty_id: 1,
                encounters: vec![]
            },
            "wrong copy".into()
        ));
        objectives.begin_map(0);
        objectives.apply(
            DungeonProgress {
                map_id: 0,
                instance_id: 0,
                difficulty_id: 1,
                encounters: vec![],
            },
            "Eastern Kingdoms".into(),
        );
        assert!(
            tracker(&objectives, skin)
                .get_by_name("DungeonObjectiveTrackerHeaderText")
                .is_none()
        );
    }
}
fn category() -> AchievementCategoryEntry {
    AchievementCategoryEntry {
        category_id: 14808,
        parent_id: -1,
        order_index: 2,
        name: "Classic".into(),
    }
}
fn criterion(tree_id: u32) -> AchievementCriterionLine {
    AchievementCriterionLine {
        tree_id,
        order_index: tree_id as i32,
        criteria_id: 3666,
        description: "Hogger defeated".into(),
        current: 1,
        required: 1,
        completed: true,
        progress_supported: true,
    }
}
fn achievement() -> AchievementCatalogEntry {
    AchievementCatalogEntry {
        achievement_id: 633,
        name: "Stormwind Stockade".into(),
        description: "Defeat Hogger in the Stormwind Stockade.".into(),
        points: 10,
        icon_fdid: 136363,
        earned: true,
        earned_at: Some(1791244800),
        progress_supported: true,
        criteria: vec![criterion(1)],
        next_criteria_id: Some(1),
    }
}
#[test]
fn dungeonclient_launcher_requests_categories_and_catalog_pages_render_both_skins() {
    for skin in [ActiveSkin::Modern, ActiveSkin::Forever] {
        assets(skin);
        let mut launcher = game_engine_ui_model::launcher::LauncherView::default();
        launcher.action(game_engine_ui_model::launcher::ACTION_OPEN);
        let destination = launcher.action("micro:AchievementMicroButton").unwrap();
        let mut window = AchievementWindow::default();
        assert_eq!(
            window.action(&destination),
            vec![QueryAchievementCatalog::Categories { after_id: 0 }]
        );
        assert_eq!(
            window.apply(AchievementCatalogPage::Categories {
                categories: vec![category()],
                next_id: Some(14808)
            }),
            vec![QueryAchievementCatalog::Categories { after_id: 14808 }]
        );
        assert!(
            window
                .apply(AchievementCatalogPage::Categories {
                    categories: vec![],
                    next_id: None
                })
                .is_empty()
        );
        assert_eq!(
            window.action("achievement:category:14808"),
            vec![QueryAchievementCatalog::Category {
                category_id: 14808,
                after_id: 0
            }]
        );
        window.apply(AchievementCatalogPage::Category {
            category_id: 14808,
            achievements: vec![achievement()],
            next_id: Some(633),
        });
        assert_eq!(
            window.action("achievement:more_rows"),
            vec![QueryAchievementCatalog::Category {
                category_id: 14808,
                after_id: 633
            }]
        );
        window.apply(AchievementCatalogPage::Category {
            category_id: 14808,
            achievements: vec![],
            next_id: None,
        });
        assert_eq!(
            window.action("achievement:more_criteria"),
            vec![QueryAchievementCatalog::Criteria {
                achievement_id: 633,
                after_id: 1
            }]
        );
        window.apply(AchievementCatalogPage::Criteria {
            achievement_id: 633,
            criteria: vec![criterion(2)],
            next_id: None,
        });
        let mut shared = SharedContext::new();
        shared.insert(skin);
        shared.insert(window.clone());
        let mut registry = FrameRegistry::new(1920.0, 1080.0);
        Screen::new(achievement_screen).sync(&shared, &mut registry);
        assert_eq!(text(&registry, "AchievementCategory14808"), "Classic");
        assert_eq!(text(&registry, "Achievement633Name"), "Stormwind Stockade");
        assert_eq!(text(&registry, "Achievement633Points"), "10");
        assert_eq!(
            text(&registry, "Achievement633Date"),
            "Earned 2026-10-06 (UTC)"
        );
        assert_eq!(
            text(&registry, "Achievement633Criterion1"),
            "[Complete] Hogger defeated — 1/1"
        );
        assert_eq!(
            text(&registry, "Achievement633Criterion2"),
            "[Complete] Hogger defeated — 1/1"
        );
        assert_eq!(
            text(&registry, "AchievementPointsHeader"),
            "Loaded earned points: 10"
        );
        window.action("achievement:close");
        assert_eq!(window.action(&destination), vec![]);
        assert_eq!(window.action("achievement:category:14808"), vec![]);
    }
}
