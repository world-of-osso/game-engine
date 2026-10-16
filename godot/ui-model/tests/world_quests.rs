use game_engine_ui_model::objective_tracker_component::{
    ObjectiveTrackerState, objective_tracker_screen,
};
use game_engine_ui_model::{quest_runtime::QuestRuntime, world_quest_ui};
use shared::protocol::*;
use ui_toolkit::{
    atlas::{ActiveSkin, set_thread_skin},
    frame::WidgetData,
    registry::FrameRegistry,
    screen::{Screen, SharedContext},
};

fn available() -> WorldQuestAvailability {
    WorldQuestAvailability {
        server_time: 1000,
        quests: vec![AvailableWorldQuest {
            quest_id: 86948,
            title: "Twilight Bonds".into(),
            map_id: 0,
            zone_id: 4922,
            ui_map_id: 241,
            position: QuestPoiPoint { x: -4655, y: -4310 },
            expires_at: 87400,
            rewards: QuestRewards::default(),
            xp: 11765,
            currencies: vec![],
            reputation: vec![],
        }],
    }
}

fn text(registry: &FrameRegistry, name: &str) -> String {
    match registry
        .get(registry.get_by_name(name).expect(name))
        .unwrap()
        .widget_data
        .as_ref()
        .unwrap()
    {
        WidgetData::FontString(value) => value.text.clone(),
        other => panic!("{name}: {other:?}"),
    }
}

#[test]
fn world_quests_available_pin_selects_without_accepting_or_granting_rewards() {
    let mut runtime = QuestRuntime::default();
    runtime.apply_availability(available(), 900);
    let mut ui = game_engine_ui_model::quest_runtime::QuestUiState::default();
    let effects = game_engine_ui_model::quest_actions::quest_ui_action(
        "world_quest:select:86948",
        &mut runtime,
        &mut ui,
    );
    assert_eq!(ui.super_tracked, Some(86948));
    assert!(effects.is_empty());
    assert!(runtime.log.is_empty());
    assert!(runtime.watched.is_empty());
    game_engine_ui_model::quest_actions::quest_ui_action(
        "world_quest:select:86948",
        &mut runtime,
        &mut ui,
    );
    assert_eq!(ui.super_tracked, None);
    game_engine_ui_model::quest_actions::quest_ui_action(
        "world_quest:select:45068",
        &mut runtime,
        &mut ui,
    );
    assert_eq!(ui.super_tracked, None);
}

#[test]
fn world_quests_received_positions_reach_minimap_without_an_accepted_log() {
    use game_engine_core::minimap_data::{MapMask, MinimapView};
    use game_engine_ui_model::minimap::cluster_style;
    let mut runtime = QuestRuntime::default();
    runtime.apply_availability(available(), 900);
    for skin in [ActiveSkin::Modern, ActiveSkin::Forever] {
        let style = cluster_style(skin);
        let view = MinimapView::new([-4655., 4310.], 0).masked(style.mask);
        let blips = game_engine_ui_model::quest_poi::quest_minimap_blips(
            &runtime,
            None,
            0,
            &view,
            style.map_size,
        );
        assert_eq!(blips.len(), 1, "received task missing in {skin:?}");
        assert_eq!(blips[0].unit, 86948);
        assert_eq!(blips[0].offset, [0., 0.]);
        assert!(runtime.log.is_empty());
        let wrong_map = game_engine_ui_model::quest_poi::quest_minimap_blips(
            &runtime,
            None,
            1,
            &view,
            style.map_size,
        );
        assert!(wrong_map.is_empty());
        assert_eq!(
            style.mask,
            if skin == ActiveSkin::Modern {
                MapMask::Round
            } else {
                MapMask::Square
            }
        );
    }
}

#[test]
fn world_quests_clock_tooltip_progress_and_expiry_both_skins() {
    let root = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../data");
    game_engine_ui_model::paths::set_data_root(root.clone()).unwrap();
    let catalog =
        game_engine_ui_model::ui_map_data::UiMapCatalog::load(&root.join("db2/active")).unwrap();
    for skin in [ActiveSkin::Modern, ActiveSkin::Forever] {
        set_thread_skin(skin);
        let mut runtime = QuestRuntime::default();
        runtime.apply_availability(available(), 900);
        let pins = world_quest_ui::map_pins(&catalog, 241, &runtime, 900);
        assert_eq!(pins.len(), 1);
        assert_eq!(pins[0].quest_id, Some(86948));
        assert!((0.0..=1.0).contains(&pins[0].x));
        let tooltip = world_quest_ui::tooltip(&runtime.world_quests[0], 4600, [20., 20.]);
        assert_eq!(tooltip.lines[0].left_text, "Time Remaining: 23h 0m");
        assert!(
            tooltip
                .lines
                .iter()
                .any(|line| line.left_text == "11765 XP")
        );
        runtime.apply_snapshot(QuestLogSnapshot {
            entries: vec![QuestEntrySnapshot {
                quest_id: 86948,
                title: "Twilight Bonds".into(),
                zone: "Twilight Highlands".into(),
                completed: false,
                expires_at: Some(87400),
                timer_sampled_at: Some(1000),
                failed: false,
                repeatability: QuestRepeatability::Normal,
                objectives: vec![QuestObjectiveSnapshot {
                    text: "Twilight enslavers slain".into(),
                    current: 4,
                    required: 8,
                    completed: false,
                    kind: QuestObjectiveKind::Monster,
                    object_id: 46327,
                    optional: false,
                }],
                level: -1,
                sort_id: 4922,
                objectives_text: String::new(),
                description: String::new(),
                rewards: shared::protocol::QuestRewards::default(),
                pushable: false,
                completion_text: String::new(),
                watched: true,
                pois: vec![],
            }],
            watched_quest_ids: vec![86948],
        });
        let mut ctx = SharedContext::new();
        ctx.insert(skin);
        ctx.insert(ObjectiveTrackerState::from_runtime_at(
            &runtime, false, false, 900,
        ));
        let mut registry = FrameRegistry::new(1280., 720.);
        Screen::new(objective_tracker_screen).sync(&ctx, &mut registry);
        assert_eq!(
            text(&registry, "WorldQuestObjectiveTrackerHeaderText"),
            "World Quests"
        );
        assert!(
            registry
                .get_by_name("WorldQuest86948ProgressLabel")
                .is_none()
        );
        assert_eq!(
            text(&registry, "QuestBlock86948Line0Text"),
            "4/8 Twilight enslavers slain"
        );
        runtime.apply_update(QuestLogUpdate {
            changed: vec![],
            removed: vec![86948],
            watched_quest_ids: vec![],
        });
        runtime.apply_availability(
            WorldQuestAvailability {
                server_time: 87400,
                quests: vec![],
            },
            87300,
        );
        assert!(world_quest_ui::map_pins(&catalog, 241, &runtime, 87300).is_empty());
        assert!(
            ObjectiveTrackerState::from_runtime_at(&runtime, false, false, 87300)
                .world_quests
                .is_empty()
        );
        ctx.insert(world_quest_ui::WorldQuestToast {
            title: "Twilight Bonds".into(),
            completed: true,
        });
        let mut registry = FrameRegistry::new(1280., 720.);
        Screen::new(world_quest_ui::toast_screen).sync(&ctx, &mut registry);
        assert_eq!(
            text(&registry, "WorldQuestToastStatus"),
            "World Quest Complete!"
        );
        assert_eq!(text(&registry, "WorldQuestToastTitle"), "Twilight Bonds");
    }
}
