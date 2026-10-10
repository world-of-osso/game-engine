use game_engine_ui_model::wild_pet_battle::{WildBattleView, wild_battle_screen};
use shared::protocol::*;
use ui_toolkit::{
    atlas::{ActiveSkin, set_thread_skin},
    frame::{Dimension, WidgetData},
    registry::FrameRegistry,
    screen::{Screen, SharedContext},
};

fn snapshot() -> WildPetBattleSnapshot {
    let pet = BattlePetSnapshot {
        instance_id: Some(41),
        species_id: 39,
        name: "Mechanical Squirrel".into(),
        display_id: 7937,
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
            cooldown: if index == 0 { 2 } else { 0 },
            usable: false,
        }),
        auras: vec![BattleAuraSnapshot {
            ability_id: 194,
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
        replacement_required: false,
    }
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
        let timer = registry
            .get(registry.get_by_name("PetBattleTurnTimerText").unwrap())
            .unwrap();
        assert!(
            matches!(timer.widget_data.as_ref(), Some(WidgetData::FontString(text)) if text.text=="Select an action")
        );
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
