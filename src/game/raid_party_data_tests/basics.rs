use super::*;

#[test]
fn role_labels() {
    assert_eq!(GroupRole::Tank.label(), "T");
    assert_eq!(GroupRole::Healer.label(), "H");
    assert_eq!(GroupRole::Dps.label(), "D");
}

#[test]
fn role_display_names() {
    assert_eq!(GroupRole::Tank.display_name(), "Tank");
    assert_eq!(GroupRole::Healer.display_name(), "Healer");
    assert_eq!(GroupRole::Dps.display_name(), "Damage");
}

#[test]
fn ready_check_active() {
    assert!(!ReadyCheck::None.is_active());
    assert!(ReadyCheck::Pending.is_active());
    assert!(ReadyCheck::Accepted.is_active());
    assert!(ReadyCheck::Declined.is_active());
}

#[test]
fn ready_check_symbols() {
    assert_eq!(ReadyCheck::Accepted.symbol(), "✓");
    assert_eq!(ReadyCheck::Pending.symbol(), "?");
    assert_eq!(ReadyCheck::Declined.symbol(), "✗");
    assert_eq!(ReadyCheck::None.symbol(), "");
}

#[test]
fn power_type_labels() {
    assert_eq!(PowerType::Mana.label(), "Mana");
    assert_eq!(PowerType::Rage.label(), "Rage");
    assert_eq!(PowerType::Energy.label(), "Energy");
    assert_eq!(PowerType::RunicPower.label(), "Runic Power");
}

#[test]
fn debuff_stacks() {
    let d = UnitDebuff {
        name: "Bleed".into(),
        icon_fdid: 1,
        stacks: 3,
        remaining_secs: 10.0,
    };
    assert!(d.has_stacks());

    let single = UnitDebuff {
        stacks: 1,
        ..d.clone()
    };
    assert!(!single.has_stacks());
}

#[test]
fn debuff_time_text() {
    let short = UnitDebuff {
        name: "X".into(),
        icon_fdid: 1,
        stacks: 1,
        remaining_secs: 45.0,
    };
    assert_eq!(short.time_text(), "45s");

    let long = UnitDebuff {
        remaining_secs: 125.0,
        ..short
    };
    assert_eq!(long.time_text(), "2m");
}

#[test]
fn texture_fdids_are_nonzero() {
    assert_ne!(textures::HEALTH_BAR_FILL, 0);
    assert_ne!(textures::LFG_ROLE_ICONS, 0);
    assert_ne!(textures::ROLE_ICONS, 0);
    assert_ne!(textures::LFG_ROLE, 0);
    assert_ne!(textures::READY_CHECK_OK, 0);
    assert_ne!(textures::READY_CHECK_FAIL, 0);
    assert_ne!(textures::READY_CHECK_WAIT, 0);
    assert_ne!(textures::READY_CHECK_FRAME, 0);
    assert_ne!(textures::DEBUFF_BORDER, 0);
    assert_ne!(textures::DEBUFF_OVERLAYS, 0);
}
