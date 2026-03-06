use bevy::camera::visibility::RenderLayers;
use bevy::prelude::*;
use bevy::text::TextFont;

use crate::ui::frame::WidgetData;
use crate::ui::plugin::UiState;
use crate::ui::widgets::font_string::JustifyH;

/// Marker component for the 2D UI overlay camera.
#[derive(Component)]
pub struct UiCamera;

/// Links a Bevy sprite entity to a UI frame by its ID.
#[derive(Component)]
pub struct UiQuad(pub u64);

/// Render layer used for all UI elements, separate from the 3D scene.
pub const UI_RENDER_LAYER: usize = 1;

/// Spawns a 2D camera that renders after the 3D camera with a transparent background.
pub fn setup_ui_camera(mut commands: Commands) {
    commands.spawn((
        Camera2d,
        Camera {
            order: 1,
            clear_color: ClearColorConfig::None,
            ..default()
        },
        RenderLayers::layer(UI_RENDER_LAYER),
        UiCamera,
    ));
}

/// Syncs the frame registry into Bevy sprite entities each frame.
///
/// - Spawns `UiQuad` sprites for visible frames with a background color.
/// - Updates transform position and z-order for existing quads.
/// - Despawns quads whose frame no longer exists or is not renderable.
pub fn sync_ui_quads(
    mut state: ResMut<UiState>,
    mut commands: Commands,
    quads: Query<(Entity, &UiQuad)>,
) {
    let screen_w = state.registry.screen_width;
    let screen_h = state.registry.screen_height;

    // Build sorted list of (frame_id, sort_index) for visible, renderable frames.
    let sorted_ids = build_sorted_frame_ids(&state);

    // Map frame_id → sort_index for z-ordering.
    let sort_map: std::collections::HashMap<u64, usize> = sorted_ids
        .iter()
        .copied()
        .enumerate()
        .map(|(i, id)| (id, i))
        .collect();

    // Update or despawn existing quads.
    for (entity, ui_quad) in &quads {
        if let Some(&sort_idx) = sort_map.get(&ui_quad.0) {
            update_existing_quad(
                &state,
                entity,
                ui_quad.0,
                sort_idx,
                screen_w,
                screen_h,
                &mut commands,
            );
        } else {
            commands.entity(entity).despawn();
        }
    }

    // Spawn new quads for frames that don't have one yet.
    let existing: std::collections::HashSet<u64> = quads.iter().map(|(_, q)| q.0).collect();
    spawn_new_quads(
        &state,
        &sorted_ids,
        &sort_map,
        &existing,
        screen_w,
        screen_h,
        &mut commands,
    );

    state.registry.render_dirty.clear();
}

/// Collect frame IDs that are visible, have size, and have a background color,
/// sorted by (strata, frame_level, raise_order).
fn build_sorted_frame_ids(state: &UiState) -> Vec<u64> {
    let mut frames: Vec<_> = state
        .registry
        .frames_iter()
        .filter(|f| is_renderable(f))
        .map(|f| (f.id, f.strata, f.frame_level, f.raise_order))
        .collect();

    frames.sort_by(|a, b| a.1.cmp(&b.1).then(a.2.cmp(&b.2)).then(a.3.cmp(&b.3)));

    frames.into_iter().map(|(id, _, _, _)| id).collect()
}

fn is_renderable(f: &crate::ui::frame::Frame) -> bool {
    f.visible && f.width > 0.0 && f.height > 0.0 && f.background_color.is_some()
}

fn frame_transform(
    f: &crate::ui::frame::Frame,
    sort_idx: usize,
    screen_w: f32,
    screen_h: f32,
) -> Transform {
    let bx = f
        .width
        .mul_add(0.5, f.layout_rect.as_ref().map_or(0.0, |r| r.x))
        - screen_w * 0.5;
    let by = screen_h * 0.5 - f.layout_rect.as_ref().map_or(0.0, |r| r.y) - f.height * 0.5;
    let bz = sort_idx as f32 * 0.001;
    Transform::from_xyz(bx, by, bz)
}

fn frame_color(f: &crate::ui::frame::Frame) -> Color {
    let [r, g, b, a] = f.background_color.unwrap_or([1.0, 1.0, 1.0, 1.0]);
    Color::srgba(r, g, b, a * f.effective_alpha)
}

fn update_existing_quad(
    state: &UiState,
    entity: Entity,
    frame_id: u64,
    sort_idx: usize,
    screen_w: f32,
    screen_h: f32,
    commands: &mut Commands,
) {
    if let Some(frame) = state.registry.get(frame_id) {
        let transform = frame_transform(frame, sort_idx, screen_w, screen_h);
        let color = frame_color(frame);
        commands.entity(entity).insert((
            transform,
            Sprite {
                color,
                custom_size: Some(Vec2::new(frame.width, frame.height)),
                ..default()
            },
        ));
    }
}

fn spawn_new_quads(
    state: &UiState,
    sorted_ids: &[u64],
    sort_map: &std::collections::HashMap<u64, usize>,
    existing: &std::collections::HashSet<u64>,
    screen_w: f32,
    screen_h: f32,
    commands: &mut Commands,
) {
    for &frame_id in sorted_ids {
        if existing.contains(&frame_id) {
            continue;
        }
        if let Some(frame) = state.registry.get(frame_id) {
            let sort_idx = sort_map[&frame_id];
            let transform = frame_transform(frame, sort_idx, screen_w, screen_h);
            let color = frame_color(frame);
            commands.spawn((
                Sprite {
                    color,
                    custom_size: Some(Vec2::new(frame.width, frame.height)),
                    ..default()
                },
                transform,
                RenderLayers::layer(UI_RENDER_LAYER),
                UiQuad(frame_id),
            ));
        }
    }
}

/// Links a Bevy Text2d entity to a UI frame by its ID.
#[derive(Component)]
pub struct UiText(pub u64);

/// Syncs text content from the frame registry into Bevy Text2d entities.
pub fn sync_ui_text(
    state: Res<UiState>,
    mut commands: Commands,
    mut texts: Query<(
        Entity,
        &UiText,
        &mut Text2d,
        &mut TextFont,
        &mut TextColor,
        &mut Transform,
    )>,
) {
    let screen_w = state.registry.screen_width;
    let screen_h = state.registry.screen_height;

    let mut existing: std::collections::HashSet<u64> = std::collections::HashSet::new();

    // Update or despawn existing text entities.
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

    // Spawn new text entities.
    for frame in state.registry.frames_iter() {
        if !frame.visible || existing.contains(&frame.id) || !has_text(frame) {
            continue;
        }
        let (content, font_size, text_color, justify) = extract_text_props(frame);
        let transform = text_transform(frame, screen_w, screen_h, justify);
        commands.spawn((
            Text2d::new(content),
            TextFont {
                font_size,
                ..default()
            },
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
        Some(WidgetData::EditBox(_)) => true, // show even when empty (cursor)
        Some(WidgetData::Button(btn)) => !btn.text.is_empty(),
        _ => false,
    }
}

fn extract_text_props(frame: &crate::ui::frame::Frame) -> (String, f32, Color, JustifyH) {
    match &frame.widget_data {
        Some(WidgetData::FontString(fs)) => {
            let [r, g, b, a] = fs.color;
            (
                fs.text.clone(),
                fs.font_size,
                Color::srgba(r, g, b, a * frame.effective_alpha),
                fs.justify_h,
            )
        }
        Some(WidgetData::EditBox(eb)) => {
            let display = if eb.password {
                "*".repeat(eb.text.len())
            } else {
                eb.text.clone()
            };
            (
                display,
                14.0,
                Color::srgba(1.0, 1.0, 1.0, frame.effective_alpha),
                JustifyH::Left,
            )
        }
        Some(WidgetData::Button(btn)) => (
            btn.text.clone(),
            14.0,
            Color::srgba(1.0, 0.82, 0.0, frame.effective_alpha),
            JustifyH::Center,
        ),
        _ => (String::new(), 12.0, Color::WHITE, JustifyH::Center),
    }
}

fn text_transform(
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
    // Text renders above quads (z slightly higher than max quad z)
    Transform::from_xyz(x, y, 10.0)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ui::plugin::UiPlugin;

    #[test]
    fn ui_camera_spawned() {
        let mut app = App::new();
        app.add_plugins(MinimalPlugins);
        app.add_plugins(UiPlugin);
        app.update();

        let mut query = app.world_mut().query_filtered::<(), With<UiCamera>>();
        let count = query.iter(app.world()).count();
        assert_eq!(count, 1);
    }

    #[test]
    fn creates_quad_for_visible_frame() {
        let mut app = App::new();
        app.add_plugins(MinimalPlugins);
        app.add_plugins(UiPlugin);
        app.update(); // startup

        // Create a frame with size and background color
        {
            let mut ui = app.world_mut().resource_mut::<UiState>();
            let id = ui.registry.create_frame("Test", None);
            let frame = ui.registry.get_mut(id).unwrap();
            frame.width = 100.0;
            frame.height = 50.0;
            frame.background_color = Some([1.0, 0.0, 0.0, 1.0]);
        }

        app.update(); // sync

        let mut query = app.world_mut().query_filtered::<(), With<UiQuad>>();
        let count = query.iter(app.world()).count();
        assert!(count > 0, "Expected at least one UiQuad entity");
    }

    #[test]
    fn no_quad_without_background_color() {
        let mut app = App::new();
        app.add_plugins(MinimalPlugins);
        app.add_plugins(UiPlugin);
        app.update();

        {
            let mut ui = app.world_mut().resource_mut::<UiState>();
            let id = ui.registry.create_frame("NoColor", None);
            let frame = ui.registry.get_mut(id).unwrap();
            frame.width = 100.0;
            frame.height = 50.0;
            // No background_color set
        }

        app.update();

        let mut query = app.world_mut().query_filtered::<(), With<UiQuad>>();
        let count = query.iter(app.world()).count();
        assert_eq!(count, 0, "Should not create quad without background color");
    }

    #[test]
    fn despawns_quad_when_hidden() {
        let mut app = App::new();
        app.add_plugins(MinimalPlugins);
        app.add_plugins(UiPlugin);
        app.update();

        let frame_id;
        {
            let mut ui = app.world_mut().resource_mut::<UiState>();
            frame_id = ui.registry.create_frame("HideMe", None);
            let frame = ui.registry.get_mut(frame_id).unwrap();
            frame.width = 100.0;
            frame.height = 50.0;
            frame.background_color = Some([0.0, 1.0, 0.0, 1.0]);
        }

        app.update(); // quad spawned

        let mut query = app.world_mut().query_filtered::<(), With<UiQuad>>();
        assert_eq!(query.iter(app.world()).count(), 1);

        // Hide the frame
        {
            let mut ui = app.world_mut().resource_mut::<UiState>();
            ui.registry.set_shown(frame_id, false);
        }

        app.update(); // quad despawned

        let mut query = app.world_mut().query_filtered::<(), With<UiQuad>>();
        assert_eq!(query.iter(app.world()).count(), 0);
    }
}
