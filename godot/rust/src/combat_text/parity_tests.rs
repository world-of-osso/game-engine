use super::*;
use game_engine_core::combat_text_options::CombatTextOptions;
use incoming::{Motion, message_for, stack_start_y};

fn event(kind: CombatLogKind, amount: i32) -> CombatLogEvent {
    CombatLogEvent {
        source: Some(1),
        target: Some(2),
        spell_id: Some(133),
        school_mask: 4,
        amount,
        overflow: 0,
        absorbed: 0,
        resisted: 0,
        blocked: 0,
        crit: false,
        glancing: false,
        periodic: false,
        extra_spell_id: None,
        timestamp_unix_ms: 0,
        kind,
    }
}

fn enabled() -> CombatTextOptions {
    CombatTextOptions {
        self_enabled: true,
        ..Default::default()
    }
}

#[test]
fn incoming_is_opt_in_and_separate_from_world_damage_and_periodic_gates() {
    let hit = event(CombatLogKind::Damage, 12345);
    assert!(message_for(&hit, &CombatTextOptions::default(), None).is_none());
    let options = CombatTextOptions {
        world_damage: false,
        periodic: false,
        ..enabled()
    };
    assert_eq!(message_for(&hit, &options, None).unwrap().text, "-12,345");
    assert!(text_for_with_options(&hit, Attacker::Player, &options).is_none());
    let tick = CombatLogEvent {
        periodic: true,
        ..hit
    };
    assert!(text_for_with_options(&tick, Attacker::Player, &enabled()).is_some());
    assert!(text_for_with_options(&tick, Attacker::Player, &options).is_none());
    assert!(message_for(&tick, &options, None).is_some());
}

#[test]
fn incoming_damage_uses_role_colors_not_outgoing_school_colors() {
    let spell = event(CombatLogKind::Damage, 92);
    assert_eq!(
        message_for(&spell, &enabled(), None).unwrap().color,
        [0.79, 0.3, 0.85]
    );
    let melee = CombatLogEvent {
        spell_id: None,
        ..spell
    };
    let message = message_for(&melee, &enabled(), None).unwrap();
    assert_eq!(message.color, [1.0, 0.1, 0.1]);
    assert!(message.stagger);
    assert!(!message.crit);
    let crit = CombatLogEvent {
        crit: true,
        ..melee
    };
    assert!(message_for(&crit, &enabled(), None).unwrap().crit);
    assert!(!message_for(&crit, &enabled(), None).unwrap().stagger);
}

#[test]
fn incoming_heals_keep_effective_amount_and_optional_healer_names() {
    let heal = CombatLogEvent {
        overflow: 200,
        ..event(CombatLogKind::Heal, 12345)
    };
    let plain = message_for(&heal, &enabled(), Some("Alice")).unwrap();
    assert_eq!(plain.text, "+12,345");
    assert_eq!(plain.color, [0.1, 1.0, 0.1]);
    let names = CombatTextOptions {
        friendly_healers: true,
        ..enabled()
    };
    assert_eq!(
        message_for(&heal, &names, Some("Alice")).unwrap().text,
        "+12,345 [Alice]"
    );
    let periodic = CombatLogEvent {
        periodic: true,
        ..heal.clone()
    };
    assert_eq!(
        message_for(&periodic, &names, Some("Alice")).unwrap().text,
        "+12,345"
    );
    let crit_tick = CombatLogEvent {
        crit: true,
        ..periodic
    };
    assert_eq!(
        message_for(&crit_tick, &names, Some("Alice")).unwrap().text,
        "+12,345 [Alice]"
    );
    assert_eq!(
        message_for(
            &CombatLogEvent {
                absorbed: 50,
                ..heal
            },
            &names,
            None
        )
        .unwrap()
        .text,
        "+12,345  (50 absorbed)"
    );
    let absorb_hidden = CombatTextOptions {
        healing_absorb_self: false,
        ..names
    };
    assert_eq!(
        message_for(
            &CombatLogEvent {
                absorbed: 50,
                ..event(CombatLogKind::Heal, 12345)
            },
            &absorb_hidden,
            None,
        )
        .unwrap()
        .text,
        "+12,345"
    );
}

#[test]
fn incoming_avoidance_and_reductions_have_independent_switches() {
    let dodge = event(CombatLogKind::Miss(MissKind::Dodge), 0);
    assert!(message_for(&dodge, &enabled(), None).is_none());
    let avoidance = CombatTextOptions {
        avoidance: true,
        ..enabled()
    };
    assert_eq!(message_for(&dodge, &avoidance, None).unwrap().text, "Dodge");
    let absorb = event(CombatLogKind::Miss(MissKind::Absorb), 0);
    assert!(message_for(&absorb, &avoidance, None).is_none());
    let reductions = CombatTextOptions {
        reductions: true,
        ..enabled()
    };
    assert_eq!(
        message_for(&absorb, &reductions, None).unwrap().text,
        "Absorb"
    );
    for (kind, word) in [
        (MissKind::Miss, "Miss"),
        (MissKind::Parry, "Parry"),
        (MissKind::Immune, "Immune"),
        (MissKind::Evade, "Evade"),
        (MissKind::Deflect, "Deflect"),
        (MissKind::Reflect, "Reflect"),
    ] {
        assert_eq!(
            message_for(&event(CombatLogKind::Miss(kind), 0), &avoidance, None)
                .unwrap()
                .text,
            word
        );
    }
    for (absorbed, resisted, suffix) in [(20, 0, " (20 absorbed)"), (0, 30, " (30 resisted)")] {
        let hit = CombatLogEvent {
            absorbed,
            resisted,
            ..event(CombatLogKind::Damage, 70)
        };
        assert_eq!(message_for(&hit, &enabled(), None).unwrap().text, "-70");
        assert_eq!(
            message_for(&hit, &reductions, None).unwrap().text,
            format!("-70 {suffix}")
        );
    }
    let partial_block = CombatLogEvent {
        blocked: 40,
        ..event(CombatLogKind::Damage, 70)
    };
    assert_eq!(
        message_for(&partial_block, &enabled(), None).unwrap().text,
        "40 (Block)"
    );
    assert_eq!(
        message_for(&partial_block, &reductions, None).unwrap().text,
        "40 (Block)"
    );
}

fn motion(mode: i32, crit: bool) -> Motion {
    Motion {
        mode,
        crit,
        start: [0.0, 384.0],
        x_dir: 1.0,
        viewport: [1024.0, 768.0],
    }
}

#[test]
fn incoming_rows_stack_in_the_selected_direction_and_scale_with_the_viewport() {
    assert_eq!(stack_start_y(1, 768.0, &[]), 384.0);
    assert_eq!(stack_start_y(1, 768.0, &[384.0]), 358.0);
    assert_eq!(stack_start_y(1, 768.0, &[384.0, 358.0]), 332.0);
    assert_eq!(stack_start_y(2, 768.0, &[384.0]), 410.0);
    assert_eq!(stack_start_y(3, 1536.0, &[384.0]), 366.0);
}

#[test]
fn incoming_scroll_and_fountain_follow_retail_lua_not_world_rise() {
    let up = motion(1, false).frame(0.95);
    assert_eq!(up.position, [512.0, 271.5]);
    assert_eq!(motion(2, false).frame(0.95).position, [512.0, 496.5]);
    let fountain = motion(3, false).frame(0.95);
    assert!((fountain.position[0] - 468.066).abs() < 0.001);
    assert!((fountain.position[1] - 277.934).abs() < 0.001);
    for mode in [1, 2, 3] {
        assert_eq!(motion(mode, true).frame(0.95).position, [512.0, 384.0]);
    }
}

#[test]
fn incoming_crit_font_and_fade_have_their_own_lifetime() {
    let crit = motion(1, true);
    assert_eq!(crit.frame(0.0).height, 30.0);
    assert_eq!(crit.frame(0.05).height, 60.0);
    assert_eq!(crit.frame(0.125).height, 45.0);
    assert!((crit.frame(0.2).height - 30.0).abs() < 1e-4);
    assert_eq!(crit.frame(1.0).height, 30.0);
    let plain = motion(1, false);
    assert_eq!(plain.frame(0.0).height, 25.0);
    assert_eq!(plain.frame(1.3).alpha, 1.0);
    assert!((plain.frame(1.6).alpha - 0.5).abs() < 1e-5);
    assert_eq!(plain.frame(1.9).alpha, 0.0);
}

#[test]
fn outgoing_runtime_cvar_gates_do_not_change_other_event_families() {
    let hit = event(CombatLogKind::Damage, 100);
    let options = CombatTextOptions {
        pet_melee: false,
        world_healing: false,
        ..Default::default()
    };
    assert!(text_for_with_options(&hit, Attacker::PetMelee, &options).is_none());
    assert!(text_for_with_options(&hit, Attacker::PetSpell, &options).is_some());
    assert!(text_for_with_options(&hit, Attacker::Player, &options).is_some());
    assert!(
        text_for_with_options(&event(CombatLogKind::Heal, 100), Attacker::Player, &options)
            .is_none()
    );
}

#[test]
fn world_scale_and_height_pair_are_runtime_multipliers() {
    let options = CombatTextOptions {
        world_scale: 2.0,
        crit_screen_y: 0.045,
        ..Default::default()
    };
    assert!((em_yards_with_options(75.0, &options) - em_yards(75.0) * 2.0).abs() < 1e-6);
    let crit = CombatLogEvent {
        crit: true,
        ..event(CombatLogKind::Damage, 88)
    };
    assert!(
        (text_for_with_options(&crit, Attacker::Player, &options)
            .unwrap()
            .style
            .scale
            - 3.0)
            .abs()
            < 1e-6
    );
}
