//! Retail CastingBarFrame.xml feedback sampled on the cast reducer's clock.
use ui_toolkit::frame::WidgetData;
use ui_toolkit::registry::FrameRegistry;
use ui_toolkit::rsx;
use ui_toolkit::widget_def::Element;
use ui_toolkit::widgets::texture::BlendMode;

use super::{CastBarStyle, CastingBarState};

/// Player-only feedback; target/nameplate presentation remains unchanged.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum CastFeedback {
    Casting,
    Finished(f32),
    Interrupted(f32),
}

impl CastFeedback {
    pub fn spark_visible(self) -> bool {
        match self {
            Self::Casting => true,
            Self::Finished(_) => false,
            Self::Interrupted(seconds) => seconds < 0.1,
        }
    }

    /// XML translations are cumulative and use y-up, unlike the UI canvas.
    pub fn shake_offset(self) -> (f32, f32) {
        let Self::Interrupted(seconds) = self else {
            return (0.0, 0.0);
        };
        match seconds {
            t if t < 0.15 => (0.0, 0.0),
            t if t < 0.20 => (-1.0, -1.0),
            t if t < 0.25 => (0.0, 1.0),
            t if t < 0.30 => (1.0, -1.0),
            _ => (0.0, 0.0),
        }
    }
}

struct Sprite {
    name: &'static str,
    atlas: &'static str,
    rect: (f32, f32, f32, f32),
    alpha: f32,
    rotation: f32,
}

fn sprite(style: &CastBarStyle, part: Sprite) -> Element {
    let (x, y, width, height) = part.rect;
    let hide = part.alpha <= 0.0;
    rsx! { texture {
        name: {style.name(part.name)}, texture_atlas: part.atlas,
        width, height, alpha: {part.alpha}, hidden: hide,
        rotation: {part.rotation}, pos_type: "absolute", pos_x: x, pos_y: y,
    } }
}

pub(super) fn feedback_art(style: &CastBarStyle, state: &CastingBarState) -> Element {
    match state.player_feedback {
        Some(CastFeedback::Finished(seconds)) => finish_art(style, state, seconds),
        Some(CastFeedback::Interrupted(seconds)) => sprite(
            style,
            Sprite {
                name: "InterruptGlow",
                atlas: "cast_interrupt_outerglow",
                rect: (-5.0, (style.bar.1 - 25.0) / 2.0, style.bar.0 + 10.0, 25.0),
                alpha: (1.0 - seconds).clamp(0.0, 1.0),
                rotation: 0.0,
            },
        ),
        _ => Element::default(),
    }
}

fn finish_art(style: &CastBarStyle, state: &CastingBarState, seconds: f32) -> Element {
    let atlas = if state.is_channel && state.is_interruptible {
        "ui-castingbar-full-glow-channel"
    } else {
        "ui-castingbar-full-glow-standard"
    };
    let flash = sprite(
        style,
        Sprite {
            name: "Flash",
            atlas,
            rect: (-1.0, -1.0, style.bar.0 + 2.0, style.bar.1 + 2.0),
            alpha: (seconds / 0.2).clamp(0.0, 1.0),
            rotation: 0.0,
        },
    );
    let finish = if !state.is_interruptible {
        Element::default()
    } else if state.is_channel {
        channel_finish(style, seconds)
    } else {
        standard_finish(style, seconds)
    };
    flash.into_iter().chain(finish).collect()
}

fn standard_finish(style: &CastBarStyle, seconds: f32) -> Element {
    // XML: StandardFinish repeats every 0.75s (Flakes03 delay 0.25 + 0.5).
    let t = seconds % 0.75;
    let linear = (t / 0.5).clamp(0.0, 1.0);
    let late = ((t - 0.25) / 0.5).clamp(0.0, 1.0);
    let center = style.bar.0 / 2.0;
    [
        Sprite {
            name: "EnergyGlow",
            atlas: "cast_standard_glowline",
            rect: (
                center - 123.165,
                style.bar.1 / 2.0 + 100.0 - 220.0 * linear - 62.22,
                246.33,
                124.44,
            ),
            alpha: 1.0,
            rotation: 0.0,
        },
        Sprite {
            name: "Flakes01",
            atlas: "cast_standard_flakes01",
            rect: (
                center - 54.75,
                style.bar.1 / 2.0 + 25.0 - 100.0 * linear * linear - 11.5,
                109.5,
                23.0,
            ),
            alpha: 1.0,
            rotation: 0.0,
        },
        Sprite {
            name: "Flakes02",
            atlas: "cast_standard_flakes02",
            rect: (
                center - 64.0,
                style.bar.1 / 2.0 + 45.0 - 90.0 * (1.0 - (1.0 - linear).powi(2)) - 18.5,
                128.0,
                37.0,
            ),
            alpha: 1.0,
            rotation: 0.0,
        },
        Sprite {
            name: "Flakes03",
            atlas: "cast_standard_flakes03",
            rect: (
                center - 51.25,
                style.bar.1 / 2.0 + 25.0 - 25.0 * late * late - 8.75,
                102.5,
                17.5,
            ),
            alpha: 1.0,
            rotation: 0.0,
        },
    ]
    .into_iter()
    .flat_map(|part| sprite(style, part))
    .collect()
}

fn channel_finish(style: &CastBarStyle, seconds: f32) -> Element {
    let linear = (seconds / 0.5).clamp(0.0, 1.0);
    let late = ((seconds - 0.1) / 0.5).clamp(0.0, 1.0);
    let base_alpha = if seconds < 0.6 {
        (seconds / 0.3).clamp(0.0, 1.0) * 0.5
    } else {
        (1.0 - (seconds - 0.6) / 0.2).clamp(0.0, 1.0) * 0.5
    };
    // Order 2 begins at 0.6s: the delayed Sparkles02 fixes order 1's duration.
    let alpha = (1.0 - (seconds - 0.6) / 0.5).clamp(0.0, 1.0);
    let middle = style.bar.1 / 2.0;
    [
        Sprite {
            name: "BaseGlow",
            atlas: "channel_wispglow2",
            rect: (0.0, middle - 5.5, 21.0 * (1.0 + 3.0 * linear), 11.0),
            alpha: base_alpha,
            rotation: 0.0,
        },
        Sprite {
            name: "WispMask",
            atlas: "cast_channel_wispmask",
            rect: (-90.0 + 50.0 * linear, middle - 17.0, 54.5, 38.0),
            alpha: 0.0,
            rotation: 0.0,
        },
        Sprite {
            name: "WispGlow",
            atlas: "cast_channel_wispglow",
            rect: (25.0, middle - 8.0, 52.0, 16.0),
            alpha,
            rotation: 0.0,
        },
        Sprite {
            name: "Sparkles01",
            atlas: "cast_channel_sparkles_01",
            rect: (40.0 * linear * linear, middle - 3.75, 24.0, 7.5),
            alpha,
            rotation: linear * std::f32::consts::FRAC_PI_4,
        },
        Sprite {
            name: "Sparkles02",
            atlas: "cast_channel_sparkles_02",
            rect: (20.0 * ease_in_out(late), middle - 4.25, 32.5, 8.5),
            alpha,
            rotation: -late * std::f32::consts::FRAC_PI_4,
        },
    ]
    .into_iter()
    .flat_map(|part| sprite(style, part))
    .collect()
}

fn ease_in_out(t: f32) -> f32 {
    if t < 0.5 {
        2.0 * t * t
    } else {
        1.0 - 2.0 * (1.0 - t).powi(2)
    }
}

pub(super) fn player_spark(style: &CastBarStyle, state: &CastingBarState, fill_w: f32) -> Element {
    let Some(feedback) = state.player_feedback else {
        return Element::default();
    };
    if !feedback.spark_visible() {
        return Element::default();
    }
    let interrupted = matches!(feedback, CastFeedback::Interrupted(_));
    let atlas = if interrupted {
        "ui-castingbar-pip-red"
    } else {
        "ui-castingbar-pip"
    };
    let pip = sprite(
        style,
        Sprite {
            name: "Spark",
            atlas,
            rect: (fill_w - 4.0, (style.bar.1 - 20.0) / 2.0, 8.0, 20.0),
            alpha: 1.0,
            rotation: 0.0,
        },
    );
    let glow = if interrupted || !state.is_interruptible {
        Element::default()
    } else if state.is_channel {
        sprite(
            style,
            Sprite {
                name: "ChannelShadow",
                atlas: "cast_channel_pipshadow",
                rect: (fill_w - 14.0, (style.bar.1 - 11.0) / 2.0, 11.0, 11.0),
                alpha: 1.0,
                rotation: 0.0,
            },
        )
    } else {
        sprite(
            style,
            Sprite {
                name: "StandardGlow",
                atlas: "cast_standard_pipglow",
                rect: (fill_w - 39.0, (style.bar.1 - 12.0) / 2.0, 37.0, 12.0),
                alpha: 1.0,
                rotation: 0.0,
            },
        )
    };
    pip.into_iter().chain(glow).collect()
}

/// Retail's ADD layers are properties, not RSX attributes.
pub fn apply_casting_bar_feedback_postsetup(registry: &mut FrameRegistry) {
    for suffix in [
        "Flash",
        "InterruptGlow",
        "EnergyGlow",
        "Flakes01",
        "Flakes02",
        "Flakes03",
        "BaseGlow",
        "WispGlow",
        "StandardGlow",
    ] {
        let Some(id) = registry.get_by_name(&format!("CastingBar{suffix}")) else {
            continue;
        };
        if let Some(WidgetData::Texture(texture)) = registry
            .get_mut(id)
            .and_then(|frame| frame.widget_data.as_mut())
        {
            texture.blend_mode = BlendMode::Additive;
        }
    }
}
