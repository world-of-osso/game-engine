use game_engine_ui_model::battleground::{PvpAction, PvpView, pvp_screen};
use shared::battleground::{BgOutcome, BgPhase, BgTeam, FlagObjective};
use shared::protocol::*;
use ui_toolkit::{
    atlas::{ActiveSkin, set_thread_skin},
    frame::WidgetData,
    registry::FrameRegistry,
    screen::{Screen, SharedContext},
};
fn snapshot(phase: BgPhase, accepted: bool) -> PvpSnapshot {
    PvpSnapshot {
        honor: 0,
        honor_max: 0,
        conquest: 0,
        conquest_max: 0,
        brackets: vec![],
        queue: None,
        battleground: Some(BattlegroundSnapshot {
            match_id: 71,
            battleground_id: 1014,
            name: "Warsong Gulch".into(),
            map_id: 2106,
            team: BgTeam::A,
            phase,
            accepted,
            invitation_time_left_secs: None,
            score_a: 2,
            score_b: 1,
            flags: FlagObjective::default(),
            outcome: (phase == BgPhase::Ended).then_some(BgOutcome::Winner(BgTeam::A)),
            players: vec![BattlegroundPlayerScoreSnapshot {
                character_id: 81,
                name: "Flagrunner".into(),
                class: 1,
                level: 90,
                team: BgTeam::A,
                killing_blows: 0,
                deaths: 0,
                damage_done: 0,
                healing_done: 0,
                flag_captures: 2,
                flag_returns: 1,
                honorable_kills: None,
                honor_level: None,
            }],
            honor_reward: None,
        }),
    }
}
fn receive(view: &mut PvpView, state: PvpSnapshot) {
    view.receive(PvpStateUpdate {
        snapshot: Some(state),
        message: None,
        error: None,
    });
}
#[test]
fn battleground_requests_wait_for_authority_and_echo_match_identity() {
    let mut view = PvpView::default();
    assert_eq!(view.action("pvp:queue"), Some(PvpAction::Queue(1014)));
    assert_eq!(view.action("pvp:queue"), None);
    receive(&mut view, snapshot(BgPhase::Waiting, false));
    assert_eq!(
        view.action("pvp:accept"),
        Some(PvpAction::Respond {
            match_id: 71,
            accepted: true
        })
    );
    assert_eq!(view.action("pvp:decline"), None);
    receive(&mut view, snapshot(BgPhase::Waiting, true));
    assert_eq!(view.action("pvp:accept"), None);
    receive(&mut view, snapshot(BgPhase::InProgress, true));
    assert_eq!(view.action("pvp:leave"), Some(PvpAction::Leave(71)));
    view.receive(PvpStateUpdate {
        snapshot: None,
        message: None,
        error: Some("Denied".into()),
    });
    assert_eq!(view.error, "Denied");
    assert_eq!(view.action("pvp:leave"), Some(PvpAction::Leave(71)));
}
#[test]
fn battleground_both_skins_render_invite_hud_score_rows_and_end_without_fake_honor() {
    game_engine_ui_model::paths::set_data_root(
        std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../data"),
    )
    .unwrap();
    for skin in [ActiveSkin::Modern, ActiveSkin::Forever] {
        set_thread_skin(skin);
        let mut view = PvpView::default();
        let mut context = SharedContext::new();
        context.insert(skin);
        let mut registry = FrameRegistry::new(1280.0, 720.0);
        let mut screen = Screen::new(pvp_screen);
        receive(&mut view, snapshot(BgPhase::Waiting, false));
        context.insert(view.clone());
        screen.sync(&context, &mut registry);
        assert!(
            registry
                .get_by_name("PVPReadyDialogEnterBattleButton")
                .is_some()
        );
        assert!(
            registry
                .get_by_name("PVPReadyDialogLeaveQueueButton")
                .is_some()
        );
        assert!(registry.get_by_name("PVPUIFrame").is_none());
        receive(&mut view, snapshot(BgPhase::InProgress, true));
        view.scoreboard = true;
        context.insert(view.clone());
        screen.sync(&context, &mut registry);
        assert!(registry.get_by_name("PVPReadyDialog").is_none());
        assert!(registry.get_by_name("WorldStateAlwaysUpFrame").is_some());
        let row = registry.get_by_name("PVPMatchPlayer81").unwrap();
        let WidgetData::FontString(label) =
            registry.get(row).unwrap().widget_data.as_ref().unwrap()
        else {
            panic!("score text missing")
        };
        assert!(label.text.contains("Flagrunner"));
        assert!(label.text.contains('2'));
        receive(&mut view, snapshot(BgPhase::Ended, true));
        context.insert(view);
        screen.sync(&context, &mut registry);
        assert!(registry.get_by_name("PVPMatchResults").is_some());
        assert!(registry.get_by_name("PVPMatchLeaveButton").is_some());
        assert!(registry.get_by_name("PVPMatchHonorReward").is_none());
    }
    set_thread_skin(ActiveSkin::Modern);
}
