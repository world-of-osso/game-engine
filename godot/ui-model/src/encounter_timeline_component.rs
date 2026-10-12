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

#[cfg(test)]
mod tests {
    use super::*;
    use shared::protocol::{EncounterTimelineEvent, EncounterTimer};
    use ui_toolkit::{frame::WidgetData, registry::FrameRegistry, screen::Screen};
    #[test]
    fn timeline_screen_both_skins_countdown_text_track_and_warning_visibility() {
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
            .take(10)
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
        name: "EncounterTimeline", width: 300.0, height: {if view.bars {300.0} else {55.0}},
        hidden, mouse_enabled: false, strata: FrameStrata::High,
        pos_type: "absolute", left: 110.0, top: 280.0,
        {content}
    } };
    timeline.into_iter().chain(warnings).collect()
}

fn timer_bar(index: usize, timer: &TimerView) -> Element {
    let fraction =
        (timer.remaining * 1000.0 / timer.timer.duration_ms.max(1) as f32).clamp(0.0, 1.0);
    let width = 224.0 * fraction;
    let text = format!(
        "{}{}",
        timer.name,
        if timer.timer.conditional { " *" } else { "" }
    );
    let seconds = format!("{:.1}", timer.remaining);
    let color = bar_color(timer.remaining <= 5.0);
    rsx! { r#frame {
        name: {DynName(format!("EncounterTimer{}",index+1))}, width: 260.0, height: 34.0,
        pos_type: "absolute", left: 20.0, top: {266.0-index as f32*36.0}, mouse_enabled: false,
        texture { name: {DynName(format!("EncounterTimer{}Icon",index+1))},
            texture_fdid: {timer.icon}, width: 34.0, height: 34.0, pos_type: "absolute", left: 0.0, top: 0.0 }
        r#frame { width: 224.0, height: 26.0, pos_type: "absolute", left: 36.0, top: 4.0,
            background_color: "0.04,0.04,0.04,0.9",
            r#frame { name: {DynName(format!("EncounterTimer{}Fill",index+1))}, width, height: 26.0,
                background_color: {color.as_str()}, pos_type: "absolute", left: 0.0, top: 0.0 }
            fontstring { name: {DynName(format!("EncounterTimer{}Name",index+1))}, text: {text.as_str()},
                width: 174.0, height: 26.0, font: GameFont::FrizQuadrata, font_size: 13.0,
                font_color: {FontColor::new(1.0,1.0,1.0,1.0)}, justify_h: "LEFT",
                pos_type: "absolute", left: 4.0, top: 0.0, frame_level: 5.0 }
            fontstring { name: {DynName(format!("EncounterTimer{}Duration",index+1))}, text: {seconds.as_str()},
                width: 42.0, height: 26.0, font: GameFont::FrizQuadrata, font_size: 13.0,
                font_color: {FontColor::new(1.0,1.0,1.0,1.0)}, justify_h: "RIGHT",
                pos_type: "absolute", left: 178.0, top: 0.0, frame_level: 5.0 }
        }
    } }
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
