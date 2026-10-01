use shared::casting::CastState;
use shared::spell_data::CastFailReason;

use super::{BarType, Interrupter, PlateCasts, Spark};

const UNIT: u64 = 42;
const BOLT: u32 = 133;
const MISSILES: u32 = 5143;

fn bolt(elapsed: f32, interruptible: bool) -> CastState {
    let mut cast = CastState::normal(BOLT, 0, 2.0, interruptible);
    cast.spell_name = "Necrotic Bolt".into();
    cast.elapsed = elapsed;
    cast
}

fn missiles(elapsed: f32) -> CastState {
    let mut cast = CastState::channel(MISSILES, 0, 3.0, 1.0, true);
    cast.spell_name = "Arcane Missiles".into();
    cast.elapsed = elapsed;
    cast
}

fn run(casts: &mut PlateCasts, seconds: f32) {
    let frames = (seconds / 0.05).round() as usize;
    for _ in 0..frames {
        casts.advance(0.05, |_| true);
    }
}

#[test]
fn a_cast_fills_from_its_replicated_progress_with_icon_and_name() {
    let mut casts = PlateCasts::default();
    casts.observe(UNIT, Some(&bolt(0.5, true)));
    let bar = casts.get(UNIT).unwrap();
    assert_eq!(bar.bar_type, BarType::Standard);
    assert!(bar.casting && !bar.channeling);
    assert_eq!(bar.fraction(), 0.25);
    assert_eq!(bar.text.text, "Necrotic Bolt");
    assert!(bar.icon_shown && !bar.shield_shown);
    assert_eq!(bar.spark, Some(Spark::Pip));
    run(&mut casts, 0.5);
    assert!((casts.get(UNIT).unwrap().fraction() - 0.5).abs() < 1e-4);
}

#[test]
fn an_uninterruptible_cast_shows_the_shield_instead_of_the_icon() {
    let mut casts = PlateCasts::default();
    casts.observe(UNIT, Some(&bolt(0.0, false)));
    let bar = casts.get(UNIT).unwrap();
    assert_eq!(bar.bar_type, BarType::Uninterruptable);
    assert!(!bar.icon_shown && bar.shield_shown);
    // `UNIT_SPELLCAST_INTERRUPTIBLE` mid-cast.
    casts.observe(UNIT, Some(&bolt(0.1, true)));
    let bar = casts.get(UNIT).unwrap();
    assert_eq!(bar.bar_type, BarType::Standard);
    assert!(bar.icon_shown && !bar.shield_shown);
}

#[test]
fn a_channel_drains_and_its_end_without_interrupter_fades_like_a_finish() {
    let mut casts = PlateCasts::default();
    casts.observe(UNIT, Some(&missiles(1.0)));
    let bar = casts.get(UNIT).unwrap();
    assert_eq!(bar.bar_type, BarType::Channel);
    assert!((bar.fraction() - 2.0 / 3.0).abs() < 1e-4);
    casts.observe(UNIT, None);
    let bar = casts.get(UNIT).unwrap();
    assert!(!bar.channeling && bar.full && bar.spark.is_none());
    assert_eq!(bar.alpha(), 1.0);
    run(&mut casts, 0.35);
    let alpha = casts.get(UNIT).unwrap().alpha();
    assert!((alpha - 0.5).abs() < 0.02, "alpha {alpha}");
    run(&mut casts, 0.2);
    assert!(casts.get(UNIT).is_none(), "faded out and hidden");
}

#[test]
fn a_removed_cast_keeps_filling_until_spell_go_completes_it() {
    let mut casts = PlateCasts::default();
    casts.observe(UNIT, Some(&bolt(1.8, true)));
    casts.observe(UNIT, None);
    assert!(casts.get(UNIT).unwrap().casting);
    casts.spell_go(UNIT, BOLT);
    let bar = casts.get(UNIT).unwrap();
    assert!(!bar.casting && bar.full);
    assert_eq!(bar.fraction(), 1.0);
    assert_eq!(bar.text.text, "Necrotic Bolt");
}

#[test]
fn an_interrupt_turns_the_bar_red_names_the_interrupter_and_holds() {
    let mut casts = PlateCasts::default();
    casts.observe(UNIT, Some(&bolt(1.0, true)));
    casts.observe(UNIT, None);
    let kicker = Interrupter {
        name: "Theron".into(),
        color: Some([1.0, 0.96, 0.41]),
    };
    casts.spell_failure(
        UNIT,
        BOLT,
        CastFailReason::InterruptedCombat,
        Some(kicker.clone()),
    );
    let bar = casts.get(UNIT).unwrap();
    assert_eq!(bar.bar_type, BarType::Interrupted);
    assert_eq!(bar.spark, Some(Spark::PipRed));
    assert_eq!(bar.text.text, "Interrupted: ");
    assert_eq!(bar.text.name, Some(kicker));
    // The fill stays where the cast stopped.
    assert_eq!(bar.fraction(), 0.5);
    run(&mut casts, 0.95);
    assert_eq!(casts.get(UNIT).unwrap().alpha(), 1.0);
    run(&mut casts, 0.2);
    assert!(casts.get(UNIT).unwrap().alpha() < 0.6);
    run(&mut casts, 0.2);
    assert!(casts.get(UNIT).is_none());
}

#[test]
fn moving_interrupts_without_a_name_and_completion_failures_read_failed() {
    let mut casts = PlateCasts::default();
    casts.observe(UNIT, Some(&bolt(0.5, true)));
    casts.spell_failure(UNIT, BOLT, CastFailReason::Interrupted, None);
    assert_eq!(casts.get(UNIT).unwrap().text.text, "Interrupted");
    let mut casts = PlateCasts::default();
    casts.observe(UNIT, Some(&bolt(1.9, true)));
    casts.spell_failure(UNIT, BOLT, CastFailReason::OutOfRange, None);
    let bar = casts.get(UNIT).unwrap();
    assert_eq!(bar.text.text, "Failed");
    assert_eq!(bar.bar_type, BarType::Interrupted);
}

#[test]
fn a_kicked_channel_is_interrupted_even_after_its_stop_started_the_fade() {
    let mut casts = PlateCasts::default();
    casts.observe(UNIT, Some(&missiles(1.0)));
    casts.observe(UNIT, None);
    let kicker = Interrupter {
        name: "Theron".into(),
        color: None,
    };
    casts.spell_failure(
        UNIT,
        MISSILES,
        CastFailReason::InterruptedCombat,
        Some(kicker),
    );
    let bar = casts.get(UNIT).unwrap();
    assert_eq!(bar.bar_type, BarType::Interrupted);
    assert_eq!(bar.text.text, "Interrupted: ");
}

#[test]
fn a_finished_cast_ignores_late_failures_and_a_recast_restarts_the_bar() {
    let mut casts = PlateCasts::default();
    casts.observe(UNIT, Some(&bolt(1.95, true)));
    run(&mut casts, 0.1);
    assert!(casts.get(UNIT).unwrap().full);
    casts.spell_failure(UNIT, BOLT, CastFailReason::Interrupted, None);
    assert_eq!(casts.get(UNIT).unwrap().bar_type, BarType::Standard);
    // `UNIT_SPELLCAST_START` while the old bar fades.
    casts.observe(UNIT, Some(&bolt(0.0, true)));
    let bar = casts.get(UNIT).unwrap();
    assert!(bar.casting && !bar.full);
    assert_eq!(bar.alpha(), 1.0);
    assert_eq!(bar.fraction(), 0.0);
}

#[test]
fn pushback_resyncs_but_progress_going_back_without_one_is_a_new_cast() {
    let mut casts = PlateCasts::default();
    casts.observe(UNIT, Some(&bolt(1.0, true)));
    run(&mut casts, 0.2);
    let mut pushed = bolt(0.5, true);
    pushed.pushback_count = 1;
    casts.observe(UNIT, Some(&pushed));
    assert_eq!(casts.get(UNIT).unwrap().fraction(), 0.25);
    casts.spell_failure(UNIT, BOLT, CastFailReason::Interrupted, None);
    assert_eq!(casts.get(UNIT).unwrap().bar_type, BarType::Interrupted);
    casts.observe(UNIT, Some(&bolt(0.1, true)));
    assert_eq!(casts.get(UNIT).unwrap().bar_type, BarType::Standard);
}

#[test]
fn bars_of_units_without_plates_are_dropped() {
    let mut casts = PlateCasts::default();
    casts.observe(UNIT, Some(&bolt(0.5, true)));
    casts.advance(0.05, |unit| unit != UNIT);
    assert!(casts.get(UNIT).is_none());
}

#[test]
fn a_failure_before_the_replicated_removal_keeps_the_bar_interrupted() {
    let mut casts = PlateCasts::default();
    casts.observe(UNIT, Some(&bolt(1.0, true)));
    casts.spell_failure(UNIT, BOLT, CastFailReason::Interrupted, None);
    // The same cast is still replicated for a frame or two.
    casts.observe(UNIT, Some(&bolt(1.0, true)));
    run(&mut casts, 0.1);
    casts.observe(UNIT, Some(&bolt(1.0, true)));
    assert_eq!(casts.get(UNIT).unwrap().bar_type, BarType::Interrupted);
    casts.observe(UNIT, None);
    assert_eq!(casts.get(UNIT).unwrap().bar_type, BarType::Interrupted);
    // After the gap the same spell is a new cast.
    casts.observe(UNIT, Some(&bolt(1.0, true)));
    assert_eq!(casts.get(UNIT).unwrap().bar_type, BarType::Standard);
}

#[test]
fn a_finished_cast_still_replicated_does_not_restart() {
    let mut casts = PlateCasts::default();
    casts.observe(UNIT, Some(&bolt(1.9, true)));
    run(&mut casts, 0.2);
    assert!(casts.get(UNIT).unwrap().full);
    casts.observe(UNIT, Some(&bolt(1.9, true)));
    let bar = casts.get(UNIT).unwrap();
    assert!(!bar.casting && bar.full);
}
