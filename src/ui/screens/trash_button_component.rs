use std::fmt::Display;

use ui_toolkit::rsx;
use ui_toolkit::screen::SharedContext;
use ui_toolkit::widget_def::Element;

use crate::ui::anchor::{AnchorPoint, FrameName};
use crate::ui::strata::FrameStrata;

const BUTTON_ATLAS_UP: &str = "defaultbutton-nineslice-up";
const BUTTON_ATLAS_PRESSED: &str = "defaultbutton-nineslice-pressed";
const BUTTON_ATLAS_HIGHLIGHT: &str = "defaultbutton-nineslice-highlight";
const BUTTON_ATLAS_DISABLED: &str = "defaultbutton-nineslice-disabled";
const DELETE_ICON_FILE: &str = "data/ui/delete-trash-icon-gold.ktx2";

pub const TRASH_BUTTON_ROOT: FrameName = FrameName("TrashButtonRoot");
pub const TRASH_BUTTON: FrameName = FrameName("TrashButton");
pub const TRASH_BUTTON_ICON: FrameName = FrameName("TrashButtonIcon");

pub struct ButtonAnchor {
    pub point: AnchorPoint,
    pub relative_to: Option<FrameName>,
    pub relative_point: AnchorPoint,
    pub x: f32,
    pub y: f32,
}

pub fn trash_icon_button(
    name: FrameName,
    icon_name: FrameName,
    onclick: impl Display,
    anchor: ButtonAnchor,
) -> Element {
    let icon = trash_icon_texture(name, icon_name);
    let relative_to = anchor
        .relative_to
        .map_or_else(|| "$parent".to_string(), |frame| frame.to_string());
    rsx! {
        button {
            name,
            width: 46.0,
            height: 42.0,
            text: "",
            font_size: 14.0,
            onclick,
            button_atlas_up: BUTTON_ATLAS_UP,
            button_atlas_pressed: BUTTON_ATLAS_PRESSED,
            button_atlas_highlight: BUTTON_ATLAS_HIGHLIGHT,
            button_atlas_disabled: BUTTON_ATLAS_DISABLED,
            anchor {
                point: anchor.point,
                relative_to: {relative_to},
                relative_point: anchor.relative_point,
                x: {anchor.x.to_string()},
                y: {anchor.y.to_string()},
            }
            {icon}
        }
    }
}

fn trash_icon_texture(button_name: FrameName, icon_name: FrameName) -> Element {
    rsx! {
        texture {
            name: icon_name,
            width: 24.0,
            height: 24.0,
            frame_level: 100.0,
            texture_file: DELETE_ICON_FILE,
            anchor {
                point: AnchorPoint::Center,
                relative_to: button_name,
                relative_point: AnchorPoint::Center,
            }
        }
    }
}

pub fn trash_button_screen(_shared: &SharedContext) -> Element {
    rsx! {
        r#frame {
            name: TRASH_BUTTON_ROOT,
            stretch: true,
            background_color: "0.02,0.02,0.03,1.0",
            strata: FrameStrata::Background,
            {
                trash_icon_button(
                    TRASH_BUTTON,
                    TRASH_BUTTON_ICON,
                    "noop",
                    ButtonAnchor {
                        point: AnchorPoint::Center,
                        relative_to: None,
                        relative_point: AnchorPoint::Center,
                        x: 0.0,
                        y: 0.0,
                    },
                )
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use ui_toolkit::registry::FrameRegistry;
    use ui_toolkit::screen::Screen;

    const ANCHOR_TARGET: FrameName = FrameName("TrashAnchorTarget");

    #[derive(Clone)]
    struct TrashButtonTestState {
        with_relative_anchor: bool,
    }

    fn test_screen(ctx: &SharedContext) -> Element {
        let state = ctx
            .get::<TrashButtonTestState>()
            .expect("TrashButtonTestState must be in SharedContext");
        let target: Element = if state.with_relative_anchor {
            rsx! {
                r#frame {
                    name: ANCHOR_TARGET,
                    width: 64.0,
                    height: 32.0,
                    anchor { point: AnchorPoint::TopLeft, relative_point: AnchorPoint::TopLeft }
                }
            }
        } else {
            Vec::new()
        };
        let anchor = ButtonAnchor {
            point: AnchorPoint::BottomRight,
            relative_to: if state.with_relative_anchor {
                Some(ANCHOR_TARGET)
            } else {
                None
            },
            relative_point: AnchorPoint::BottomRight,
            x: -18.0,
            y: 64.0,
        };
        rsx! {
            r#frame {
                name: TRASH_BUTTON_ROOT,
                width: 300.0,
                height: 200.0,
                anchor { point: AnchorPoint::TopLeft, relative_point: AnchorPoint::TopLeft }
                {target}
                {trash_icon_button(TRASH_BUTTON, TRASH_BUTTON_ICON, "trash", anchor)}
            }
        }
    }

    fn build_registry(with_relative_anchor: bool) -> FrameRegistry {
        let mut registry = FrameRegistry::new(1920.0, 1080.0);
        let mut shared = SharedContext::new();
        shared.insert(TrashButtonTestState {
            with_relative_anchor,
        });
        Screen::new(test_screen).sync(&shared, &mut registry);
        registry
    }

    #[test]
    fn trash_button_anchor_without_relative_to_uses_screen_space_anchor() {
        let registry = build_registry(false);
        let root_id = registry
            .get_by_name(TRASH_BUTTON_ROOT.0)
            .expect("TrashButtonRoot frame");

        let button = registry
            .get(
                registry
                    .get_by_name(TRASH_BUTTON.0)
                    .expect("TrashButton frame"),
            )
            .expect("TrashButton data");
        assert_eq!(button.anchors.len(), 1);
        assert_eq!(button.anchors[0].relative_to, Some(root_id));
    }

    #[test]
    fn trash_button_anchor_with_relative_to_points_at_target_and_icon_points_to_button() {
        let registry = build_registry(true);

        let target_id = registry
            .get_by_name(ANCHOR_TARGET.0)
            .expect("TrashAnchorTarget frame");
        let button_id = registry
            .get_by_name(TRASH_BUTTON.0)
            .expect("TrashButton frame");
        let button = registry.get(button_id).expect("TrashButton data");
        assert_eq!(button.anchors.len(), 1);
        assert_eq!(button.anchors[0].relative_to, Some(target_id));

        let icon = registry
            .get(
                registry
                    .get_by_name(TRASH_BUTTON_ICON.0)
                    .expect("TrashButtonIcon frame"),
            )
            .expect("TrashButtonIcon data");
        assert_eq!(icon.anchors.len(), 1);
        assert_eq!(icon.anchors[0].relative_to, Some(button_id));
    }
}
