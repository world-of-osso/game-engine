use bevy::prelude::Val;
use ui_toolkit::frame::{Dimension, Frame};
use ui_toolkit::registry::FrameRegistry;

use super::part_widths;

const PARTS: [&str; 3] = ["Left", "Center", "Right"];

/// Taffy rounds absolute children independently, so derive slice widths from
/// the button's physical edges rather than rounding each child offset.
fn snapped_part_widths(button: &Frame, screen_width: f32, scale: f32) -> [f32; 3] {
    let width = button.width.value();
    let [left, _, right] = part_widths(width, button.height.value());
    let x = match (button.position.left, button.position.right) {
        (Val::Px(left), _) => left,
        (_, Val::Px(right)) => screen_width - right - width,
        _ => panic!("navigation button must be anchored to a screen side"),
    };
    let total_pixels = ((x + width) * scale).round() - (x * scale).round();
    let left_pixels = (left * scale).round();
    let right_pixels = (right * scale).round();
    [
        left_pixels,
        total_pixels - left_pixels - right_pixels,
        right_pixels,
    ]
    .map(|pixels| pixels / scale)
}

pub fn snap_navigation_parts(registry: &mut FrameRegistry, scale: f32) {
    for name in ["CharCreateBack", "CharCreateNext", "CharCreateButton"] {
        let Some(button_id) = registry.get_by_name(name) else {
            continue;
        };
        let button = registry.get(button_id).expect("named navigation button");
        let widths = snapped_part_widths(button, registry.screen_width, scale);
        let offsets = [0.0, widths[0], widths[0] + widths[1]];
        for ((part, part_width), x) in PARTS.into_iter().zip(widths).zip(offsets) {
            let part_name = format!("{name}_{part}");
            let Some(id) = registry.get_by_name(&part_name) else {
                eprintln!("Character-creation navigation art missing part {part_name}");
                continue;
            };
            let frame = registry.get_mut(id).expect("named navigation part");
            let snapped_width = Dimension::Fixed(part_width);
            let snapped_left = Val::Px(x);
            if frame.width != snapped_width || frame.position.left != snapped_left {
                frame.width = snapped_width;
                frame.position.left = snapped_left;
                registry.mark_rect_dirty(id);
            }
        }
    }
}
