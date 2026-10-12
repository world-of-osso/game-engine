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
        conquest_max: None,
        brackets: vec![],
        queue: None,
        war_mode: None,
        talents: None,
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
            world_states: Vec::new(),
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
                objective_scores: vec![(929, 1), (928, 2)],
                honorable_kills: None,
                honor_level: None,
            }],
            honor_reward: None,
        }),
    }
}
#[test]
fn node_battleground_both_skin_hud_columns_and_results() {
    game_engine_ui_model::paths::set_data_root(
        std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../data"),
    )
    .unwrap();
    for skin in [ActiveSkin::Modern, ActiveSkin::Forever] {
        set_thread_skin(skin);
        for template in [
            shared::battleground::arathi_basin(),
            shared::battleground::battle_for_gilneas(),
        ] {
            let rules = shared::battleground::node_rules(template.map_id).unwrap();
            let mut state = snapshot(BgPhase::InProgress, true);
            let bg = state.battleground.as_mut().unwrap();
            bg.battleground_id = template.id;
            bg.map_id = template.map_id;
            bg.name = template.name;
            bg.score_a = 1200;
            bg.world_states = vec![(rules.nodes[0].poi, 5)];
            bg.players[0].objective_scores = vec![(rules.assault_stat, 5), (rules.defense_stat, 1)];
            let mut view = PvpView {
                scoreboard: true,
                ..Default::default()
            };
            receive(&mut view, state.clone());
            view.scoreboard = true;
            let mut context = SharedContext::new();
            context.insert(skin);
            context.insert(view.clone());
            let mut registry = FrameRegistry::new(1280.0, 720.0);
            let mut screen = Screen::new(pvp_screen);
            screen.sync(&context, &mut registry);
            assert_text(
                &registry,
                "WorldStateScore",
                "Alliance 1200 / 1500    Horde 1 / 1500",
            );
            assert_text(
                &registry,
                &format!("WorldStateNode{}", rules.nodes[0].entry),
                &format!("{}\nAlliance", rules.nodes[0].name),
            );
            assert_text(&registry, "PVPMatchCategory7", "Bases Assaulted");
            assert_text(&registry, "PVPMatchCategory8", "Bases Defended");
            assert_text(
                &registry,
                &format!("PVPMatchPlayer81Stat{}", rules.assault_stat),
                "5",
            );
            assert_text(
                &registry,
                &format!("PVPMatchPlayer81Stat{}", rules.defense_stat),
                "1",
            );
            view.action("pvp:score-close");
            let bg = state.battleground.as_mut().unwrap();
            bg.phase = BgPhase::Ended;
            bg.outcome = Some(BgOutcome::Winner(BgTeam::A));
            receive(&mut view, state);
            context.insert(view);
            screen.sync(&context, &mut registry);
            assert_text(&registry, "PVPMatchOutcome", "Victory");
        }
    }
    set_thread_skin(ActiveSkin::Modern);
}

#[test]
fn eye_of_the_storm_both_skin_tower_flag_hud_source_column_and_results() {
    game_engine_ui_model::paths::set_data_root(
        std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../data"),
    )
    .unwrap();
    let template = shared::battleground::eye_of_the_storm();
    for skin in [ActiveSkin::Modern, ActiveSkin::Forever] {
        set_thread_skin(skin);
        let mut state = snapshot(BgPhase::InProgress, true);
        let bg = state.battleground.as_mut().unwrap();
        bg.battleground_id = template.id;
        bg.map_id = template.map_id;
        bg.name = template.name.clone();
        bg.score_a = 75;
        // Fel Reaver Alliance 2726, Blood Elf Horde 2724, Draenei uncontrolled 2731,
        // Netherstorm flag carried (8863 = FlagState::Taken).
        bg.world_states = vec![(2726, 1), (2724, 1), (2731, 1), (8863, 2)];
        bg.players[0].objective_scores = vec![(183, 1)];
        let mut view = PvpView::default();
        receive(&mut view, state.clone());
        view.scoreboard = true;
        let mut context = SharedContext::new();
        context.insert(skin);
        context.insert(view.clone());
        let mut registry = FrameRegistry::new(1280.0, 720.0);
        let mut screen = Screen::new(pvp_screen);
        screen.sync(&context, &mut registry);
        assert_text(
            &registry,
            "WorldStateScore",
            "Alliance 75 / 1500    Horde 1 / 1500",
        );
        assert_text(
            &registry,
            "WorldStateNode184081",
            "Fel Reaver Ruins\nAlliance",
        );
        assert_text(&registry, "WorldStateNode184080", "Blood Elf Tower\nHorde");
        assert_text(
            &registry,
            "WorldStateNode184083",
            "Draenei Ruins\nUncontrolled",
        );
        assert_text(&registry, "WorldStateNode184082", "Mage Tower\n—");
        assert_text(&registry, "WorldStateEyeFlag", "Netherstorm Flag\nCarried");
        assert_text(&registry, "PVPMatchCategory7", "Flag Captures");
        assert!(registry.get_by_name("PVPMatchCategory8").is_none());
        assert_text(&registry, "PVPMatchPlayer81Stat183", "1");
        view.action("pvp:score-close");
        let bg = state.battleground.as_mut().unwrap();
        bg.phase = BgPhase::Ended;
        bg.outcome = Some(BgOutcome::Winner(BgTeam::A));
        receive(&mut view, state);
        context.insert(view);
        screen.sync(&context, &mut registry);
        assert_text(&registry, "PVPMatchOutcome", "Victory");
    }
    set_thread_skin(ActiveSkin::Modern);
}

#[test]
fn kotmogu_both_skin_orb_hud_source_columns_queue_and_results() {
    game_engine_ui_model::paths::set_data_root(
        std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../data"),
    )
    .unwrap();
    for skin in [ActiveSkin::Modern, ActiveSkin::Forever] {
        set_thread_skin(skin);
        let template = shared::battleground::temple_of_kotmogu();
        let mut state = snapshot(BgPhase::InProgress, true);
        let bg = state.battleground.as_mut().unwrap();
        bg.battleground_id = template.id;
        bg.map_id = template.map_id;
        bg.name = template.name.clone();
        bg.score_a = 42;
        bg.world_states = vec![(6964, 1), (6969, 1), (6716, 1), (6715, 1)];
        bg.players[0].objective_scores = vec![(418, 1), (419, 42)];
        let mut view = PvpView::default();
        view.action("pvp:select:699");
        assert_eq!(view.action("pvp:queue"), Some(PvpAction::Queue(699)));
        receive(&mut view, state.clone());
        view.scoreboard = true;
        let mut context = SharedContext::new();
        context.insert(skin);
        context.insert(view.clone());
        let mut registry = FrameRegistry::new(1280.0, 720.0);
        let mut screen = Screen::new(pvp_screen);
        screen.sync(&context, &mut registry);
        assert_text(
            &registry,
            "WorldStateScore",
            "Alliance 42 / 1500    Horde 1 / 1500",
        );
        assert_text(&registry, "WorldStateOrb212094", "Orange Orb\nAlliance");
        assert_text(&registry, "WorldStateOrb212091", "Blue Orb\nHorde");
        assert_text(&registry, "WorldStateOrb212093", "Green Orb\nAt base");
        assert_text(&registry, "WorldStateOrb212092", "Purple Orb\nAt base");
        assert_text(&registry, "PVPMatchCategory7", "Orb Possessions");
        assert_text(&registry, "PVPMatchCategory8", "Victory Points");
        assert_text(&registry, "PVPMatchPlayer81Stat418", "1");
        assert_text(&registry, "PVPMatchPlayer81Stat419", "42");
        view.action("pvp:score-close");
        let bg = state.battleground.as_mut().unwrap();
        bg.phase = BgPhase::Ended;
        bg.outcome = Some(BgOutcome::Winner(BgTeam::A));
        receive(&mut view, state);
        context.insert(view);
        screen.sync(&context, &mut registry);
        assert_text(&registry, "PVPMatchOutcome", "Victory");
        assert!(registry.get_by_name("PVPMatchHonorReward").is_none());
    }
    set_thread_skin(ActiveSkin::Modern);
}

#[test]
fn silvershard_both_skin_hidden_queue_retains_cart_hud_source_column_and_results() {
    game_engine_ui_model::paths::set_data_root(
        std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../data"),
    )
    .unwrap();
    for skin in [ActiveSkin::Modern, ActiveSkin::Forever] {
        set_thread_skin(skin);
        let template = shared::battleground::silvershard_mines();
        let mut state = snapshot(BgPhase::InProgress, true);
        let bg = state.battleground.as_mut().unwrap();
        bg.battleground_id = template.id;
        bg.map_id = template.map_id;
        bg.name = template.name;
        bg.score_a = 42;
        bg.world_states = vec![(6881, 1), (6879, 1), (6439, 0), (6440, 0)];
        bg.players[0].objective_scores = vec![(428, 3)];
        let mut view = PvpView::default();
        view.visible = true;
        view.action("pvp:select:708");
        assert_ne!(
            view.selected_battleground, 708,
            "unpublished client assets keep Silvershard hidden"
        );
        assert_eq!(view.action("pvp:queue"), Some(PvpAction::Queue(1014)));
        receive(&mut view, state.clone());
        view.scoreboard = true;
        let mut context = SharedContext::new();
        context.insert(skin);
        context.insert(view.clone());
        let mut registry = FrameRegistry::new(1280.0, 720.0);
        let mut screen = Screen::new(pvp_screen);
        screen.sync(&context, &mut registry);
        assert!(
            registry
                .get_by_name("HonorFrameSpecificBattlegroundButton708")
                .is_none()
        );
        assert_text(
            &registry,
            "WorldStateScore",
            "Alliance 42 / 1500    Horde 1 / 1500",
        );
        assert_text(&registry, "WorldStateCart0", "South Cart\nAlliance");
        assert_text(&registry, "WorldStateCart1", "North Cart\nHorde");
        assert_text(&registry, "WorldStateCart2", "East Cart\nNeutral");
        assert_text(&registry, "PVPMatchCategory7", "Carts Controlled");
        assert_text(&registry, "PVPMatchPlayer81Stat428", "3");
        assert!(registry.get_by_name("WorldStateFlags").is_none());
        view.action("pvp:score-close");
        let bg = state.battleground.as_mut().unwrap();
        bg.phase = BgPhase::Ended;
        bg.outcome = Some(BgOutcome::Winner(BgTeam::A));
        receive(&mut view, state);
        context.insert(view);
        screen.sync(&context, &mut registry);
        assert_text(&registry, "PVPMatchOutcome", "Victory");
    }
    set_thread_skin(ActiveSkin::Modern);
}

fn assert_text(registry: &FrameRegistry, name: &str, expected: &str) {
    let handle = registry
        .get_by_name(name)
        .unwrap_or_else(|| panic!("{name}: missing font string"));
    let frame = registry.get(handle).unwrap();
    let WidgetData::FontString(label) = frame.widget_data.as_ref().unwrap() else {
        panic!("{name}: expected a native font string")
    };
    assert_eq!(label.text, expected);
}

fn assert_size(registry: &FrameRegistry, name: &str, width: f32, height: f32) {
    let frame = registry.get(registry.get_by_name(name).unwrap()).unwrap();
    assert_eq!(frame.width, ui_toolkit::frame::Dimension::Fixed(width));
    assert_eq!(frame.height, ui_toolkit::frame::Dimension::Fixed(height));
}
fn receive(view: &mut PvpView, state: PvpSnapshot) {
    view.receive(PvpStateUpdate {
        snapshot: Some(state),
        message: None,
        error: None,
    });
}
#[test]
fn node_battleground_selection_preserves_queue_identity() {
    for id in [1018, 120, 108, 7] {
        let mut view = PvpView::default();
        assert_eq!(view.action(&format!("pvp:select:{id}")), None);
        assert_eq!(view.action("pvp:queue"), Some(PvpAction::Queue(id)));
        assert_eq!(view.action("pvp:queue"), None);
    }
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
    assert_eq!(view.action("pvp:leave"), Some(PvpAction::Leave(71)));
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
        assert_size(&registry, "PVPReadyDialog", 306.0, 210.0);
        receive(&mut view, snapshot(BgPhase::InProgress, true));
        view.scoreboard = true;
        context.insert(view.clone());
        screen.sync(&context, &mut registry);
        assert!(registry.get_by_name("PVPReadyDialog").is_none());
        assert!(registry.get_by_name("WorldStateAlwaysUpFrame").is_some());
        assert_size(&registry, "PVPMatchScoreboard", 1024.0, 420.0);
        let row = registry.get_by_name("PVPMatchPlayer81").unwrap();
        let WidgetData::FontString(label) =
            registry.get(row).unwrap().widget_data.as_ref().unwrap()
        else {
            panic!("score text missing")
        };
        assert!(label.text.contains("Flagrunner"));
        // PvpScoreboardLayout orders WSG Flag Returns (0) before Flag Captures (1).
        assert_text(&registry, "PVPMatchCategory7", "Flag Returns");
        assert_text(&registry, "PVPMatchCategory8", "Flag Captures");
        assert_text(&registry, "PVPMatchPlayer81Stat928", "2");
        assert_text(&registry, "PVPMatchPlayer81Stat929", "1");
        view.action("pvp:horde");
        context.insert(view.clone());
        screen.sync(&context, &mut registry);
        assert!(registry.get_by_name("PVPMatchPlayer81").is_none());
        view.action("pvp:all");
        view.action("pvp:score-close");
        receive(&mut view, snapshot(BgPhase::Ended, true));
        context.insert(view);
        screen.sync(&context, &mut registry);
        assert!(registry.get_by_name("PVPMatchResults").is_some());
        assert_size(&registry, "PVPMatchResults", 1024.0, 620.0);
        assert!(registry.get_by_name("PVPMatchLeaveButton").is_some());
        assert!(registry.get_by_name("PVPMatchHonorReward").is_none());
        let mut losing_view = context.get::<PvpView>().unwrap().clone();
        let mut losing_snapshot = snapshot(BgPhase::Ended, true);
        losing_snapshot.battleground.as_mut().unwrap().team = BgTeam::B;
        receive(&mut losing_view, losing_snapshot);
        context.insert(losing_view);
        screen.sync(&context, &mut registry);
        let outcome = registry
            .get(registry.get_by_name("PVPMatchOutcome").unwrap())
            .unwrap();
        let WidgetData::FontString(label) = outcome.widget_data.as_ref().unwrap() else {
            panic!("outcome text missing")
        };
        assert_eq!(label.text, "Defeat");
        context.insert(PvpView {
            visible: true,
            ..Default::default()
        });
        screen.sync(&context, &mut registry);
        assert_size(&registry, "PVPUIFrame", 563.0, 428.0);
    }
    set_thread_skin(ActiveSkin::Modern);
}

#[test]
fn twin_peaks_both_skin_flag_hud_source_columns_and_results() {
    game_engine_ui_model::paths::set_data_root(
        std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../data"),
    )
    .unwrap();
    for skin in [ActiveSkin::Modern, ActiveSkin::Forever] {
        set_thread_skin(skin);
        let template = shared::battleground::twin_peaks();
        let mut state = snapshot(BgPhase::InProgress, true);
        let bg = state.battleground.as_mut().unwrap();
        bg.battleground_id = template.id;
        bg.map_id = template.map_id;
        bg.name = template.name;
        bg.flags.flag_b = shared::battleground::FlagState::Carried { carrier: 81 };
        bg.players[0].objective_scores = vec![(290, 2), (291, 1)];
        let mut view = PvpView::default();
        receive(&mut view, state.clone());
        view.scoreboard = true;
        let mut context = SharedContext::new();
        context.insert(skin);
        context.insert(view.clone());
        let mut registry = FrameRegistry::new(1280.0, 720.0);
        let mut screen = Screen::new(pvp_screen);
        screen.sync(&context, &mut registry);
        assert_text(
            &registry,
            "WorldStateScore",
            "Alliance 2 / 3    Horde 1 / 3",
        );
        assert_text(
            &registry,
            "WorldStateFlags",
            "Alliance flag: At base    Horde flag: Carried",
        );
        assert_text(&registry, "PVPMatchCategory7", "Flag Captures");
        assert_text(&registry, "PVPMatchCategory8", "Flag Returns");
        assert_text(&registry, "PVPMatchPlayer81Stat290", "2");
        assert_text(&registry, "PVPMatchPlayer81Stat291", "1");
        view.action("pvp:score-close");
        let bg = state.battleground.as_mut().unwrap();
        bg.phase = BgPhase::Ended;
        bg.score_a = 3;
        bg.outcome = Some(BgOutcome::Winner(BgTeam::A));
        receive(&mut view, state);
        context.insert(view);
        screen.sync(&context, &mut registry);
        assert_text(&registry, "PVPMatchOutcome", "Victory");
        assert!(registry.get_by_name("PVPMatchHonorReward").is_none());
    }
    set_thread_skin(ActiveSkin::Modern);
}
