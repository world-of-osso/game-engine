use game_engine_ui_model::{
    encounter_timeline::EncounterTimeline,
    encounter_timeline_component::{EncounterTimelineScreen, encounter_timeline_screen},
};
use shared::protocol::{EncounterSeverity, EncounterTimelineEvent};
use ui_toolkit::{
    registry::FrameRegistry,
    screen::{Screen, SharedContext},
};

#[test]
fn timeline_native_warning_regions_are_centered_in_the_viewport() {
    let mut timeline = EncounterTimeline::default();
    timeline.receive(
        EncounterTimelineEvent::Cast {
            caster: 3669,
            spell_id: 20805,
            severity: EncounterSeverity::Low,
        },
        |_| ("Lightning Bolt".into(), 136048),
    );
    let mut shared = SharedContext::new();
    shared.insert(EncounterTimelineScreen {
        timeline,
        enabled: true,
        bars: true,
        warnings: true,
        preview: false,
    });
    let mut registry = FrameRegistry::new(1280.0, 720.0);
    Screen::new(encounter_timeline_screen).sync(&shared, &mut registry);
    let bounds =
        super::layout::compute_layout_with_intrinsics(&registry, &Default::default()).unwrap();
    let warning = &bounds[&registry.get_by_name("MinorEncounterWarnings").unwrap()];
    assert_eq!(
        [warning.x, warning.y, warning.width, warning.height],
        [390.0, 130.0, 500.0, 36.0]
    );
}
