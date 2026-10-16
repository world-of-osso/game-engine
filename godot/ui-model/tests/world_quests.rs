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
            zone_id: 4922,
            pois: vec![WorldQuestPoi {
                map_id: 0,
                ui_map_id: 241,
                position: QuestPoiPoint { x: -4655, y: -4310 },
            }],
            expires_at: 87400,
            rewards: QuestRewards::default(),
            xp: 11765,
            currencies: vec![],
            reputation: vec![],
        }],
    }
}

#[derive(serde::Deserialize)]
struct PinFixture {
    quests: Vec<PinFixtureQuest>,
}

#[derive(serde::Deserialize)]
struct PinFixtureQuest {
    quest_id: u32,
    title: String,
    zone_id: u32,
    pois: Vec<WorldQuestPoi>,
}

fn received_pin_fixture() -> WorldQuestAvailability {
    let fixture: PinFixture =
        serde_json::from_str(include_str!("fixtures/world_quest_pins_70391.json"))
            .expect("authentic239-task/309-area pin fixture");
    WorldQuestAvailability {
        server_time: 1000,
        quests: fixture
            .quests
            .into_iter()
            .map(|quest| AvailableWorldQuest {
                quest_id: quest.quest_id,
                title: quest.title,
                zone_id: quest.zone_id,
                pois: quest.pois,
                expires_at: 87400,
                rewards: QuestRewards::default(),
                xp: 0,
                currencies: vec![],
                reputation: vec![],
            })
            .collect(),
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
fn world_quests_authored_lower_floor_pin_does_not_invent_height_zero() {
    let root = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../data");
    let catalog = game_engine_ui_model::ui_map_data::UiMapCatalog::load(&root.join("db2/active"))
        .expect("installed UiMap catalog");
    // Real 70391 task82521/blob514681. UiMap2256's relevant ceilings are below -1360.
    let mut quest = available().quests.remove(0);
    quest.quest_id = 82521;
    quest.title = "Pawns of Dark Masters".into();
    quest.pois = vec![WorldQuestPoi {
        map_id: 2601,
        ui_map_id: 2256,
        position: QuestPoiPoint { x: -2268, y: -1960 },
    }];
    quest.xp = 0;
    let mut runtime = QuestRuntime::default();
    runtime.apply_availability(
        WorldQuestAvailability {
            server_time: 1000,
            quests: vec![quest],
        },
        900,
    );
    assert!(
        catalog
            .map_position(2256, 2601, [-2268., -1960., 0.])
            .is_none(),
        "real-height callers must retain floor discrimination"
    );
    let pins = world_quest_ui::map_pins(&catalog, 2256, &runtime, 900);
    assert_eq!(
        pins.len(),
        1,
        "authored lower-floor task must remain visible without invented height"
    );
    assert_eq!(pins[0].quest_id, Some(82521));
    assert_eq!(pins[0].label, "Pawns of Dark Masters");
    assert!((pins[0].x - 0.6442027).abs() < 0.00001);
    assert!((pins[0].y - 0.8322616).abs() < 0.00001);
}

#[test]
fn world_quests_all_authored_maps_survive_partial_snapshot_removal_until_real_expiry() {
    let root = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../data");
    let catalog =
        game_engine_ui_model::ui_map_data::UiMapCatalog::load(&root.join("db2/active")).unwrap();
    let snapshot = received_pin_fixture();
    assert_eq!(snapshot.quests.len(), 239);
    assert_eq!(
        snapshot
            .quests
            .iter()
            .map(|quest| quest.pois.len())
            .sum::<usize>(),
        309
    );
    let mut runtime = QuestRuntime::default();
    runtime.apply_availability(snapshot.clone(), 900);
    for quest in &snapshot.quests {
        for poi in &quest.pois {
            let pins = world_quest_ui::map_pins(&catalog, poi.ui_map_id, &runtime, 900);
            let matches: Vec<_> = pins
                .iter()
                .filter(|pin| pin.quest_id == Some(quest.quest_id))
                .collect();
            assert_eq!(
                matches.len(),
                1,
                "task{} on authored map{}",
                quest.quest_id,
                poi.ui_map_id
            );
            assert_eq!(matches[0].label, quest.title);
            assert!((0.0..=1.0).contains(&matches[0].x) && (0.0..=1.0).contains(&matches[0].y));
        }
    }
    // Bounding rectangles overlap, but a Twilight task does not belong to sibling Wetlands.
    assert!(
        !world_quest_ui::map_pins(&catalog, 56, &runtime, 900)
            .iter()
            .any(|pin| pin.quest_id == Some(86948))
    );
    let mut remaining = snapshot;
    remaining.server_time = 1100;
    remaining.quests.retain(|quest| quest.quest_id != 82521);
    runtime.apply_availability(remaining, 1000);
    assert_eq!(runtime.active_world_quests(1000).count(), 238);
    assert!(
        !world_quest_ui::map_pins(&catalog, 2256, &runtime, 1000)
            .iter()
            .any(|pin| pin.quest_id == Some(82521))
    );
    assert!(
        world_quest_ui::map_pins(&catalog, 241, &runtime, 1000)
            .iter()
            .any(|pin| pin.quest_id == Some(86948))
    );
    assert_eq!(runtime.active_world_quests(87299).count(), 238);
    assert_eq!(runtime.active_world_quests(87300).count(), 0);
    assert!(world_quest_ui::map_pins(&catalog, 241, &runtime, 87300).is_empty());
}

#[test]
fn world_quests_multiple_floor_areas_choose_the_nearest_navigation_point_once() {
    use game_engine_core::minimap_data::MinimapView;
    let root = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../data");
    let catalog =
        game_engine_ui_model::ui_map_data::UiMapCatalog::load(&root.join("db2/active")).unwrap();
    let mut snapshot = received_pin_fixture();
    snapshot.quests.retain(|quest| quest.quest_id == 82521);
    let mut runtime = QuestRuntime::default();
    runtime.apply_availability(snapshot, 900);
    for (map, expected_uv) in [
        (2256, [0.6442027, 0.8322616]),
        (2255, [0.6488530, 0.8337875]),
    ] {
        let pins = world_quest_ui::map_pins(&catalog, map, &runtime, 900);
        assert_eq!(pins.len(), 1);
        assert_eq!(pins[0].quest_id, Some(82521));
        assert!(
            (pins[0].x - expected_uv[0]).abs() < 0.00001
                && (pins[0].y - expected_uv[1]).abs() < 0.00001
        );
    }
    for (center, map) in [([-2268., 1960.], 2256), ([-2275., 1992.], 2255)] {
        let poi = world_quest_ui::nearest_poi(&runtime.world_quests[0], 2601, center).unwrap();
        assert_eq!(poi.ui_map_id, map);
        let view = MinimapView::new(center, 0);
        let blips = world_quest_ui::minimap_blips(&runtime, Some(82521), 2601, &view, 150., 900);
        assert_eq!(blips.len(), 1, "one task marker, not one marker per area");
        assert_eq!(blips[0].unit, 82521);
        assert_eq!(blips[0].offset, [0., 0.]);
    }
    assert!(world_quest_ui::nearest_poi(&runtime.world_quests[0], 0, [0., 0.]).is_none());
}

#[test]
fn world_quests_tooltip_time_uses_full_source_units_at_boundaries() {
    let quest = available().quests.remove(0);
    for (remaining, expected) in [
        (86400, "Time Left: 1 Day"),
        (90000, "Time Left: 1 Day 1 Hour"),
        (174600, "Time Left: 2 Days 30 Minutes"),
        (30, "Time Left: < 1 Minute"),
    ] {
        let tip = world_quest_ui::tooltip(&quest, quest.expires_at - remaining, [0., 0.]);
        assert_eq!(tip.lines[0].left_text, expected);
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
        let blips = world_quest_ui::minimap_blips(&runtime, None, 0, &view, style.map_size, 900);
        assert_eq!(blips.len(), 1, "received task missing in {skin:?}");
        assert_eq!(blips[0].unit, 86948);
        assert_eq!(blips[0].offset, [0., 0.]);
        assert!(runtime.log.is_empty());
        let wrong_map =
            world_quest_ui::minimap_blips(&runtime, None, 1, &view, style.map_size, 900);
        assert!(wrong_map.is_empty());
        let far = MinimapView::new([-5655., 4310.], 0).masked(style.mask);
        assert!(
            world_quest_ui::minimap_blips(&runtime, None, 0, &far, style.map_size, 900).is_empty()
        );
        let selected =
            world_quest_ui::minimap_blips(&runtime, Some(86948), 0, &far, style.map_size, 900);
        assert_eq!(
            selected[0].kind,
            game_engine_ui_model::minimap::BlipKind::QuestSuperTrackArrow
        );
        assert!(
            world_quest_ui::minimap_blips(&runtime, Some(86948), 0, &view, style.map_size, 87300)
                .is_empty()
        );
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
        assert_eq!(tooltip.lines[0].left_text, "Time Left: 23 Hours");
        assert!(tooltip.lines.iter().any(|line| line.left_text == "Rewards"));
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
        assert!(
            registry
                .get_by_name("QuestBlock86948POIButtonNumber")
                .is_none(),
            "World Quests use unnumbered POIs rather than restarting ordinary quest numbering"
        );
        assert!(
            registry
                .get_by_name("QuestBlock86948POIButtonNormal")
                .is_some(),
            "Retail WorldQuest icon retains its normal POI background"
        );
        assert!(
            registry
                .get_by_name("QuestBlock86948POIButtonWorldQuest")
                .is_some()
        );
        let low_time = ObjectiveTrackerState::from_runtime_at(&runtime, false, false, 86700);
        assert_eq!(
            low_time.world_quests[0].0.lines.last().unwrap().text,
            "Time Left: 10 Min"
        );
        assert!(low_time.world_quests[0].1.is_none());
        runtime.log[0].objectives[0].kind = QuestObjectiveKind::ProgressBar;
        let bar_state = ObjectiveTrackerState::from_runtime_at(&runtime, false, false, 900);
        assert_eq!(bar_state.world_quests[0].1, Some((4, 8)));
        assert_eq!(
            bar_state.world_quests[0].0.lines[0].text,
            "Twilight enslavers slain"
        );
        let warning_edge = ObjectiveTrackerState::from_runtime_at(&runtime, false, false, 86401);
        assert_eq!(
            warning_edge.world_quests[0].0.lines.last().unwrap().text,
            "Time Left: 14 Min"
        );
        ctx.insert(bar_state);
        let mut bar_registry = FrameRegistry::new(1280., 720.);
        Screen::new(objective_tracker_screen).sync(&ctx, &mut bar_registry);
        assert_eq!(text(&bar_registry, "WorldQuest86948ProgressLabel"), "50%");
        runtime.log[0].objectives[0].kind = QuestObjectiveKind::Monster;
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
