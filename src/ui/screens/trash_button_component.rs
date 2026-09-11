use std::fmt::Display;

use ui_toolkit::rsx;
use ui_toolkit::screen::SharedContext;
use ui_toolkit::widget_def::Element;

use crate::ui::anchor::FrameName;
use crate::ui::strata::FrameStrata;

const BUTTON_ATLAS_UP: &str = "defaultbutton-nineslice-up";
const BUTTON_ATLAS_PRESSED: &str = "defaultbutton-nineslice-pressed";
const BUTTON_ATLAS_HIGHLIGHT: &str = "defaultbutton-nineslice-highlight";
const BUTTON_ATLAS_DISABLED: &str = "defaultbutton-nineslice-disabled";
const DELETE_ICON_FILE: &str = "data/ui/delete-trash-icon-gold.ktx2";

pub const TRASH_BUTTON_ROOT: FrameName = FrameName("TrashButtonRoot");
pub const TRASH_BUTTON: FrameName = FrameName("TrashButton");
pub const TRASH_BUTTON_ICON: FrameName = FrameName("TrashButtonIcon");

/// Distances inward from the enclosing parent's right and bottom edges.
pub struct ButtonPosition {
    pub right: f32,
    pub bottom: f32,
}

pub fn trash_icon_button(
    name: FrameName,
    icon_name: FrameName,
    onclick: impl Display,
    position: ButtonPosition,
) -> Element {
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
            pos_type: "absolute",
            right: position.right,
            bottom: position.bottom,
            {trash_icon_texture(icon_name)}
        }
    }
}

fn trash_icon_texture(icon_name: FrameName) -> Element {
    rsx! {
        texture {
            name: icon_name,
            width: 24.0,
            height: 24.0,
            frame_level: 100.0,
            texture_file: DELETE_ICON_FILE,
            pos_type: "absolute",
            left: "50%",
            top: "50%",
            translate_x: "-50%",
            translate_y: "-50%",
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
            r#frame {
                name: "TrashButtonMount",
                width: 46.0,
                height: 42.0,
                pos_type: "absolute",
                left: "50%",
                top: "50%",
                translate_x: "-50%",
                translate_y: "-50%",
                {trash_icon_button(TRASH_BUTTON, TRASH_BUTTON_ICON, "noop", ButtonPosition { right: 0.0, bottom: 0.0 })}
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ui::screens::menu_character_layout_test_support::compute_layout;
    use ui_toolkit::registry::FrameRegistry;
    use ui_toolkit::screen::Screen;

    fn nested_button(_ctx: &SharedContext) -> Element {
        rsx! {
            r#frame {
                name: TRASH_BUTTON_ROOT,
                width: 300.0,
                height: 200.0,
                r#frame {
                    name: "TrashButtonParent",
                    width: 200.0,
                    height: 120.0,
                    {trash_icon_button(TRASH_BUTTON, TRASH_BUTTON_ICON, "trash", ButtonPosition { right: 18.0, bottom: 64.0 })}
                }
            }
        }
    }

    #[test]
    fn trash_button_and_icon_follow_actual_nested_parent() {
        let mut registry = FrameRegistry::new(1920.0, 1080.0);
        Screen::new(nested_button).sync(&SharedContext::new(), &mut registry);
        compute_layout(&mut registry);
        let parent = registry.get_by_name("TrashButtonParent").unwrap();
        let button_id = registry.get_by_name(TRASH_BUTTON.0).unwrap();
        let button = registry.get(button_id).unwrap();
        assert_eq!(button.parent_id, Some(parent));
        assert_eq!(button.onclick.as_deref(), Some("trash"));
        let parent_rect = registry.get(parent).unwrap().layout_rect.as_ref().unwrap();
        let button_rect = button.layout_rect.as_ref().unwrap();
        assert!(
            (parent_rect.x + parent_rect.width - button_rect.x - button_rect.width - 18.0).abs()
                < 0.51
        );
        assert!(
            (parent_rect.y + parent_rect.height - button_rect.y - button_rect.height - 64.0).abs()
                < 0.51
        );
        let icon = registry
            .get(registry.get_by_name(TRASH_BUTTON_ICON.0).unwrap())
            .unwrap();
        assert_eq!(icon.parent_id, Some(button_id));
        let icon_rect = icon.layout_rect.as_ref().unwrap();
        assert!(
            (icon_rect.x + icon_rect.width / 2.0 - button_rect.x - button_rect.width / 2.0).abs()
                < 0.51
        );
        assert!(
            (icon_rect.y + icon_rect.height / 2.0 - button_rect.y - button_rect.height / 2.0).abs()
                < 0.51
        );
    }

    #[test]
    fn standalone_trash_button_preserves_noop_action_and_centered_mount() {
        let mut registry = FrameRegistry::new(1920.0, 1080.0);
        Screen::new(trash_button_screen).sync(&SharedContext::new(), &mut registry);
        compute_layout(&mut registry);
        let mount = registry.get_by_name("TrashButtonMount").unwrap();
        let button = registry
            .get(registry.get_by_name(TRASH_BUTTON.0).unwrap())
            .unwrap();
        assert_eq!(button.parent_id, Some(mount));
        assert_eq!(button.onclick.as_deref(), Some("noop"));
        let rect = button.layout_rect.as_ref().unwrap();
        assert!((rect.x + rect.width / 2.0 - 960.0).abs() < 0.51);
        assert!((rect.y + rect.height / 2.0 - 540.0).abs() < 0.51);
    }
}
