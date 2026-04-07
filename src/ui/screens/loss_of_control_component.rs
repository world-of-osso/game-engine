use ui_toolkit::rsx;
use ui_toolkit::screen::SharedContext;
use ui_toolkit::widget_def::Element;

use crate::ui::anchor::AnchorPoint;

pub const FRAME_W: f32 = 200.0;
pub const FRAME_H: f32 = 100.0;
const ICON_SIZE: f32 = 48.0;
const NAME_H: f32 = 16.0;
const BAR_W: f32 = 160.0;
const BAR_H: f32 = 14.0;
const BAR_GAP: f32 = 4.0;

const ICON_BG: &str = "0.1,0.1,0.1,0.9";
const NAME_COLOR: &str = "1.0,0.3,0.3,1.0";
const BAR_BG: &str = "0.15,0.15,0.15,0.9";
const BAR_FILL: &str = "0.8,0.0,0.0,1.0";
const DURATION_COLOR: &str = "1.0,1.0,1.0,1.0";

#[derive(Clone, Debug, PartialEq)]
pub struct LossOfControlState {
    pub visible: bool,
    pub ability_name: String,
    pub icon_fdid: u32,
    pub duration_remaining: f32,
    pub duration_total: f32,
}

impl Default for LossOfControlState {
    fn default() -> Self {
        Self {
            visible: false,
            ability_name: String::new(),
            icon_fdid: 0,
            duration_remaining: 0.0,
            duration_total: 0.0,
        }
    }
}

impl LossOfControlState {
    fn progress(&self) -> f32 {
        if self.duration_total <= 0.0 {
            return 0.0;
        }
        (self.duration_remaining / self.duration_total).clamp(0.0, 1.0)
    }

    fn duration_text(&self) -> String {
        let secs = self.duration_remaining.ceil() as u32;
        format!("{secs}s")
    }
}

pub fn loss_of_control_screen(ctx: &SharedContext) -> Element {
    let state = ctx
        .get::<LossOfControlState>()
        .expect("LossOfControlState must be in SharedContext");
    let hide = !state.visible;
    let fill_w = BAR_W * state.progress();
    let duration_text = state.duration_text();
    rsx! {
        r#frame {
            name: "LossOfControlFrame",
            width: {FRAME_W},
            height: {FRAME_H},
            hidden: hide,
            anchor {
                point: AnchorPoint::Center,
                relative_point: AnchorPoint::Center,
                x: "0",
                y: "60",
            }
            r#frame {
                name: "LossOfControlIcon",
                width: {ICON_SIZE},
                height: {ICON_SIZE},
                background_color: ICON_BG,
                anchor {
                    point: AnchorPoint::Top,
                    relative_point: AnchorPoint::Top,
                    x: "0",
                    y: "0",
                }
            }
            fontstring {
                name: "LossOfControlName",
                width: {FRAME_W},
                height: {NAME_H},
                text: {state.ability_name.as_str()},
                font_size: 12.0,
                font_color: NAME_COLOR,
                justify_h: "CENTER",
                anchor {
                    point: AnchorPoint::Top,
                    relative_point: AnchorPoint::Top,
                    x: "0",
                    y: {-(ICON_SIZE + BAR_GAP)},
                }
            }
            r#frame {
                name: "LossOfControlBarBg",
                width: {BAR_W},
                height: {BAR_H},
                background_color: BAR_BG,
                anchor {
                    point: AnchorPoint::Top,
                    relative_point: AnchorPoint::Top,
                    x: "0",
                    y: {-(ICON_SIZE + BAR_GAP + NAME_H + BAR_GAP)},
                }
                r#frame {
                    name: "LossOfControlBarFill",
                    width: {fill_w},
                    height: {BAR_H},
                    background_color: BAR_FILL,
                    anchor {
                        point: AnchorPoint::TopLeft,
                        relative_point: AnchorPoint::TopLeft,
                    }
                }
                fontstring {
                    name: "LossOfControlDuration",
                    width: {BAR_W},
                    height: {BAR_H},
                    text: {duration_text.as_str()},
                    font_size: 9.0,
                    font_color: DURATION_COLOR,
                    justify_h: "CENTER",
                    anchor {
                        point: AnchorPoint::TopLeft,
                        relative_point: AnchorPoint::TopLeft,
                    }
                }
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

    fn make_state(remaining: f32, total: f32) -> LossOfControlState {
        LossOfControlState {
            visible: true,
            ability_name: "Hammer of Justice".into(),
            icon_fdid: 12345,
            duration_remaining: remaining,
            duration_total: total,
        }
    }

    fn build_registry(remaining: f32, total: f32) -> FrameRegistry {
        let mut reg = FrameRegistry::new(1920.0, 1080.0);
        let mut shared = SharedContext::new();
        shared.insert(make_state(remaining, total));
        Screen::new(loss_of_control_screen).sync(&shared, &mut reg);
        reg
    }

    fn layout_reg(remaining: f32, total: f32) -> FrameRegistry {
        let mut reg = build_registry(remaining, total);
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
        let reg = build_registry(3.0, 6.0);
        assert!(reg.get_by_name("LossOfControlFrame").is_some());
        assert!(reg.get_by_name("LossOfControlIcon").is_some());
        assert!(reg.get_by_name("LossOfControlName").is_some());
        assert!(reg.get_by_name("LossOfControlBarBg").is_some());
        assert!(reg.get_by_name("LossOfControlBarFill").is_some());
        assert!(reg.get_by_name("LossOfControlDuration").is_some());
    }

    #[test]
    fn hidden_when_not_visible() {
        let mut reg = FrameRegistry::new(1920.0, 1080.0);
        let mut shared = SharedContext::new();
        shared.insert(LossOfControlState::default());
        Screen::new(loss_of_control_screen).sync(&shared, &mut reg);
        let id = reg.get_by_name("LossOfControlFrame").expect("frame");
        assert!(reg.get(id).expect("data").hidden);
    }

    #[test]
    fn fill_width_matches_progress() {
        let reg = build_registry(3.0, 6.0);
        let id = reg.get_by_name("LossOfControlBarFill").expect("fill");
        let frame = reg.get(id).expect("data");
        assert_eq!(frame.width, Dimension::Fixed(BAR_W * 0.5));
    }

    // --- Coord validation ---

    #[test]
    fn coord_frame_centered() {
        let reg = layout_reg(3.0, 6.0);
        let r = rect(&reg, "LossOfControlFrame");
        let expected_x = (1920.0 - FRAME_W) / 2.0;
        assert!((r.x - expected_x).abs() < 1.0);
        assert!((r.width - FRAME_W).abs() < 1.0);
    }

    #[test]
    fn coord_icon_dimensions() {
        let reg = layout_reg(3.0, 6.0);
        let r = rect(&reg, "LossOfControlIcon");
        assert!((r.width - ICON_SIZE).abs() < 1.0);
        assert!((r.height - ICON_SIZE).abs() < 1.0);
    }

    #[test]
    fn coord_bar_dimensions() {
        let reg = layout_reg(3.0, 6.0);
        let r = rect(&reg, "LossOfControlBarBg");
        assert!((r.width - BAR_W).abs() < 1.0);
        assert!((r.height - BAR_H).abs() < 1.0);
    }
}
