//! Nine-slice frame rendering — 9 sprites per frame with nine_slice set.
//! Parts: 0=TL, 1=T, 2=TR, 3=L, 4=Center, 5=R, 6=BL, 7=B, 8=BR

use bevy::camera::visibility::RenderLayers;
use bevy::prelude::*;
use std::collections::HashSet;

use crate::ui::frame::NineSlice;
use crate::ui::plugin::UiState;
use super::render::UI_RENDER_LAYER;

/// Links a Bevy sprite to a nine-slice part (frame_id, part 0-8).
#[derive(Component)]
pub struct UiNineSlicePart(pub u64, pub u8);

/// Syncs nine-slice sprites (9 per frame that has nine_slice set).
pub fn sync_ui_nine_slices(
    state: Res<UiState>,
    mut commands: Commands,
    parts: Query<(Entity, &UiNineSlicePart)>,
) {
    let screen_w = state.registry.screen_width;
    let screen_h = state.registry.screen_height;

    let mut existing: HashSet<(u64, u8)> = HashSet::new();
    for (entity, part) in &parts {
        if should_keep_part(&state, part) {
            existing.insert((part.0, part.1));
            update_part(&state, entity, part, screen_w, screen_h, &mut commands);
        } else {
            commands.entity(entity).despawn();
        }
    }

    spawn_missing_parts(&state, &existing, screen_w, screen_h, &mut commands);
}

fn should_keep_part(state: &UiState, part: &UiNineSlicePart) -> bool {
    state.registry.get(part.0).is_some_and(|f| f.visible && f.nine_slice.is_some())
}

fn update_part(
    state: &UiState,
    entity: Entity,
    part: &UiNineSlicePart,
    screen_w: f32,
    screen_h: f32,
    commands: &mut Commands,
) {
    let Some(frame) = state.registry.get(part.0) else { return };
    let Some(nine_slice) = &frame.nine_slice else { return };
    let (transform, size, color) = part_geometry(frame, nine_slice, part.1, screen_w, screen_h);
    commands.entity(entity).insert((
        transform,
        Sprite { color, custom_size: Some(size), ..default() },
    ));
}

fn spawn_missing_parts(
    state: &UiState,
    existing: &HashSet<(u64, u8)>,
    screen_w: f32,
    screen_h: f32,
    commands: &mut Commands,
) {
    for frame in state.registry.frames_iter() {
        if !frame.visible { continue; }
        let Some(nine_slice) = &frame.nine_slice else { continue };
        for p in 0..9u8 {
            if existing.contains(&(frame.id, p)) { continue; }
            let (transform, size, color) = part_geometry(frame, nine_slice, p, screen_w, screen_h);
            commands.spawn((
                Sprite { color, custom_size: Some(size), ..default() },
                transform,
                RenderLayers::layer(UI_RENDER_LAYER),
                UiNineSlicePart(frame.id, p),
            ));
        }
    }
}

/// Compute transform, size, color for one nine-slice part.
/// Layout: corners are edge×edge; edges stretch; center fills interior.
/// Parts: 0=TL, 1=T, 2=TR, 3=L, 4=Center, 5=R, 6=BL, 7=B, 8=BR
pub(crate) fn part_geometry(
    frame: &crate::ui::frame::Frame,
    ns: &NineSlice,
    part: u8,
    screen_w: f32,
    screen_h: f32,
) -> (Transform, Vec2, Color) {
    let e = ns.edge_size;
    let rect = frame.layout_rect.as_ref();
    let fx = rect.map_or(0.0, |r| r.x);
    let fy = rect.map_or(0.0, |r| r.y);
    let fw = frame.width;
    let fh = frame.height;
    let inner_w = (fw - e * 2.0).max(0.0);
    let inner_h = (fh - e * 2.0).max(0.0);

    // cx/cy in WoW screen space (top-left origin, y down)
    let (cx, cy, w, h, is_border) = match part {
        0 => (fx + e * 0.5,           fy + e * 0.5,            e,       e,       true),
        1 => (fx + e + inner_w * 0.5, fy + e * 0.5,            inner_w, e,       true),
        2 => (fx + e + inner_w + e * 0.5, fy + e * 0.5,        e,       e,       true),
        3 => (fx + e * 0.5,           fy + e + inner_h * 0.5,  e,       inner_h, true),
        4 => (fx + e + inner_w * 0.5, fy + e + inner_h * 0.5, inner_w, inner_h, false),
        5 => (fx + e + inner_w + e * 0.5, fy + e + inner_h * 0.5, e,   inner_h, true),
        6 => (fx + e * 0.5,           fy + e + inner_h + e * 0.5, e,   e,       true),
        7 => (fx + e + inner_w * 0.5, fy + e + inner_h + e * 0.5, inner_w, e,   true),
        _ => (fx + e + inner_w + e * 0.5, fy + e + inner_h + e * 0.5, e, e,    true),
    };

    let [r, g, b, a] = if is_border { ns.border_color } else { ns.bg_color };
    let color = Color::srgba(r, g, b, a * frame.effective_alpha);
    let bx = cx - screen_w * 0.5;
    let by = screen_h * 0.5 - cy;
    (Transform::from_xyz(bx, by, 9.4), Vec2::new(w, h), color)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ui::plugin::UiPlugin;

    #[test]
    fn nine_slice_spawns_9_parts() {
        let mut app = App::new();
        app.add_plugins(MinimalPlugins);
        app.add_plugins(UiPlugin);
        app.update();
        {
            let mut ui = app.world_mut().resource_mut::<UiState>();
            let id = ui.registry.create_frame("NineSliceFrame", None);
            let frame = ui.registry.get_mut(id).unwrap();
            frame.width = 200.0;
            frame.height = 100.0;
            frame.nine_slice = Some(NineSlice::default());
        }
        app.update();
        let mut q = app.world_mut().query_filtered::<(), With<UiNineSlicePart>>();
        assert_eq!(q.iter(app.world()).count(), 9);
    }

    #[test]
    fn frame_without_nine_slice_spawns_no_parts() {
        let mut app = App::new();
        app.add_plugins(MinimalPlugins);
        app.add_plugins(UiPlugin);
        app.update();
        {
            let mut ui = app.world_mut().resource_mut::<UiState>();
            let id = ui.registry.create_frame("PlainFrame", None);
            let frame = ui.registry.get_mut(id).unwrap();
            frame.width = 200.0;
            frame.height = 100.0;
        }
        app.update();
        let mut q = app.world_mut().query_filtered::<(), With<UiNineSlicePart>>();
        assert_eq!(q.iter(app.world()).count(), 0);
    }
}
