use super::*;

fn requeue_snapshot(mode: Option<ArenaQueueMode>, phase: BgPhase, accepted: bool) -> PvpSnapshot {
    let mut state = snapshot(phase, accepted);
    if let Some(mode) = mode {
        let bg = state.battleground.as_mut().unwrap();
        bg.name = "Nagrand Arena".into();
        bg.map_id = 559;
        bg.arena = Some(ArenaMatchSnapshot {
            mode,
            preparation_remaining_secs: None,
            shadow_sight_available: false,
            units: Vec::new(),
            rating_changes: Vec::new(),
            shuffle: None,
        });
    }
    state
}

fn mount_requeue(view: &PvpView, skin: ActiveSkin) -> (Screen, SharedContext, FrameRegistry) {
    game_engine_ui_model::paths::set_data_root(
        std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../data"),
    )
    .unwrap();
    set_thread_skin(skin);
    let mut context = SharedContext::new();
    context.insert(skin);
    context.insert(view.clone());
    let mut registry = FrameRegistry::new(1280.0, 720.0);
    let mut screen = Screen::new(pvp_screen);
    screen.sync(&context, &mut registry);
    (screen, context, registry)
}

fn requeue_enabled(registry: &FrameRegistry) -> bool {
    let frame = registry
        .get(
            registry
                .get_by_name("PVPMatchRequeueButton")
                .expect("ended skirmish has a Queue Again button"),
        )
        .unwrap();
    let Some(WidgetData::Button(button)) = &frame.widget_data else {
        panic!("Queue Again button");
    };
    assert_eq!(button.text, "Queue Again");
    // UIPanelButtonTemplate: a disabled button carries no action.
    let action = if button.enabled { "pvp:requeue" } else { "" };
    assert_eq!(frame.onclick.as_deref(), Some(action));
    button.enabled
}

#[test]
#[ignore = "RED: ended skirmish results render Leave only"]
fn skirmish_requeue_both_skin_results_button_and_immediate_disable() {
    for skin in [ActiveSkin::Modern, ActiveSkin::Forever] {
        for mode in [ArenaQueueMode::Skirmish2v2, ArenaQueueMode::Skirmish3v3] {
            let mut view = PvpView::default();
            receive(
                &mut view,
                requeue_snapshot(Some(mode), BgPhase::Ended, true),
            );
            let (mut screen, mut context, mut registry) = mount_requeue(&view, skin);
            assert!(registry.get_by_name("PVPMatchLeaveButton").is_some());
            assert!(requeue_enabled(&registry));
            assert!(view.action("pvp:requeue").is_some());
            assert!(
                view.pending,
                "disable immediately, before any server response"
            );
            context.insert(view.clone());
            screen.sync(&context, &mut registry);
            assert!(!requeue_enabled(&registry));
            assert!(
                view.action("pvp:requeue").is_none(),
                "one pending request only"
            );
        }
    }
    set_thread_skin(ActiveSkin::Modern);
}

#[test]
#[ignore = "RED: ended skirmish Queue Again has no action route"]
fn skirmish_requeue_action_binds_ended_match_and_blocks_duplicates() {
    for mode in [ArenaQueueMode::Skirmish2v2, ArenaQueueMode::Skirmish3v3] {
        let mut view = PvpView::default();
        receive(
            &mut view,
            requeue_snapshot(Some(mode), BgPhase::Ended, true),
        );
        assert_eq!(
            view.action("pvp:requeue"),
            Some(PvpAction::RequeueSkirmish(71)),
            "ended skirmish dispatches its match identity, not a client-selected mode"
        );
        assert!(view.pending);
        assert!(view.action("pvp:requeue").is_none());
        let mut returned = requeue_snapshot(None, BgPhase::Ended, true);
        returned.battleground = None;
        view.receive(PvpStateUpdate {
            snapshot: Some(returned),
            message: None,
            error: Some("player level unavailable".into()),
        });
        assert!(!view.pending);
        assert_eq!(view.error, "player level unavailable");
        assert!(view.battleground().is_none());
        assert!(
            view.action("pvp:requeue").is_none(),
            "no result match at the returned origin"
        );
    }
}

#[test]
fn skirmish_requeue_hides_and_refuses_bg_rated_shuffle_active_unaccepted() {
    for skin in [ActiveSkin::Modern, ActiveSkin::Forever] {
        for (mode, phase, accepted) in [
            (None, BgPhase::Ended, true),
            (Some(ArenaQueueMode::Rated2v2), BgPhase::Ended, true),
            (Some(ArenaQueueMode::Rated3v3), BgPhase::Ended, true),
            (Some(ArenaQueueMode::SoloShuffle), BgPhase::Ended, true),
            (Some(ArenaQueueMode::Skirmish2v2), BgPhase::Waiting, true),
            (Some(ArenaQueueMode::Skirmish3v3), BgPhase::InProgress, true),
            (Some(ArenaQueueMode::Skirmish2v2), BgPhase::Ended, false),
        ] {
            let mut view = PvpView::default();
            receive(&mut view, requeue_snapshot(mode, phase, accepted));
            let (_, _, registry) = mount_requeue(&view, skin);
            assert!(registry.get_by_name("PVPMatchRequeueButton").is_none());
            assert!(view.action("pvp:requeue").is_none());
            assert!(!view.pending);
        }
    }
    set_thread_skin(ActiveSkin::Modern);
}
