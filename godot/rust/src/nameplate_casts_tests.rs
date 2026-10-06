use shared::casting::CastState;
use shared::spell_data::CastFailReason;

use super::{BarType, Interrupter, PlateCasts, Spark, casting_bar_state, player_casting_bar_state};

#[test]
fn player_castbaranim_success_flashes_then_fades_at_retail_timestamps() {
    use ui_toolkit::atlas::ActiveSkin;
    for skin in [ActiveSkin::Modern, ActiveSkin::Forever] {
        let mut casts = PlateCasts::default();
        casts.observe(UNIT, Some(&bolt(1.0, true)));
        casts.spell_go(UNIT, BOLT); // t=10.000
        casts.advance(0.1, |_| true); // t=10.100
        let state = player_casting_bar_state(casts.get(UNIT).unwrap(), None);
        let mut shared = ui_toolkit::screen::SharedContext::new();
        shared.insert(skin);
        shared.insert(state);
        let mut registry = ui_toolkit::registry::FrameRegistry::new(1920.0, 1080.0);
        ui_toolkit::screen::Screen::new(
            game_engine_ui_model::casting_bar_frame_component::casting_bar_frame_screen,
        )
        .sync(&shared, &mut registry);
        let flash = registry
            .get_by_name("CastingBarFlash")
            .expect("completion flash missing");
        assert!((registry.get(flash).unwrap().alpha - 0.5).abs() < 0.001);
        casts.advance(0.25, |_| true); // t=10.350
        assert!((casts.get(UNIT).unwrap().alpha() - 0.5).abs() < 0.001);
        casts.advance(0.16, |_| true); // t=10.510
        assert!(casts.get(UNIT).is_none());
    }
}

#[test]
fn player_castbaranim_interrupt_fills_after_spark_holds_and_fades() {
    use ui_toolkit::atlas::ActiveSkin;
    for skin in [ActiveSkin::Modern, ActiveSkin::Forever] {
        let mut casts = PlateCasts::default();
        casts.observe(UNIT, Some(&bolt(0.5, true)));
        casts.spell_failure(UNIT, BOLT, CastFailReason::Interrupted, None); // t=20.000
        casts.advance(0.15, |_| true); // t=20.150, spark animation complete
        let state = player_casting_bar_state(casts.get(UNIT).unwrap(), None);
        let mut shared = ui_toolkit::screen::SharedContext::new();
        shared.insert(skin);
        shared.insert(state.clone());
        let mut registry = ui_toolkit::registry::FrameRegistry::new(1920.0, 1080.0);
        ui_toolkit::screen::Screen::new(
            game_engine_ui_model::casting_bar_frame_component::casting_bar_frame_screen,
        )
        .sync(&shared, &mut registry);
        let fill = registry
            .get(registry.get_by_name("CastingBarFill").unwrap())
            .unwrap();
        match skin {
            ActiveSkin::Modern => {
                let Some(ui_toolkit::frame::WidgetData::Texture(texture)) =
                    fill.widget_data.as_ref()
                else {
                    panic!("missing interrupted texture")
                };
                assert_eq!(
                    texture.source,
                    ui_toolkit::widgets::texture::TextureSource::Atlas(
                        "ui-castingbar-interrupted".into()
                    )
                );
            }
            ActiveSkin::Forever => assert_eq!(fill.background_color, Some([1.0, 0.0, 0.0, 1.0])),
        }
        assert!(registry.get_by_name("CastingBarSpark").is_none());
        assert_eq!(state.spell_name, "Interrupted");
        assert_eq!(state.progress, 1.0, "{skin:?}: interrupted fill");
        assert_eq!(state.alpha, 1.0);
        casts.advance(0.85, |_| true); // t=21.000: end of hold
        assert_eq!(casts.get(UNIT).unwrap().alpha(), 1.0);
        casts.advance(0.15, |_| true); // t=21.150
        assert!((casts.get(UNIT).unwrap().alpha() - 0.5).abs() < 0.001);
        casts.advance(0.16, |_| true); // t=21.310
        assert!(casts.get(UNIT).is_none());
    }
}

#[test]
fn player_castbaranim_failure_reads_failed_and_channel_uses_channel_finish() {
    use ui_toolkit::atlas::ActiveSkin;
    for skin in [ActiveSkin::Modern, ActiveSkin::Forever] {
        let mut casts = PlateCasts::default();
        casts.observe(UNIT, Some(&bolt(0.5, true)));
        casts.spell_failure(UNIT, BOLT, CastFailReason::OutOfRange, None); // t=30.000
        assert_eq!(
            casting_bar_state(casts.get(UNIT).unwrap(), None).spell_name,
            "Failed"
        );
        casts.observe(UNIT, Some(&missiles(3.0)));
        casts.observe(UNIT, None); // t=40.000, channel stop
        casts.advance(0.1, |_| true); // t=40.100
        let state = player_casting_bar_state(casts.get(UNIT).unwrap(), None);
        assert!(state.is_channel);
        assert_eq!(state.progress, 0.0);
        assert_eq!(state.alpha, 1.0);
        let mut shared = ui_toolkit::screen::SharedContext::new();
        shared.insert(skin);
        shared.insert(state);
        let mut registry = ui_toolkit::registry::FrameRegistry::new(1920.0, 1080.0);
        ui_toolkit::screen::Screen::new(
            game_engine_ui_model::casting_bar_frame_component::casting_bar_frame_screen,
        )
        .sync(&shared, &mut registry);
        let glow = registry
            .get_by_name("CastingBarBaseGlow")
            .expect("channel finish missing");
        assert!((registry.get(glow).unwrap().alpha - 1.0 / 6.0).abs() < 0.001);
        casts.advance(0.25, |_| true); // t=40.350
        assert!((casts.get(UNIT).unwrap().alpha() - 0.5).abs() < 0.001);
        casts.advance(0.16, |_| true); // t=40.510
        assert!(casts.get(UNIT).is_none());
    }
}

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

/// The HUD bar of `UNIT` after `seconds` more of the bar's clock, if it still shows.
fn hud_after(
    casts: &mut PlateCasts,
    seconds: f32,
) -> Option<game_engine_ui_model::casting_bar_frame_component::CastingBarState> {
    run(casts, seconds);
    casts
        .get(UNIT)
        .map(|bar| casting_bar_state(bar, Some(135846)))
}

/// `HandleInterruptOrSpellFailed` then `HoldFadeOutAnim` (1.0 s hold, 0.3 s fade,
/// CastingBarFrameTemplates.xml:11-13), with the replicated cast already removed.
#[test]
fn hud_cast_bar_interrupted_holds_then_fades_out_and_hides() {
    let mut casts = PlateCasts::default();
    casts.observe(UNIT, Some(&bolt(0.5, true)));
    let s = hud_after(&mut casts, 0.5).expect("casting");
    assert_eq!(
        (s.spell_name.as_str(), s.timer_text.as_str()),
        ("Necrotic Bolt", "1.0")
    );
    assert!(!s.is_interrupted);
    casts.observe(UNIT, None);
    casts.spell_failure(UNIT, BOLT, CastFailReason::Interrupted, None);
    let s = hud_after(&mut casts, 0.0).expect("interrupted");
    assert!(s.visible && s.is_interrupted);
    assert_eq!(s.spell_name, "Interrupted");
    assert_eq!(s.timer_text, "", "no cast time once the cast has ended");
    assert_eq!(s.alpha, 1.0);
    let s = hud_after(&mut casts, 0.95).expect("still held");
    assert_eq!((s.spell_name.as_str(), s.alpha), ("Interrupted", 1.0));
    let s = hud_after(&mut casts, 0.2).expect("fading");
    assert!(s.alpha > 0.0 && s.alpha < 1.0, "fading, alpha {}", s.alpha);
    assert!(
        hud_after(&mut casts, 0.2).is_none(),
        "hidden after the fade"
    );
}

#[test]
fn hud_cast_bar_interrupter_and_failure_texts() {
    let mut casts = PlateCasts::default();
    casts.observe(UNIT, Some(&bolt(0.2, true)));
    let kicker = Interrupter {
        name: "Thrall".into(),
        color: None,
    };
    casts.spell_failure(UNIT, BOLT, CastFailReason::Interrupted, Some(kicker));
    assert_eq!(
        hud_after(&mut casts, 0.0).unwrap().spell_name,
        "Interrupted: Thrall"
    );
    let mut casts = PlateCasts::default();
    casts.observe(UNIT, Some(&bolt(0.2, true)));
    casts.spell_failure(UNIT, BOLT, CastFailReason::OutOfRange, None);
    let s = hud_after(&mut casts, 0.0).expect("failed");
    assert!(s.is_interrupted, "failed and interrupted share the bar art");
    assert_eq!(s.spell_name, "Failed");
    assert!(hud_after(&mut casts, 1.4).is_none());
}

/// `FinishSpell` then `FadeOutAnim` (0.2 s delay, 0.3 s fade,
/// CastingBarFrameTemplates.xml:5-6): a completed cast fills and fades, never red.
#[test]
fn hud_cast_bar_completed_cast_fills_then_fades_out() {
    let mut casts = PlateCasts::default();
    casts.observe(UNIT, Some(&bolt(1.5, true)));
    casts.observe(UNIT, None);
    casts.spell_go(UNIT, BOLT);
    let s = hud_after(&mut casts, 0.0).expect("finished");
    assert_eq!((s.progress, s.alpha), (1.0, 1.0));
    assert_eq!(s.spell_name, "Necrotic Bolt");
    assert!(!s.is_interrupted);
    let s = hud_after(&mut casts, 0.35).expect("fading");
    assert!(s.alpha > 0.0 && s.alpha < 1.0);
    assert!(hud_after(&mut casts, 0.2).is_none());
}

#[test]
fn hud_cast_bar_channel_drains_and_keeps_uninterruptible() {
    let mut casts = PlateCasts::default();
    let mut cast = missiles(1.0);
    cast.interruptible = false;
    casts.observe(UNIT, Some(&cast));
    let s = hud_after(&mut casts, 0.0).expect("channel");
    assert!(s.is_channel && !s.is_interruptible);
    assert!((s.progress - 2.0 / 3.0).abs() < 1e-6);
    assert_eq!(s.timer_text, "2.0");
}
