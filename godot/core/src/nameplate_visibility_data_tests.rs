use crate::nameplate_visibility_data::{
    NameplateCvars, PlateUnit, in_combat_with_player, nameplate_alpha, plate_alpha, plate_shown,
};

#[test]
fn camera_body_distance_steps_to_min_alpha_past_the_limit() {
    let cvars = NameplateCvars::default();
    assert_eq!(nameplate_alpha(&cvars, 10.0, 40.0), 1.0);
    assert_eq!(nameplate_alpha(&cvars, 39.9, 40.0), 1.0);
    assert_eq!(nameplate_alpha(&cvars, 40.0, 40.0), 1.0);
    assert_eq!(nameplate_alpha(&cvars, 40.1, 40.0), 0.6);
    assert_eq!(nameplate_alpha(&cvars, 59.0, 40.0), 0.6);
    assert_eq!(nameplate_alpha(&cvars, 45.0, 60.0), 1.0);
}

const LOCAL: u64 = 1;
const OTHER_PLAYER: u64 = 2;

/// A living, selectable enemy NPC 20 yards away, out of combat and not targeted.
fn enemy_npc() -> PlateUnit {
    PlateUnit {
        is_local_player: false,
        selectable: true,
        alive: true,
        is_player: false,
        enemy: true,
        targeted: false,
        in_combat_with_player: false,
        distance: 20.0,
    }
}

#[test]
fn retail_defaults_match_the_engine_cvars() {
    let cvars = NameplateCvars::default();
    assert!(!cvars.show_all, "nameplateShowAll 0");
    assert!(cvars.show_enemies, "nameplateShowEnemies 1");
    assert!(
        !cvars.show_friendly_players,
        "nameplateShowFriendlyPlayers 0"
    );
    assert!(!cvars.show_friendly_npcs, "nameplateShowFriendlyNpcs 0");
    assert_eq!(cvars.max_distance, 60.0, "nameplateMaxDistance 60");
    assert_eq!(
        cvars.occluded_alpha_mult, 0.4,
        "nameplateOccludedAlphaMult 0.4"
    );
}

#[test]
fn out_of_combat_untargeted_enemy_has_no_plate_by_default() {
    assert!(!plate_shown(&NameplateCvars::default(), &enemy_npc()));
}

#[test]
fn target_and_combat_each_show_an_enemy_plate() {
    let cvars = NameplateCvars::default();
    let targeted = PlateUnit {
        targeted: true,
        ..enemy_npc()
    };
    let fighting = PlateUnit {
        in_combat_with_player: true,
        ..enemy_npc()
    };
    assert!(plate_shown(&cvars, &targeted));
    assert!(plate_shown(&cvars, &fighting));
}

#[test]
fn show_all_shows_every_enemy_regardless_of_combat_or_target() {
    let cvars = NameplateCvars {
        show_all: true,
        ..NameplateCvars::default()
    };
    assert!(plate_shown(&cvars, &enemy_npc()));
}

#[test]
fn enemy_plates_follow_show_enemies_even_for_the_target() {
    let cvars = NameplateCvars {
        show_enemies: false,
        ..NameplateCvars::default()
    };
    let targeted = PlateUnit {
        targeted: true,
        in_combat_with_player: true,
        ..enemy_npc()
    };
    assert!(!plate_shown(&cvars, &targeted));
}

#[test]
fn friendly_npcs_and_players_have_no_plate_by_default_even_when_targeted() {
    let cvars = NameplateCvars::default();
    let friendly_npc = PlateUnit {
        enemy: false,
        targeted: true,
        ..enemy_npc()
    };
    let friendly_player = PlateUnit {
        is_player: true,
        ..friendly_npc
    };
    assert!(!plate_shown(&cvars, &friendly_npc));
    assert!(!plate_shown(&cvars, &friendly_player));
}

#[test]
fn friendly_kinds_are_enabled_independently_and_still_gated_by_show_all() {
    let npcs_on = NameplateCvars {
        show_friendly_npcs: true,
        ..NameplateCvars::default()
    };
    let friendly_npc = PlateUnit {
        enemy: false,
        ..enemy_npc()
    };
    let friendly_player = PlateUnit {
        is_player: true,
        ..friendly_npc
    };
    // Enabled, but out of combat and untargeted with nameplateShowAll 0.
    assert!(!plate_shown(&npcs_on, &friendly_npc));
    let targeted_npc = PlateUnit {
        targeted: true,
        ..friendly_npc
    };
    assert!(plate_shown(&npcs_on, &targeted_npc));
    // The NPC switch does not cover players.
    let targeted_player = PlateUnit {
        targeted: true,
        ..friendly_player
    };
    assert!(!plate_shown(&npcs_on, &targeted_player));
    let players_on = NameplateCvars {
        show_friendly_players: true,
        ..NameplateCvars::default()
    };
    assert!(plate_shown(&players_on, &targeted_player));
    assert!(!plate_shown(&players_on, &targeted_npc));
}

#[test]
fn local_player_unselectable_dead_and_far_units_never_have_a_plate() {
    let cvars = NameplateCvars {
        show_all: true,
        show_friendly_players: true,
        show_friendly_npcs: true,
        ..NameplateCvars::default()
    };
    let target = PlateUnit {
        targeted: true,
        in_combat_with_player: true,
        ..enemy_npc()
    };
    assert!(plate_shown(&cvars, &target));
    for (case, unit) in [
        (
            "local player",
            PlateUnit {
                is_local_player: true,
                is_player: true,
                enemy: false,
                ..target
            },
        ),
        (
            "UNIT_FLAG_NOT_SELECTABLE",
            PlateUnit {
                selectable: false,
                ..target
            },
        ),
        (
            "dead",
            PlateUnit {
                alive: false,
                ..target
            },
        ),
        (
            "beyond nameplateMaxDistance",
            PlateUnit {
                distance: 60.5,
                ..target
            },
        ),
    ] {
        assert!(!plate_shown(&cvars, &unit), "{case} has a plate");
    }
    let at_limit = PlateUnit {
        distance: 60.0,
        ..target
    };
    assert!(plate_shown(&cvars, &at_limit));
}

#[test]
fn a_unit_is_in_combat_with_the_player_only_while_fighting_and_targeting_them() {
    assert!(in_combat_with_player(true, Some(LOCAL), &[], LOCAL));
    assert!(!in_combat_with_player(false, Some(LOCAL), &[], LOCAL));
    assert!(!in_combat_with_player(true, Some(OTHER_PLAYER), &[], LOCAL));
    assert!(!in_combat_with_player(true, None, &[], LOCAL));
}

#[test]
fn occluded_plates_take_the_occluded_alpha_multiplier() {
    let cvars = NameplateCvars::default();
    assert_eq!(plate_alpha(&cvars, false, 1.0, false), 1.0);
    assert_eq!(plate_alpha(&cvars, false, 1.0, true), 0.4);
    let custom = NameplateCvars {
        occluded_alpha_mult: 1.0,
        ..cvars
    };
    assert_eq!(plate_alpha(&custom, false, 1.0, true), 1.0);
}

#[test]
fn the_target_plate_is_opaque_at_any_camera_distance() {
    let cvars = NameplateCvars::default();
    // The target 50 yd from the camera, past the 40 yd limit, would get min alpha.
    let fade = nameplate_alpha(&cvars, 50.0, 40.0);
    assert_eq!(fade, 0.6);
    assert_eq!(plate_alpha(&cvars, true, fade, false), 1.0);
    assert_eq!(plate_alpha(&cvars, true, 0.0, false), 1.0);
    assert_eq!(plate_alpha(&cvars, false, fade, false), 0.6);
    // Line of sight still dims it.
    assert_eq!(plate_alpha(&cvars, true, fade, true), 0.4);
}
