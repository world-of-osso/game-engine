use shared::casting::CastType;

use super::{advance_demo_cast, caption, demo_cast};

#[test]
fn demo_casts_are_the_original_normal_and_channel_examples() {
    let normal = demo_cast(false);
    let channel = demo_cast(true);
    assert_eq!(
        (
            normal.cast_type,
            normal.spell_id,
            normal.spell_name.as_str()
        ),
        (CastType::Normal, 133, "Necrotic Bolt")
    );
    assert_eq!(
        (
            channel.cast_type,
            channel.spell_id,
            channel.spell_name.as_str()
        ),
        (CastType::Channel, 5143, "Arcane Missiles")
    );
    assert_eq!((normal.elapsed, normal.duration), (2.0, 5.0));
    assert!((channel.elapsed - 2.4).abs() < 1e-6);
    assert!(normal.interruptible && channel.interruptible);
}

#[test]
fn casts_loop_while_playing_and_hold_while_paused() {
    let mut cast = demo_cast(false);
    advance_demo_cast(&mut cast, 1.0, true);
    assert_eq!(cast.elapsed, 2.0);
    advance_demo_cast(&mut cast, 3.5, false);
    assert!((cast.elapsed - 0.5).abs() < 1e-6, "{}", cast.elapsed);
}

#[test]
fn caption_names_playback_and_selection() {
    assert_eq!(
        caption(false, None),
        "Nameplate preview — Playing\nSpace: pause/resume · Click a plate to select\nSelected: none"
    );
    assert!(caption(true, Some("Channeling Adept")).starts_with("Nameplate preview — Paused\n"));
    assert!(caption(true, Some("Channeling Adept")).ends_with("Selected: Channeling Adept"));
}
