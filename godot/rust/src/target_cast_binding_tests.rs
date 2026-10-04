use super::target_cast_state;
use crate::nameplate_casts::PlateCasts;
use shared::casting::CastState;

#[test]
fn target_cast_binding_projects_active_target_and_clock_without_a_nameplate() {
    let mut casts = PlateCasts::default();
    let mut cast = CastState::normal(116, 0, 2.0, true);
    cast.spell_name = "Frostbolt".into();
    cast.elapsed = 0.5;
    casts.observe(42, Some(&cast));
    let s = target_cast_state(Some(42), &casts, Some(135846)).expect("target cast");
    assert!(s.visible);
    assert_eq!(s.spell_name, "Frostbolt");
    assert_eq!(s.progress, 0.25);
    assert_eq!(s.timer_text, "1.5");
    assert_eq!(s.icon_fdid, Some(135846));
    assert!(s.is_interruptible);
    casts.advance(0.5, |id| id == 42);
    let s = target_cast_state(Some(42), &casts, Some(135846)).unwrap();
    assert_eq!(s.progress, 0.5);
    assert_eq!(s.timer_text, "1.0");
    assert!(target_cast_state(None, &casts, None).is_none());
    assert!(target_cast_state(Some(43), &casts, None).is_none());
    casts.advance(1.0, |id| id == 42);
    assert!(target_cast_state(Some(42), &casts, None).is_none());
}
#[test]
fn target_cast_binding_channel_drains_and_uninterruptible_is_preserved() {
    let mut casts = PlateCasts::default();
    let mut cast = CastState::channel(5143, 0, 3.0, 1.0, false);
    cast.spell_name = "Arcane Missiles".into();
    cast.elapsed = 1.0;
    casts.observe(42, Some(&cast));
    let s = target_cast_state(Some(42), &casts, None).expect("channel");
    assert!(s.is_channel);
    assert!(!s.is_interruptible);
    assert!((s.progress - 2.0 / 3.0).abs() < 1e-6);
    assert_eq!(s.timer_text, "2.0");
}
