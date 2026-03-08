use bevy::camera::visibility::RenderLayers;
use bevy::prelude::*;
use bevy::text::TextFont;
use std::collections::HashSet;

use crate::ui::frame::WidgetData;
use crate::ui::plugin::UiState;
use crate::ui::render::UI_RENDER_LAYER;
use crate::ui::render::UiText;
use crate::ui::widgets::button::ButtonState;
use crate::ui::widgets::font_string::JustifyH;

/// Syncs text content from the frame registry into Bevy Text2d entities.
pub fn sync_ui_text(
    state: Res<UiState>,
    mut commands: Commands,
    mut texts: Query<(
        Entity, &UiText, &mut Text2d, &mut TextFont, &mut TextColor, &mut Transform,
    )>,
) {
    let screen_w = state.registry.screen_width;
    let screen_h = state.registry.screen_height;
    let mut existing: HashSet<u64> = HashSet::new();

    for (entity, ui_text, mut text, mut font, mut color, mut transform) in texts.iter_mut() {
        let Some(frame) = state.registry.get(ui_text.0) else {
            commands.entity(entity).despawn();
            continue;
        };
        if !frame.visible || !has_text(frame) {
            commands.entity(entity).despawn();
            continue;
        }
        existing.insert(ui_text.0);
        let (content, font_size, text_color, justify) = extract_text_props(frame);
        *text = Text2d::new(content);
        font.font_size = font_size;
        *color = TextColor(text_color);
        *transform = text_transform(frame, screen_w, screen_h, justify);
    }

    spawn_missing_text(&state, &existing, screen_w, screen_h, &mut commands);
}

fn spawn_missing_text(
    state: &UiState,
    existing: &HashSet<u64>,
    screen_w: f32,
    screen_h: f32,
    commands: &mut Commands,
) {
    for frame in state.registry.frames_iter() {
        if !frame.visible || existing.contains(&frame.id) || !has_text(frame) {
            continue;
        }
        let (content, font_size, text_color, justify) = extract_text_props(frame);
        let transform = text_transform(frame, screen_w, screen_h, justify);
        commands.spawn((
            Text2d::new(content),
            TextFont { font_size, ..default() },
            TextColor(text_color),
            transform,
            RenderLayers::layer(UI_RENDER_LAYER),
            UiText(frame.id),
        ));
    }
}

fn has_text(frame: &crate::ui::frame::Frame) -> bool {
    match &frame.widget_data {
        Some(WidgetData::FontString(fs)) => !fs.text.is_empty(),
        Some(WidgetData::EditBox(_)) => true,
        Some(WidgetData::Button(btn)) => !btn.text.is_empty(),
        _ => false,
    }
}

fn extract_text_props(frame: &crate::ui::frame::Frame) -> (String, f32, Color, JustifyH) {
    match &frame.widget_data {
        Some(WidgetData::FontString(fs)) => {
            let [r, g, b, a] = fs.color;
            (fs.text.clone(), fs.font_size, Color::srgba(r, g, b, a * frame.effective_alpha), fs.justify_h)
        }
        Some(WidgetData::EditBox(eb)) => {
            let display = if eb.password { "*".repeat(eb.text.len()) } else { eb.text.clone() };
            (display, 14.0, Color::srgba(1.0, 1.0, 1.0, frame.effective_alpha), JustifyH::Left)
        }
        Some(WidgetData::Button(btn)) => extract_button_text(btn, frame.effective_alpha),
        _ => (String::new(), 12.0, Color::WHITE, JustifyH::Center),
    }
}

fn extract_button_text(btn: &crate::ui::widgets::button::ButtonData, alpha: f32) -> (String, f32, Color, JustifyH) {
    let (r, g, b) = match btn.state {
        ButtonState::Normal => (1.0, 0.82, 0.0),
        ButtonState::Pushed => (0.8, 0.65, 0.0),
        ButtonState::Disabled => (0.5, 0.5, 0.5),
    };
    (btn.text.clone(), 14.0, Color::srgba(r, g, b, alpha), JustifyH::Center)
}

/// Compute the transform for a text entity. Public for use by render_text_fx.
pub fn text_transform(
    frame: &crate::ui::frame::Frame,
    screen_w: f32,
    screen_h: f32,
    justify: JustifyH,
) -> Transform {
    let rect = frame.layout_rect.as_ref();
    let fx = rect.map_or(0.0, |r| r.x);
    let fy = rect.map_or(0.0, |r| r.y);
    let x = match justify {
        JustifyH::Left => fx + 4.0 - screen_w * 0.5,
        JustifyH::Center => fx + frame.width * 0.5 - screen_w * 0.5,
        JustifyH::Right => fx + frame.width - 4.0 - screen_w * 0.5,
    };
    let y = screen_h * 0.5 - fy - frame.height * 0.5;
    Transform::from_xyz(x, y, 10.0)
}
