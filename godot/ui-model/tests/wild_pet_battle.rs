use game_engine_ui_model::wild_pet_battle::{WildBattleView, wild_battle_screen};
use shared::protocol::*;
use ui_toolkit::{
    atlas::{ActiveSkin, set_thread_skin},
    frame::Dimension,
    registry::FrameRegistry,
    screen::{Screen, SharedContext},
};

fn snapshot() -> WildPetBattleSnapshot {
    let pet = BattlePetSnapshot {
        instance_id: Some(41),
        species_id: 39,
        name: "Mechanical Squirrel".into(),
        display_id: 7937,
        icon: 132145,
        quality: 1,
        xp: 25,
        next_level_xp: 100,
        family: 9,
        level: 1,
        health: 159,
        max_health: 159,
        power: 10,
        speed: 11,
        abilities: std::array::from_fn(|index| BattleAbilitySnapshot {
            id: if index == 0 { 119 } else { 0 },
            name: if index == 0 {
                "Scratch".into()
            } else {
                "Locked".into()
            },
            icon: 132139,
            family: 7,
            cooldown: if index == 0 { 2 } else { 0 },
            usable: false,
        }),
        auras: vec![BattleAuraSnapshot {
            ability_id: 194,
            icon: 132139,
            rounds_remaining: 2,
        }],
    };
    let mut enemy = pet.clone();
    enemy.instance_id = None;
    enemy.species_id = 378;
    enemy.name = "Rabbit".into();
    enemy.display_id = 328;
    enemy.family = 4;
    enemy.health = 30;
    WildPetBattleSnapshot {
        battle_id: 100,
        round: 3,
        teams: [vec![pet.clone(), pet.clone(), pet], vec![enemy]],
        active: [0, 0],
        wild_creature: 99,
        can_trap: true,
        turn_time_ms: 0,
        initial_selection_required: false,
        replacement_required: false,
        feedback: vec![],
    }
}
#[test]
fn petbattle_pvp_initial_selection_allows_current_pet_but_not_combat_actions() {
    let mut view = WildBattleView::default();
    let mut state = snapshot();
    state.wild_creature = 0;
    state.can_trap = false;
    state.initial_selection_required = true;
    state.teams[0][0].abilities[0].usable = true;
    state.teams[0][0].abilities[0].cooldown = 0;
    view.receive(WildPetBattleUpdate::Start(state.clone()));
    assert!(view.action("pb:ability:1").is_none());
    assert!(view.action("pb:pass").is_none());
    let request = view.action("pb:swap:1").expect("current front pet can be selected");
    assert_eq!(request.action, WildPetBattleAction::Swap(1));
    assert_eq!(request.round, state.round);
    assert!(view.action("pb:swap:2").is_none());
    view.receive(WildPetBattleUpdate::Rejected("initial pet already selected".into()));
    view.receive(WildPetBattleUpdate::State(WildPetBattleSnapshot { initial_selection_required: false, ..state }));
    assert!(view.action("pb:ability:1").is_some());
}

#[test]
fn wild_pet_battle_actions_use_authoritative_round_and_block_cooldowns_pending_and_fainted_swaps() {
    let mut view = WildBattleView::default();
    view.receive(WildPetBattleUpdate::Start(snapshot()));
    assert!(view.action("pb:ability:1").is_none());
    let request = view.action("pb:trap").unwrap();
    assert_eq!(
        request,
        WildPetBattleActionRequest {
            battle_id: 100,
            round: 3,
            action: WildPetBattleAction::Trap
        }
    );
    assert!(view.action("pb:pass").is_none());
    let mut state = snapshot();
    state.round = 4;
    state.teams[0][1].health = 0;
    view.receive(WildPetBattleUpdate::Round {
        state,
        combat_text: vec!["Trap failed".into()],
    });
    assert!(view.action("pb:swap:2").is_none());
    assert_eq!(
        view.action("pb:swap:3").unwrap().action,
        WildPetBattleAction::Swap(3)
    );
}
#[test]
fn wild_pet_battle_skins_render_same_frames_auras_abilities_and_untimed_pve_timer() {
    game_engine_ui_model::paths::set_data_root(
        std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../data"),
    )
    .unwrap();
    for skin in [ActiveSkin::Modern, ActiveSkin::Forever] {
        set_thread_skin(skin);
        let mut view = WildBattleView::default();
        view.receive(WildPetBattleUpdate::Start(snapshot()));
        let mut context = SharedContext::new();
        context.insert(skin);
        context.insert(view);
        let mut registry = FrameRegistry::new(1280.0, 720.0);
        Screen::new(wild_battle_screen).sync(&context, &mut registry);
        for name in ["PetBattleActiveAlly", "PetBattleActiveEnemy"] {
            let frame = registry.get(registry.get_by_name(name).unwrap()).unwrap();
            assert_eq!(frame.width, Dimension::Fixed(270.0));
            assert_eq!(frame.height, Dimension::Fixed(80.0));
        }
        assert!(registry.get_by_name("PetBattleTurnTimerText").is_none());
        for name in [
            "PetBattleAllyType",
            "PetBattleEnemyType",
            "PetBattleAllyAura0",
            "PetBattleEnemyAura0",
            "PetBattleAbility1",
            "PetBattleAbility2",
            "PetBattleAbility3",
            "PetBattleSwap",
            "PetBattleTrap",
            "PetBattlePass",
            "PetBattleForfeit",
        ] {
            assert!(registry.get_by_name(name).is_some(), "{name}");
        }
    }
}
#[test]
fn wild_pet_battle_retail_hud_has_icons_locks_reserves_and_no_debug_output() {
    game_engine_ui_model::paths::set_data_root(
        std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../data"),
    )
    .unwrap();
    for skin in [ActiveSkin::Modern, ActiveSkin::Forever] {
        set_thread_skin(skin);
        let mut view = WildBattleView::default();
        view.receive(WildPetBattleUpdate::Round {
            state: snapshot(),
            combat_text: vec![
                "Used { pet: PetRef { team: 1, slot: 0 }, ability: 119, turn: 1 }".into(),
                "Order([0, 1])".into(),
                "29 damage".into(),
            ],
        });
        let mut context = SharedContext::new();
        context.insert(skin);
        context.insert(view);
        let mut registry = FrameRegistry::new(1280.0, 720.0);
        Screen::new(wild_battle_screen).sync(&context, &mut registry);
        assert!(registry.get_by_name("PetBattleCombatText").is_none());
        assert!(registry.get_by_name("PetBattleAbility1Label").is_none());
        for name in [
            "PetBattleAbility1Icon",
            "PetBattleAbility1Effectiveness",
            "PetBattleAbility2Lock",
            "PetBattleAllyIcon",
            "PetBattleEnemyIcon",
            "PetBattleAllyReserve1",
            "PetBattleAllyReserve2",
            "PetBattleXPBar",
        ] {
            assert!(registry.get_by_name(name).is_some(), "{skin:?}: {name}");
        }
    }
}
#[test]
fn wild_pet_battle_feedback_preserves_event_amounts_and_badges_follow_enemy_type() {
    game_engine_ui_model::paths::set_data_root(
        std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../data"),
    )
    .unwrap();
    for (family, badge) in [(4, Some(608706)), (2, Some(608707)), (0, None)] {
        let mut state = snapshot();
        state.teams[1][0].family = family;
        state.feedback = vec![BattleCombatFeedback {
            team: 1,
            slot: 0,
            text: "29".into(),
            healing: false,
        }];
        let mut view = WildBattleView::default();
        view.receive(WildPetBattleUpdate::Round {
            state,
            combat_text: vec!["29 damage".into()],
        });
        let mut context = SharedContext::new();
        context.insert(ActiveSkin::Modern);
        context.insert(view);
        let mut registry = FrameRegistry::new(1280.0, 720.0);
        Screen::new(wild_battle_screen).sync(&context, &mut registry);
        let floating = registry
            .get(registry.get_by_name("PetBattleFloating0").unwrap())
            .unwrap();
        assert!(
            matches!(floating.widget_data.as_ref(), Some(ui_toolkit::frame::WidgetData::FontString(text)) if text.text == "29")
        );
        let indicator = registry.get_by_name("PetBattleAbility1Effectiveness");
        assert_eq!(indicator.is_some(), badge.is_some());
        if let Some(id) = indicator {
            let data = registry.get(id).unwrap().widget_data.as_ref().unwrap();
            assert!(
                matches!(data, ui_toolkit::frame::WidgetData::Texture(texture) if texture.source == ui_toolkit::widgets::texture::TextureSource::FileDataId(badge.unwrap()))
            );
        }
    }
}
#[test]
fn wild_pet_battle_end_updates_the_active_pet_xp_bar_after_level_up() {
    let mut view = WildBattleView::default();
    view.receive(WildPetBattleUpdate::Start(snapshot()));
    view.receive(WildPetBattleUpdate::End {
        battle_id: 100,
        outcome: WildPetBattleOutcome::Won,
        captured_pet_id: None,
        combat_text: vec![],
        rewards: vec![BattlePetXpReward {
            instance_id: 41,
            xp_gained: 75,
            level: 2,
            xp: 50,
            next_level_xp: 100,
        }],
    });
    let pet = &view.state.as_ref().unwrap().teams[0][0];
    assert_eq!((pet.level, pet.xp, pet.next_level_xp), (2, 50, 100));
}
#[test]
fn wild_pet_battle_forfeit_requires_confirmation_and_end_stops_actions() {
    let mut view = WildBattleView::default();
    view.receive(WildPetBattleUpdate::Start(snapshot()));
    assert!(view.action("pb:forfeit").is_none());
    assert!(view.confirm_forfeit);
    assert_eq!(
        view.action("pb:confirm-forfeit").unwrap().action,
        WildPetBattleAction::Forfeit
    );
    view.receive(WildPetBattleUpdate::End {
        battle_id: 100,
        outcome: WildPetBattleOutcome::Forfeited,
        rewards: vec![],
        captured_pet_id: None,
        combat_text: vec![],
    });
    assert!(view.action("pb:trap").is_none());
}

#[test]
fn petbattle_pvp_waiting_player_can_still_confirm_forfeit() {
    let mut view = WildBattleView::default();
    let mut state = snapshot();
    state.wild_creature = 0;
    state.can_trap = false;
    view.receive(WildPetBattleUpdate::Start(state));
    assert!(view.action("pb:pass").is_some());
    assert!(view.pending);
    assert!(view.action("pb:forfeit").is_none());
    assert!(view.confirm_forfeit);
    let request = view.action("pb:confirm-forfeit").unwrap();
    assert_eq!(request.action, WildPetBattleAction::Forfeit);
}
