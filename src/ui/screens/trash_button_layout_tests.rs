use super::menu_character_layout_test_support::compute_layout;
use super::trash_button_component::*;
use ui_toolkit::registry::FrameRegistry;
use ui_toolkit::rsx;
use ui_toolkit::screen::{Screen, SharedContext};
use ui_toolkit::widget_def::Element;

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
        (parent_rect.x + parent_rect.width - button_rect.x - button_rect.width - 18.0).abs() < 0.51
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
