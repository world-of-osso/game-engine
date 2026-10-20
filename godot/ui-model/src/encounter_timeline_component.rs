//! Retail timer view (300×300, ten 34px timers, 2px spacing), track view,
//! and three independent severity warning regions. Forever changes art only.
use crate::{
    encounter_timeline::{EncounterTimeline, TimerView, WarningView},
    ui::{
        strata::FrameStrata,
        widgets::font_string::{FontColor, GameFont},
    },
};
use shared::protocol::EncounterSeverity;
use ui_toolkit::{rsx, screen::SharedContext, widget_def::Element};

#[derive(Clone, Debug, PartialEq)]
pub struct EncounterTimelineScreen {
    pub timeline: EncounterTimeline,
    pub enabled: bool,
    pub bars: bool,
    pub warnings: bool,
    pub preview: bool,
}
struct DynName(String);

// EncounterTimelineTimerView.xml padding and TimerEvent.xml/Lua default layout.
const MAX_TIMERS: usize = 10;
const ROW_WIDTH: f32 = 300.0;
const ROW_HEIGHT: f32 = 34.0;
const ROW_SPACING: f32 = 2.0;
const HORIZONTAL_PADDING: f32 = 20.0;
const TOP_PADDING: f32 = 10.0;
const BOTTOM_PADDING: f32 = 5.0;
const TRACK_DIVIDER_HEIGHT: f32 = 14.0;
// TimerView.lua UpdateSize replaces the XML's placeholder300x300.
const VIEW_WIDTH: f32 = HORIZONTAL_PADDING + ROW_WIDTH;
const VIEW_HEIGHT: f32 = TOP_PADDING
    + TRACK_DIVIDER_HEIGHT
    + MAX_TIMERS as f32 * ROW_HEIGHT
    + (MAX_TIMERS - 1) as f32 * ROW_SPACING
    + BOTTOM_PADDING;
const BAR_LEFT: f32 = 40.0; // Icon LEFT +2, width34, Bar LEFT +4.
const BAR_WIDTH: f32 = ROW_WIDTH - BAR_LEFT - 4.0;
const BAR_HEIGHT: f32 = 26.0;

#[cfg(test)]
mod tests {
    use super::*;
    use shared::protocol::{EncounterTimelineEvent, EncounterTimer};
    use ui_toolkit::{frame::WidgetData, registry::FrameRegistry, screen::Screen};
    #[test]
    fn ui_template_blend_timer_bars_project_source_art_and_remaining_fraction_in_both_skins() {
        use ui_toolkit::{
            frame::Dimension,
            layout_values::Val,
            widgets::texture::{BlendMode, TextureSource},
        };
        crate::paths::set_data_root(
            std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../data"),
        )
        .unwrap();
        for skin in [
            ui_toolkit::atlas::ActiveSkin::Modern,
            ui_toolkit::atlas::ActiveSkin::Forever,
        ] {
            ui_toolkit::atlas::set_thread_skin(skin);
            for remaining_ms in [10_000, 5_000, 0] {
                let mut timeline = EncounterTimeline::default();
                timeline.receive(
                    EncounterTimelineEvent::Timers {
                        caster: 3669,
                        sample_unix_ms: 0,
                        timers: vec![EncounterTimer {
                            event_id: 2,
                            spell_id: 20805,
                            remaining_ms,
                            duration_ms: 10_000,
                            conditional: false,
                        }],
                    },
                    0,
                    |_| ("Lightning Bolt".into(), 136048),
                );
                let mut context = SharedContext::new();
                context.insert(EncounterTimelineScreen {
                    timeline,
                    enabled: true,
                    bars: true,
                    warnings: false,
                    preview: false,
                });
                let mut registry = FrameRegistry::new(1920.0, 1080.0);
                Screen::new(encounter_timeline_screen).sync(&context, &mut registry);
                let frame = |name| {
                    registry
                        .get(registry.get_by_name(name).expect(name))
                        .unwrap()
                };
                let background = frame("EncounterTimer1Background");
                assert_eq!(
                    [background.width, background.height],
                    [Dimension::Fixed(264.0), Dimension::Fixed(36.0)]
                );
                assert_eq!(
                    [background.position.left, background.position.top],
                    [Val::Px(-2.0), Val::Px(-3.0)]
                );
                let Some(WidgetData::Texture(texture)) = &background.widget_data else {
                    panic!("source bar background");
                };
                assert_eq!(
                    texture.source,
                    TextureSource::Atlas("UI-HUD-CoolDownManager-Bar-BG".into())
                );
                assert_eq!(frame("EncounterTimeline").width, Dimension::Fixed(320.0));
                assert_eq!(frame("EncounterTimeline").height, Dimension::Fixed(387.0));
                assert_eq!(frame("EncounterTimer1").width, Dimension::Fixed(300.0));
                assert_eq!(frame("EncounterTimer1").position.left, Val::Px(10.0));
                assert_eq!(frame("EncounterTimer1").position.top, Val::Px(343.0));
                let bar = frame("EncounterTimer1Bar");
                assert_eq!(
                    [bar.width, bar.height],
                    [Dimension::Fixed(256.0), Dimension::Fixed(26.0)]
                );
                assert_eq!(
                    [bar.position.left, bar.position.top],
                    [Val::Px(40.0), Val::Px(4.0)]
                );
                let fraction = remaining_ms as f32 / 10_000.0;
                let fill = frame("EncounterTimer1Fill");
                assert_eq!(fill.width, Dimension::Fixed(256.0 * fraction));
                let Some(WidgetData::Texture(texture)) = &fill.widget_data else {
                    panic!("source bar fill");
                };
                assert_eq!(
                    texture.source,
                    TextureSource::Atlas("UI-HUD-CoolDownManager-Bar".into())
                );
                assert_eq!(texture.tex_coords, [0.0, fraction, 0.0, 1.0]);
                assert_eq!(texture.blend_mode, BlendMode::Blend);
                let spark = frame("EncounterTimer1Spark");
                let Some(WidgetData::Texture(texture)) = &spark.widget_data else {
                    panic!("source bar spark");
                };
                assert_eq!(
                    (texture.source.clone(), texture.blend_mode),
                    (
                        TextureSource::Atlas("UI-HUD-CoolDownManager-Bar-Pip".into()),
                        BlendMode::Additive
                    )
                );
                assert_eq!(
                    [spark.width, spark.height],
                    [Dimension::Fixed(10.0), Dimension::Fixed(46.0)]
                );
                assert_eq!(
                    [spark.position.left, spark.position.top],
                    [Val::Px(256.0 * fraction - 5.0), Val::Px(-10.0)]
                );
                assert_eq!(spark.hidden, remaining_ms == 0);
            }
        }
    }

    #[test]
    fn timeline_screen_both_skins_countdown_text_track_and_warning_visibility() {
        crate::paths::set_data_root(
            std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../data"),
        )
        .unwrap();
        for skin in [
            ui_toolkit::atlas::ActiveSkin::Modern,
            ui_toolkit::atlas::ActiveSkin::Forever,
        ] {
            ui_toolkit::atlas::set_thread_skin(skin);
            let mut timeline = EncounterTimeline::default();
            let spell = |_: u32| ("Lightning Bolt".into(), 136048);
            timeline.receive(
                EncounterTimelineEvent::Timers {
                    caster: 3669,
                    sample_unix_ms: 0,
                    timers: vec![EncounterTimer {
                        event_id: 2,
                        spell_id: 20805,
                        remaining_ms: 7312,
                        duration_ms: 7312,
                        conditional: true,
                    }],
                },
                0,
                spell,
            );
            timeline.advance(2.0);
            timeline.receive(
                EncounterTimelineEvent::Cast {
                    caster: 3669,
                    spell_id: 20805,
                    severity: EncounterSeverity::Low,
                    warning_text: "Casting Lightning Bolt".into(),
                },
                0,
                spell,
            );
            let mut state = EncounterTimelineScreen {
                timeline,
                enabled: true,
                bars: true,
                warnings: true,
                preview: false,
            };
            let mut ctx = SharedContext::new();
            ctx.insert(state.clone());
            let mut registry = FrameRegistry::new(1280.0, 720.0);
            let mut screen = Screen::new(encounter_timeline_screen);
            screen.sync(&ctx, &mut registry);
            let label = registry
                .get(registry.get_by_name("EncounterTimer1Duration").unwrap())
                .unwrap();
            let Some(WidgetData::FontString(label)) = &label.widget_data else {
                panic!("countdown text");
            };
            assert_eq!(label.text, "5.3");
            assert!(
                !registry
                    .get(registry.get_by_name("MinorEncounterWarnings").unwrap())
                    .unwrap()
                    .hidden
            );
            assert!(
                registry
                    .get(registry.get_by_name("CriticalEncounterWarnings").unwrap())
                    .unwrap()
                    .hidden
            );
            state.bars = false;
            ctx.insert(state.clone());
            screen.sync(&ctx, &mut registry);
            assert!(registry.get_by_name("EncounterTrackEvent1").is_some());
            state.enabled = false;
            state.warnings = false;
            ctx.insert(state);
            screen.sync(&ctx, &mut registry);
            assert!(
                registry
                    .get(registry.get_by_name("EncounterTimeline").unwrap())
                    .unwrap()
                    .hidden
            );
            assert!(
                registry
                    .get(registry.get_by_name("MinorEncounterWarnings").unwrap())
                    .unwrap()
                    .hidden
            );
        }
        ui_toolkit::atlas::set_thread_skin(ui_toolkit::atlas::ActiveSkin::Modern);
    }
}

pub fn encounter_timeline_screen(ctx: &SharedContext) -> Element {
    let view = ctx
        .get::<EncounterTimelineScreen>()
        .expect("EncounterTimelineScreen");
    let hidden = !view.preview && (!view.enabled || view.timeline.timers.is_empty());
    let content: Element = if view.bars {
        view.timeline
            .timers
            .iter()
            .take(MAX_TIMERS)
            .enumerate()
            .flat_map(|(i, t)| timer_bar(i, t))
            .collect()
    } else {
        track(&view.timeline.timers)
    };
    let warnings: Element = [
        EncounterSeverity::High,
        EncounterSeverity::Medium,
        EncounterSeverity::Low,
    ]
    .into_iter()
    .flat_map(|severity| {
        warning_region(
            severity,
            view.timeline
                .warnings
                .iter()
                .find(|w| w.severity == severity),
            view.warnings,
            view.preview,
        )
    })
    .collect();
    let timeline = rsx! { r#frame {
        name: "EncounterTimeline", width: {if view.bars {VIEW_WIDTH} else {300.0}}, height: {if view.bars {VIEW_HEIGHT} else {55.0}},
        hidden, mouse_enabled: false, strata: FrameStrata::High,
        pos_type: "absolute", left: 110.0, top: 280.0,
        {content}
    } };
    timeline.into_iter().chain(warnings).collect()
}

fn timer_bar(index: usize, timer: &TimerView) -> Element {
    let prefix = format!("EncounterTimer{}", index + 1);
    let fraction =
        (timer.remaining * 1000.0 / timer.timer.duration_ms.max(1) as f32).clamp(0.0, 1.0);
    let text = format!(
        "{}{}",
        timer.name,
        if timer.timer.conditional { " *" } else { "" }
    );
    let seconds = format!("{:.1}", timer.remaining);
    let color = bar_color(timer.remaining <= 5.0);
    let art = timer_bar_art(&prefix, fraction, &color);
    let labels = timer_bar_labels(&prefix, &text, &seconds);
    let top = VIEW_HEIGHT - TOP_PADDING - ROW_HEIGHT - index as f32 * (ROW_HEIGHT + ROW_SPACING);
    rsx! { r#frame {
        name: {DynName(prefix.clone())}, width: ROW_WIDTH, height: ROW_HEIGHT,
        pos_type: "absolute", left: {HORIZONTAL_PADDING / 2.0}, top, mouse_enabled: false,
        texture { name: {DynName(format!("{prefix}Icon"))},
            texture_fdid: {timer.icon}, width: ROW_HEIGHT, height: ROW_HEIGHT,
            pos_type: "absolute", left: 2.0, top: 0.0 }
        r#frame { name: {DynName(format!("{prefix}Bar"))}, width: BAR_WIDTH, height: BAR_HEIGHT,
            pos_type: "absolute", left: BAR_LEFT, top: 4.0, {art} {labels} }
    } }
}

fn timer_bar_art(prefix: &str, fraction: f32, color: &str) -> Element {
    let spark = ui_toolkit::atlas::resolve_region(
        "UI-HUD-CoolDownManager-Bar-Pip",
        ui_toolkit::atlas::thread_skin(),
    )
    .expect("EncounterTimeline timer spark atlas");
    let width = BAR_WIDTH * fraction;
    let coords = format!("0,{fraction},0,1");
    let hidden = fraction <= 0.0;
    rsx! {
        texture { name: {DynName(format!("{prefix}Background"))},
            texture_atlas: "UI-HUD-CoolDownManager-Bar-BG", width: {BAR_WIDTH + 8.0}, height: 36.0,
            pos_type: "absolute", left: -2.0, top: -3.0, draw_layer: "BACKGROUND" }
        texture { name: {DynName(format!("{prefix}Fill"))},
            texture_atlas: "UI-HUD-CoolDownManager-Bar", width, height: BAR_HEIGHT,
            tex_coords: {coords.as_str()}, vertex_color: color, hidden,
            pos_type: "absolute", left: 0.0, top: 0.0 }
        texture { name: {DynName(format!("{prefix}Spark"))},
            texture_atlas: "UI-HUD-CoolDownManager-Bar-Pip", width: spark.width, height: spark.height,
            hidden, pos_type: "absolute", left: {width - spark.width / 2.0},
            top: {(BAR_HEIGHT - spark.height) / 2.0}, draw_layer: "OVERLAY",
            // EncounterTimelineTimerEvent.xml:53 Spark alphaMode="ADD".
            blend_mode: "ADD" }
    }
}

fn timer_bar_labels(prefix: &str, text: &str, seconds: &str) -> Element {
    // TimerEvent.lua:586-635: countdown RIGHT -5; name LEFT +5 and countdown LEFT -10.
    const DURATION_WIDTH: f32 = 42.0;
    let duration_left = BAR_WIDTH - 5.0 - DURATION_WIDTH;
    rsx! {
        fontstring { name: {DynName(format!("{prefix}Name"))}, text,
            width: {duration_left - 15.0}, height: BAR_HEIGHT, font: GameFont::FrizQuadrata, font_size: 13.0,
            font_color: {FontColor::new(1.0,1.0,1.0,1.0)}, justify_h: "LEFT",
            pos_type: "absolute", left: 5.0, top: 0.0, frame_level: 5.0 }
        fontstring { name: {DynName(format!("{prefix}Duration"))}, text: seconds,
            width: DURATION_WIDTH, height: BAR_HEIGHT, font: GameFont::FrizQuadrata, font_size: 13.0,
            font_color: {FontColor::new(1.0,1.0,1.0,1.0)}, justify_h: "RIGHT",
            pos_type: "absolute", left: duration_left, top: 0.0, frame_level: 5.0 }
    }
}

fn bar_color(imminent: bool) -> String {
    if imminent {
        return "0.9,0.32,0.12,1".into();
    }
    match ui_toolkit::atlas::thread_skin() {
        ui_toolkit::atlas::ActiveSkin::Forever => "0.63,0.42,0.15,1".into(),
        _ => "0.12,0.55,0.72,1".into(),
    }
}

fn track(timers: &[TimerView]) -> Element {
    // Events travel toward the zero-time end. Long events queue beyond the
    // 30s linear span; no cast is generated when they reach that end.
    let icons: Element = timers.iter().take(10).enumerate().flat_map(|(i,t)| {
        let seconds = format!("{:.1}",t.remaining);
        rsx! { r#frame {
            name: {DynName(format!("EncounterTrackEvent{}",i+1))}, width: 34.0, height: 55.0,
            pos_type: "absolute", left: {20.0+226.0*(t.remaining/30.0).clamp(0.0,1.0)}, top: 0.0,
            texture { texture_fdid: {t.icon}, width: 34.0, height: 34.0 }
            fontstring { text: {seconds.as_str()}, width: 34.0, height: 18.0, font_size: 12.0,
                justify_h: "CENTER", pos_type: "absolute", left: 0.0, top: 36.0 }
        } }
    }).collect();
    rsx! { r#frame {
        name: "EncounterTimelineTrackView", width: 300.0, height: 55.0,
        r#frame { width: 260.0, height: 2.0, background_color: "0.7,0.7,0.7,1",
            pos_type: "absolute", left: 20.0, top: 17.0 }
        fontstring { text: "5", width: 20.0, height: 18.0, font_size: 12.0,
            pos_type: "absolute", left: 57.0, top: 36.0 }
        {icons}
    } }
}

fn warning_region(
    severity: EncounterSeverity,
    warning: Option<&WarningView>,
    enabled: bool,
    preview: bool,
) -> Element {
    let (name, width, height, top, font_size, color) = match severity {
        EncounterSeverity::High => (
            "CriticalEncounterWarnings",
            600.0,
            48.0,
            40.0,
            34.0,
            [1.0, 0.2, 0.1],
        ),
        EncounterSeverity::Medium => (
            "MediumEncounterWarnings",
            550.0,
            36.0,
            90.0,
            28.0,
            [1.0, 0.65, 0.1],
        ),
        EncounterSeverity::Low => (
            "MinorEncounterWarnings",
            500.0,
            36.0,
            130.0,
            20.0,
            [1.0, 0.9, 0.2],
        ),
    };
    let hidden = !preview && (!enabled || warning.is_none());
    let text = warning.map_or("Encounter Warning", |w| w.name.as_str());
    let icon = warning.map_or(0, |w| w.icon);
    rsx! { r#frame {
        name: {DynName(name.into())}, width, height, hidden, mouse_enabled: false,
        strata: FrameStrata::High, pos_type: "absolute", left: "50%", translate_x: "-50%", top,
        fontstring { name: {DynName(format!("{name}Text"))}, text, width: {width-110.0}, height,
            font: GameFont::FrizQuadrata, font_size, justify_h: "CENTER",
            font_color: {FontColor::new(color[0],color[1],color[2],1.0)},
            pos_type: "absolute", left: 55.0, top: 0.0 }
        texture { name: {DynName(format!("{name}LeftIcon"))}, texture_fdid: icon,
            width: height, height, pos_type: "absolute", left: 0.0, top: 0.0 }
        texture { name: {DynName(format!("{name}RightIcon"))}, texture_fdid: icon,
            width: height, height, pos_type: "absolute", left: {width-height}, top: 0.0 }
    } }
}
