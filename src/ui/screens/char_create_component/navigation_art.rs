use bevy::prelude::*;
use ui_toolkit::rsx;
use ui_toolkit::widget_def::Element;

use crate::ui::frame::{Dimension, Frame, WidgetData};
use crate::ui::plugin::UiState;
use crate::ui::registry::FrameRegistry;
use crate::ui::widgets::button::ButtonState;
use crate::ui::widgets::font_string::{FontColor, GameFont, JustifyH};
use crate::ui::widgets::texture::TextureSource;

use super::{BACK_BUTTON, CREATE_BUTTON, DynName, NEXT_BUTTON};

const LEFT: &str = "128-redbutton-left";
const CENTER: &str = "_128-redbutton-center";
const RIGHT: &str = "128-redbutton-right";
const HIGHLIGHT: &str = "retail-128-redbutton-highlight";
const PARTS: [(&str, &str); 3] = [("Left", LEFT), ("Center", CENTER), ("Right", RIGHT)];

fn cap_width(name: &str, height: f32) -> f32 {
    let region = ui_toolkit::atlas::get_region(name)
        .unwrap_or_else(|| panic!("missing authored Retail navigation atlas {name}"));
    region.width * height / region.height
}

/// At the authored 250×66 size, left and right caps retain their source ratio;
/// only the narrow center strip is stretched to fit the remaining width.
fn part_widths(width: f32, height: f32) -> [f32; 3] {
    let left = cap_width(LEFT, height);
    let right = cap_width(RIGHT, height);
    assert!(
        width >= left + right,
        "navigation button is too narrow for authored caps"
    );
    [left, width - left - right, right]
}

pub(super) fn navigation_layers(name: &str, label: &str, width: f32, height: f32) -> Element {
    let [left_width, center_width, right_width] = part_widths(width, height);
    let center_x = left_width;
    let right_x = center_x + center_width;
    rsx! {
        texture { name: DynName(format!("{name}_Left")), width: left_width, height,
            texture_atlas: LEFT, pos_type: "absolute", left: 0.0, top: 0.0,
        }
        texture { name: DynName(format!("{name}_Center")), width: center_width, height,
            texture_atlas: CENTER, pos_type: "absolute", left: center_x, top: 0.0,
        }
        texture { name: DynName(format!("{name}_Right")), width: right_width, height,
            texture_atlas: RIGHT, pos_type: "absolute", left: right_x, top: 0.0,
        }
        texture { name: DynName(format!("{name}_Highlight")), width, height,
            texture_atlas: HIGHLIGHT, pos_type: "absolute", left: 0.0, top: 0.0,
            hidden: true,
        }
        fontstring { name: DynName(format!("{name}_Label")), width, height,
            text: label, font: GameFont::FrizQuadrata, font_size: 22.0,
            font_color: FontColor::new(1.0, 0.82, 0.0, 1.0),
            justify_h: JustifyH::Center,
            pos_type: "absolute", left: 0.0, top: 0.0,
        }
    }
}

/// Taffy rounds absolute children independently, so derive all slice widths
/// from the button's actual physical edges rather than rounding each offset.
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
    let center_pixels = total_pixels - left_pixels - right_pixels;
    [left_pixels, center_pixels, right_pixels].map(|pixels| pixels / scale)
}

fn snap_navigation_parts(registry: &mut FrameRegistry, name: &str, scale: f32) {
    let Some(button_id) = registry.get_by_name(name) else {
        return;
    };
    let button = registry.get(button_id).expect("named navigation button");
    let widths = snapped_part_widths(button, registry.screen_width, scale);
    let offsets = [0.0, widths[0], widths[0] + widths[1]];
    for ((part, _), (part_width, x)) in PARTS.into_iter().zip(widths.into_iter().zip(offsets)) {
        let part_name = format!("{name}_{part}");
        let Some(id) = registry.get_by_name(&part_name) else {
            warn!("Character-creation navigation art missing part {part_name}");
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

fn state_suffix(state: ButtonState) -> &'static str {
    match state {
        ButtonState::Normal => "",
        ButtonState::Pushed => "-pressed",
        ButtonState::Disabled => "-disabled",
    }
}

fn sync_part(registry: &mut FrameRegistry, name: &str, atlas: &str) {
    let Some(id) = registry.get_by_name(name) else {
        warn!("Character-creation navigation art missing part {name}");
        return;
    };
    let source = TextureSource::Atlas(atlas.to_string());
    let current = registry.get(id).and_then(|frame| match &frame.widget_data {
        Some(WidgetData::Texture(texture)) => Some(&texture.source),
        _ => None,
    });
    if current == Some(&source) {
        return;
    }
    if let Some(frame) = registry.get_mut(id)
        && let Some(WidgetData::Texture(texture)) = &mut frame.widget_data
    {
        texture.source = source;
    } else {
        warn!("Character-creation navigation part {name} is not a texture");
    }
}

fn sync_navigation_highlight(registry: &mut FrameRegistry, name: &str, visible: bool) {
    let highlight_name = format!("{name}_Highlight");
    let Some(highlight) = registry.get_by_name(&highlight_name) else {
        warn!("Character-creation navigation art missing highlight {highlight_name}");
        return;
    };
    let hidden = !visible;
    if registry
        .get(highlight)
        .is_some_and(|frame| frame.hidden != hidden)
    {
        registry.set_hidden(highlight, hidden);
    }
}

fn sync_navigation_label(registry: &mut FrameRegistry, name: &str, state: ButtonState) {
    let label_name = format!("{name}_Label");
    let Some(id) = registry.get_by_name(&label_name) else {
        warn!("Character-creation navigation art missing label {label_name}");
        return;
    };
    let color = match state {
        ButtonState::Normal => [1.0, 0.82, 0.0, 1.0],
        ButtonState::Pushed => [0.8, 0.65, 0.0, 1.0],
        ButtonState::Disabled => [0.5, 0.5, 0.5, 1.0],
    };
    let current = registry.get(id).and_then(|frame| match &frame.widget_data {
        Some(WidgetData::FontString(text)) => Some(text.color),
        _ => None,
    });
    if current == Some(color) {
        return;
    }
    if let Some(frame) = registry.get_mut(id)
        && let Some(WidgetData::FontString(text)) = &mut frame.widget_data
    {
        text.color = color;
    } else {
        warn!("Character-creation navigation label {label_name} is not text");
    }
}

fn sync_button(registry: &mut FrameRegistry, name: &str) {
    let Some(id) = registry.get_by_name(name) else {
        return;
    };
    let Some(WidgetData::Button(button)) = registry
        .get(id)
        .and_then(|frame| frame.widget_data.as_ref())
    else {
        warn!("Character-creation navigation control {name} is not a button");
        return;
    };
    let (state, show_highlight) = (
        button.state,
        button.state != ButtonState::Disabled && (button.hovered || button.highlight_locked),
    );
    let suffix = state_suffix(state);
    for (part, atlas) in PARTS {
        sync_part(
            registry,
            &format!("{name}_{part}"),
            &format!("{atlas}{suffix}"),
        );
    }
    sync_navigation_highlight(registry, name, show_highlight);
    sync_navigation_label(registry, name, state);
}

/// Keep only the three authored parts and hover overlay in sync with the live
/// button state; register before `UiRenderSet::Prepare` in the host scene.
pub fn sync_navigation_art(
    mut ui: ResMut<UiState>,
    windows: Query<&Window, With<bevy::window::PrimaryWindow>>,
) {
    for name in [BACK_BUTTON.0, NEXT_BUTTON.0, CREATE_BUTTON.0] {
        if let Some(window) = windows.iter().next() {
            snap_navigation_parts(&mut ui.registry, name, window.scale_factor());
        }
        sync_button(&mut ui.registry, name);
    }
}

#[cfg(test)]
#[path = "navigation_art_tests.rs"]
mod tests;
