use ui_toolkit::rsx;
use ui_toolkit::screen::SharedContext;
use ui_toolkit::widget_def::Element;

use crate::ui::anchor::AnchorPoint;

pub const BAR_W: f32 = 195.0;
pub const BAR_H: f32 = 20.0;
const BORDER_W: f32 = BAR_W + 8.0;
const BORDER_H: f32 = BAR_H + 8.0;
const SPARK_W: f32 = 8.0;
const TEXT_H: f32 = 14.0;
const TIMER_W: f32 = 40.0;

const BORDER_BG: &str = "0.0,0.0,0.0,0.8";
const BAR_BG: &str = "0.15,0.15,0.15,0.9";
const FILL_CAST: &str = "1.0,0.7,0.0,1.0";
const FILL_CHANNEL: &str = "0.0,0.64,0.0,1.0";
const FILL_UNINTERRUPTIBLE: &str = "0.63,0.63,0.63,1.0";
const SPARK_COLOR: &str = "1.0,1.0,1.0,0.8";
const SPELL_NAME_COLOR: &str = "1.0,1.0,1.0,1.0";
const TIMER_COLOR: &str = "1.0,1.0,1.0,1.0";

#[derive(Clone, Debug, PartialEq)]
pub struct CastingBarState {
    pub visible: bool,
    pub spell_name: String,
    pub timer_text: String,
    /// Fill fraction 0.0..=1.0.
    pub progress: f32,
    pub is_channel: bool,
    pub is_interruptible: bool,
}

impl Default for CastingBarState {
    fn default() -> Self {
        Self {
            visible: false,
            spell_name: String::new(),
            timer_text: String::new(),
            progress: 0.0,
            is_channel: false,
            is_interruptible: true,
        }
    }
}

pub fn casting_bar_frame_screen(ctx: &SharedContext) -> Element {
    let state = ctx
        .get::<CastingBarState>()
        .expect("CastingBarState must be in SharedContext");
    let hide = !state.visible;
    let fill_w = BAR_W * state.progress.clamp(0.0, 1.0);
    let fill_color = bar_fill_color(state.is_channel, state.is_interruptible);
    let spark_x = fill_w - SPARK_W / 2.0;
    rsx! {
        r#frame {
            name: "CastingBarFrame",
            width: {BORDER_W},
            height: {BORDER_H},
            background_color: BORDER_BG,
            hidden: hide,
            anchor {
                point: AnchorPoint::Bottom,
                relative_point: AnchorPoint::Bottom,
                x: "0",
                y: "150",
            }
            {bar_background()}
            {fill_bar(fill_w, fill_color)}
            {spark(spark_x)}
            {spell_name_text(&state.spell_name)}
            {timer_text(&state.timer_text)}
        }
    }
}

fn bar_fill_color(is_channel: bool, is_interruptible: bool) -> &'static str {
    if !is_interruptible {
        FILL_UNINTERRUPTIBLE
    } else if is_channel {
        FILL_CHANNEL
    } else {
        FILL_CAST
    }
}

fn bar_background() -> Element {
    rsx! {
        r#frame {
            name: "CastingBarBackground",
            width: {BAR_W},
            height: {BAR_H},
            background_color: BAR_BG,
            anchor {
                point: AnchorPoint::Center,
                relative_point: AnchorPoint::Center,
            }
        }
    }
}

fn fill_bar(fill_w: f32, color: &str) -> Element {
    rsx! {
        r#frame {
            name: "CastingBarFill",
            width: {fill_w},
            height: {BAR_H},
            background_color: color,
            anchor {
                point: AnchorPoint::Left,
                relative_to: "CastingBarBackground",
                relative_point: AnchorPoint::Left,
            }
        }
    }
}

fn spark(x: f32) -> Element {
    rsx! {
        r#frame {
            name: "CastingBarSpark",
            width: {SPARK_W},
            height: {BAR_H + 6.0},
            background_color: SPARK_COLOR,
            anchor {
                point: AnchorPoint::Left,
                relative_to: "CastingBarBackground",
                relative_point: AnchorPoint::Left,
                x: {x},
                y: "0",
            }
        }
    }
}

fn spell_name_text(name: &str) -> Element {
    rsx! {
        fontstring {
            name: "CastingBarSpellName",
            width: {BAR_W - TIMER_W},
            height: {TEXT_H},
            text: name,
            font_size: 10.0,
            font_color: SPELL_NAME_COLOR,
            justify_h: "CENTER",
            anchor {
                point: AnchorPoint::Center,
                relative_to: "CastingBarBackground",
                relative_point: AnchorPoint::Center,
            }
        }
    }
}

fn timer_text(timer: &str) -> Element {
    rsx! {
        fontstring {
            name: "CastingBarTimer",
            width: {TIMER_W},
            height: {TEXT_H},
            text: timer,
            font_size: 9.0,
            font_color: TIMER_COLOR,
            justify_h: "RIGHT",
            anchor {
                point: AnchorPoint::Right,
                relative_to: "CastingBarBackground",
                relative_point: AnchorPoint::Right,
                x: "-2",
                y: "0",
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use ui_toolkit::frame::Dimension;
    use ui_toolkit::layout::{LayoutRect, recompute_layouts};
    use ui_toolkit::registry::FrameRegistry;
    use ui_toolkit::screen::{Screen, SharedContext};

    fn make_state(progress: f32) -> CastingBarState {
        CastingBarState {
            visible: true,
            spell_name: "Fireball".into(),
            timer_text: "1.5s".into(),
            progress,
            is_channel: false,
            is_interruptible: true,
        }
    }

    fn build_registry(progress: f32) -> FrameRegistry {
        let mut reg = FrameRegistry::new(1920.0, 1080.0);
        let mut shared = SharedContext::new();
        shared.insert(make_state(progress));
        Screen::new(casting_bar_frame_screen).sync(&shared, &mut reg);
        reg
    }

    fn layout_reg(progress: f32) -> FrameRegistry {
        let mut reg = build_registry(progress);
        recompute_layouts(&mut reg);
        reg
    }

    fn rect(reg: &FrameRegistry, name: &str) -> LayoutRect {
        reg.get(reg.get_by_name(name).expect(name))
            .and_then(|f| f.layout_rect.clone())
            .unwrap_or_else(|| panic!("{name} has no layout_rect"))
    }

    #[test]
    fn builds_all_elements() {
        let reg = build_registry(0.5);
        assert!(reg.get_by_name("CastingBarFrame").is_some());
        assert!(reg.get_by_name("CastingBarBackground").is_some());
        assert!(reg.get_by_name("CastingBarFill").is_some());
        assert!(reg.get_by_name("CastingBarSpark").is_some());
        assert!(reg.get_by_name("CastingBarSpellName").is_some());
        assert!(reg.get_by_name("CastingBarTimer").is_some());
    }

    #[test]
    fn hidden_when_not_visible() {
        let mut reg = FrameRegistry::new(1920.0, 1080.0);
        let mut shared = SharedContext::new();
        shared.insert(CastingBarState::default());
        Screen::new(casting_bar_frame_screen).sync(&shared, &mut reg);
        let id = reg.get_by_name("CastingBarFrame").expect("frame");
        assert!(reg.get(id).expect("data").hidden);
    }

    #[test]
    fn fill_width_matches_progress() {
        let reg = build_registry(0.6);
        let id = reg.get_by_name("CastingBarFill").expect("fill");
        let frame = reg.get(id).expect("data");
        let expected = BAR_W * 0.6;
        assert_eq!(frame.width, Dimension::Fixed(expected));
    }

    #[test]
    fn fill_color_changes_for_channel() {
        assert_eq!(bar_fill_color(false, true), FILL_CAST);
        assert_eq!(bar_fill_color(true, true), FILL_CHANNEL);
        assert_eq!(bar_fill_color(false, false), FILL_UNINTERRUPTIBLE);
        assert_eq!(bar_fill_color(true, false), FILL_UNINTERRUPTIBLE);
    }

    // --- Coord validation ---

    #[test]
    fn coord_frame_centered_bottom() {
        let reg = layout_reg(0.5);
        let r = rect(&reg, "CastingBarFrame");
        let expected_x = (1920.0 - BORDER_W) / 2.0;
        assert!((r.x - expected_x).abs() < 1.0);
        assert!((r.width - BORDER_W).abs() < 1.0);
        assert!((r.height - BORDER_H).abs() < 1.0);
    }

    #[test]
    fn coord_bar_background_dimensions() {
        let reg = layout_reg(0.5);
        let r = rect(&reg, "CastingBarBackground");
        assert!((r.width - BAR_W).abs() < 1.0);
        assert!((r.height - BAR_H).abs() < 1.0);
    }
}
