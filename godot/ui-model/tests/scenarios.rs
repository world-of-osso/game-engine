use game_engine_ui_model::objective_tracker_component::{
    ObjectiveTrackerState, objective_tracker_screen,
};
use game_engine_ui_model::scenario_progress::ScenarioObjectives;
use shared::protocol::*;
use ui_toolkit::atlas::{ActiveSkin, set_thread_skin};
use ui_toolkit::frame::WidgetData;
use ui_toolkit::registry::FrameRegistry;
use ui_toolkit::screen::{Screen, SharedContext};

fn progress(completed: bool) -> ScenarioProgress {
    ScenarioProgress {
        map_id: 90,
        instance_id: 17,
        difficulty_id: 1,
        scenario: Some(ScenarioStage {
            scenario_id: 649,
            name: "Gnomeregan".into(),
            kind: 3,
            flags: 10,
            step_id: if completed { 0 } else { 1176 },
            stage: if completed { 2 } else { 1 },
            num_stages: 1,
            title: "Gnomeregan".into(),
            description: "Defeat Mekgineer Thermaplugg.".into(),
            completed,
            criteria: if completed {
                vec![]
            } else {
                vec![AchievementCriterionLine {
                    tree_id: 36987,
                    order_index: 0,
                    criteria_id: 18529,
                    description: "Mekgineer Thermaplugg defeated".into(),
                    flags: 0,
                    current: 0,
                    required: 1,
                    completed: false,
                    progress_supported: true,
                }]
            },
        }),
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
fn render(objectives: &ScenarioObjectives, skin: ActiveSkin) -> FrameRegistry {
    let mut ctx = SharedContext::new();
    ctx.insert(skin);
    ctx.insert(ObjectiveTrackerState {
        scenario: objectives.block.clone(),
        ..Default::default()
    });
    let mut registry = FrameRegistry::new(1920., 1080.);
    Screen::new(objective_tracker_screen).sync(&ctx, &mut registry);
    registry
}

#[test]
fn scenarios_gnomeregan_stage_and_completion_same_elements_both_skins() {
    game_engine_ui_model::paths::set_data_root(
        std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../data"),
    )
    .unwrap();
    for skin in [ActiveSkin::Modern, ActiveSkin::Forever] {
        set_thread_skin(skin);
        let mut objectives = ScenarioObjectives::default();
        objectives.begin_map(90);
        assert!(objectives.apply(progress(false)));
        let registry = render(&objectives, skin);
        assert_eq!(
            text(&registry, "ScenarioObjectiveTrackerHeaderText"),
            "Dungeon"
        );
        assert_eq!(
            text(&registry, "ScenarioObjectiveTrackerStage"),
            "Gnomeregan"
        );
        assert_eq!(
            text(&registry, "ScenarioCriteria0Text"),
            "Mekgineer Thermaplugg defeated"
        );
        assert!(objectives.apply(progress(true)));
        let registry = render(&objectives, skin);
        assert_eq!(
            text(&registry, "ScenarioObjectiveTrackerCompleteLabel"),
            "Dungeon Complete!"
        );
        assert!(registry.get_by_name("ScenarioCriteria0Text").is_none());
        let mut stale = progress(false);
        stale.instance_id = 18;
        assert!(!objectives.apply(stale));
        objectives.begin_map(0);
        assert!(!objectives.apply(progress(false)));
        assert!(objectives.apply(ScenarioProgress {
            map_id: 0,
            instance_id: 0,
            difficulty_id: 0,
            scenario: None
        }));
        assert!(objectives.block.is_none());
    }
}
