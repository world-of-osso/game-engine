//! Nine-slice frame rendering — 9 sprites per frame with nine_slice set.
//! Parts: 0=TL, 1=T, 2=TR, 3=L, 4=Center, 5=R, 6=BL, 7=B, 8=BR

use bevy::camera::visibility::RenderLayers;
use bevy::prelude::*;
use std::collections::{HashMap, HashSet};

use super::render::{UI_RENDER_LAYER, load_texture_source_pub};
use crate::ui::frame::NineSlice;
use crate::ui::plugin::UiState;
use crate::ui::widgets::texture::TextureSource;

/// Links a Bevy sprite to a nine-slice part (frame_id, part 0-8).
#[derive(Component)]
pub struct UiNineSlicePart(pub u64, pub u8);

/// Syncs nine-slice sprites (9 per frame that has nine_slice set).
pub fn sync_ui_nine_slices(
    state: Res<UiState>,
    mut commands: Commands,
    mut images: Option<ResMut<Assets<Image>>>,
    parts: Query<(Entity, &UiNineSlicePart)>,
    mut texture_cache: Local<HashMap<u32, Handle<Image>>>,
    mut file_texture_cache: Local<HashMap<String, Handle<Image>>>,
    mut missing_textures: Local<HashSet<u32>>,
    mut missing_file_textures: Local<HashSet<String>>,
) {
    let screen_w = state.registry.screen_width;
    let screen_h = state.registry.screen_height;
    let z_map = build_z_map(&state);

    let mut existing: HashSet<(u64, u8)> = HashSet::new();
    for (entity, part) in &parts {
        if should_keep_part(&state, part) {
            existing.insert((part.0, part.1));
            let z = z_map.get(&part.0).copied().unwrap_or(0.0);
            update_part(
                &state,
                entity,
                part,
                screen_w,
                screen_h,
                z,
                &mut commands,
                &mut images,
                &mut texture_cache,
                &mut file_texture_cache,
                &mut missing_textures,
                &mut missing_file_textures,
            );
        } else {
            commands.entity(entity).despawn();
        }
    }

    spawn_missing_parts(
        &state,
        &existing,
        &z_map,
        screen_w,
        screen_h,
        &mut commands,
        &mut images,
        &mut texture_cache,
        &mut file_texture_cache,
        &mut missing_textures,
        &mut missing_file_textures,
    );
}

/// Build z-order map: frame_id → z value, matching the strata sort used by UiQuad.
fn build_z_map(state: &UiState) -> HashMap<u64, f32> {
    let mut frames: Vec<_> = state
        .registry
        .frames_iter()
        .filter(|f| f.visible && f.width > 0.0 && f.height > 0.0)
        .map(|f| (f.id, f.strata, f.frame_level, f.raise_order))
        .collect();
    frames.sort_by(|a, b| a.1.cmp(&b.1).then(a.2.cmp(&b.2)).then(a.3.cmp(&b.3)));
    frames
        .iter()
        .enumerate()
        .map(|(i, &(id, _, _, _))| (id, i as f32 * 0.001))
        .collect()
}

fn should_keep_part(state: &UiState, part: &UiNineSlicePart) -> bool {
    state
        .registry
        .get(part.0)
        .is_some_and(|f| f.visible && f.nine_slice.is_some())
}

#[allow(clippy::too_many_arguments)]
fn update_part(
    state: &UiState,
    entity: Entity,
    part: &UiNineSlicePart,
    screen_w: f32,
    screen_h: f32,
    z: f32,
    commands: &mut Commands,
    images: &mut Option<ResMut<Assets<Image>>>,
    texture_cache: &mut HashMap<u32, Handle<Image>>,
    file_texture_cache: &mut HashMap<String, Handle<Image>>,
    missing_textures: &mut HashSet<u32>,
    missing_file_textures: &mut HashSet<String>,
) {
    let Some(frame) = state.registry.get(part.0) else {
        return;
    };
    let Some(nine_slice) = &frame.nine_slice else {
        return;
    };
    let (transform, size, color) = part_geometry(frame, nine_slice, part.1, screen_w, screen_h, z);
    let (image, tex_rect) = resolve_part_texture(
        nine_slice,
        part.1,
        images,
        texture_cache,
        file_texture_cache,
        missing_textures,
        missing_file_textures,
    );
    commands.entity(entity).insert((
        transform,
        Sprite {
            color,
            custom_size: Some(size),
            image,
            rect: tex_rect,
            ..default()
        },
    ));
}

#[allow(clippy::too_many_arguments)]
fn spawn_missing_parts(
    state: &UiState,
    existing: &HashSet<(u64, u8)>,
    z_map: &HashMap<u64, f32>,
    screen_w: f32,
    screen_h: f32,
    commands: &mut Commands,
    images: &mut Option<ResMut<Assets<Image>>>,
    texture_cache: &mut HashMap<u32, Handle<Image>>,
    file_texture_cache: &mut HashMap<String, Handle<Image>>,
    missing_textures: &mut HashSet<u32>,
    missing_file_textures: &mut HashSet<String>,
) {
    for frame in state.registry.frames_iter() {
        if !frame.visible {
            continue;
        }
        let Some(nine_slice) = &frame.nine_slice else {
            continue;
        };
        let z = z_map.get(&frame.id).copied().unwrap_or(0.0);
        for p in 0..9u8 {
            if existing.contains(&(frame.id, p)) {
                continue;
            }
            let (transform, size, color) =
                part_geometry(frame, nine_slice, p, screen_w, screen_h, z);
            let (image, tex_rect) = resolve_part_texture(
                nine_slice,
                p,
                images,
                texture_cache,
                file_texture_cache,
                missing_textures,
                missing_file_textures,
            );
            commands.spawn((
                Sprite {
                    color,
                    custom_size: Some(size),
                    image,
                    rect: tex_rect,
                    ..default()
                },
                transform,
                RenderLayers::layer(UI_RENDER_LAYER),
                UiNineSlicePart(frame.id, p),
            ));
        }
    }
}

/// Load the texture handle and compute the UV sub-rect for a nine-slice part.
/// Returns `(Handle<Image>, Option<Rect>)`. If no texture is set, returns defaults.
fn resolve_part_texture(
    nine_slice: &NineSlice,
    part: u8,
    images: &mut Option<ResMut<Assets<Image>>>,
    texture_cache: &mut HashMap<u32, Handle<Image>>,
    file_texture_cache: &mut HashMap<String, Handle<Image>>,
    missing_textures: &mut HashSet<u32>,
    missing_file_textures: &mut HashSet<String>,
) -> (Handle<Image>, Option<Rect>) {
    let source = if let Some(part_textures) = &nine_slice.part_textures {
        &part_textures[part as usize]
    } else {
        let Some(source) = &nine_slice.texture else {
            return (Handle::default(), None);
        };
        source
    };
    if matches!(source, TextureSource::None) {
        return (Handle::default(), None);
    }
    let Some(handle) = load_texture_source_pub(
        source,
        images,
        texture_cache,
        file_texture_cache,
        missing_textures,
        missing_file_textures,
    ) else {
        return (Handle::default(), None);
    };

    let uv_rect = images.as_ref().and_then(|assets| {
        let img = assets.get(&handle.handle)?;
        let atlas_rect = handle.rect.unwrap_or(Rect {
            min: Vec2::ZERO,
            max: Vec2::new(img.width() as f32, img.height() as f32),
        });
        if nine_slice.part_textures.is_some() {
            None
        } else if let Some(uv_rects) = &nine_slice.uv_rects {
            Some(explicit_uv_rect_for_part(uv_rects, part, atlas_rect))
        } else {
            let c = nine_slice.edge_size;
            let w = atlas_rect.max.x - atlas_rect.min.x;
            let h = atlas_rect.max.y - atlas_rect.min.y;
            let mut rect = uv_rect_for_part(part, w, h, c);
            rect.min += atlas_rect.min;
            rect.max += atlas_rect.min;
            Some(rect)
        }
    });

    (handle.handle, uv_rect)
}

fn explicit_uv_rect_for_part(uv_rects: &[[f32; 4]; 9], part: u8, atlas_rect: Rect) -> Rect {
    let [left, right, top, bottom] = uv_rects[part as usize];
    let size = atlas_rect.max - atlas_rect.min;
    Rect {
        min: Vec2::new(
            atlas_rect.min.x + left * size.x,
            atlas_rect.min.y + top * size.y,
        ),
        max: Vec2::new(
            atlas_rect.min.x + right * size.x,
            atlas_rect.min.y + bottom * size.y,
        ),
    }
}

/// Compute the UV sub-rect (in pixel coords) for each of the 9 parts.
fn uv_rect_for_part(part: u8, w: f32, h: f32, c: f32) -> Rect {
    let (min_x, max_x, min_y, max_y) = match part {
        0 => (0.0, c, 0.0, c),
        1 => (c, w - c, 0.0, c),
        2 => (w - c, w, 0.0, c),
        3 => (0.0, c, c, h - c),
        4 => (c, w - c, c, h - c),
        5 => (w - c, w, c, h - c),
        6 => (0.0, c, h - c, h),
        7 => (c, w - c, h - c, h),
        _ => (w - c, w, h - c, h),
    };
    Rect {
        min: Vec2::new(min_x, min_y),
        max: Vec2::new(max_x, max_y),
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
    z: f32,
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
        0 => (fx + e * 0.5, fy + e * 0.5, e, e, true),
        1 => (fx + e + inner_w * 0.5, fy + e * 0.5, inner_w, e, true),
        2 => (fx + e + inner_w + e * 0.5, fy + e * 0.5, e, e, true),
        3 => (fx + e * 0.5, fy + e + inner_h * 0.5, e, inner_h, true),
        4 => (
            fx + e + inner_w * 0.5,
            fy + e + inner_h * 0.5,
            inner_w,
            inner_h,
            false,
        ),
        5 => (
            fx + e + inner_w + e * 0.5,
            fy + e + inner_h * 0.5,
            e,
            inner_h,
            true,
        ),
        6 => (fx + e * 0.5, fy + e + inner_h + e * 0.5, e, e, true),
        7 => (
            fx + e + inner_w * 0.5,
            fy + e + inner_h + e * 0.5,
            inner_w,
            e,
            true,
        ),
        _ => (
            fx + e + inner_w + e * 0.5,
            fy + e + inner_h + e * 0.5,
            e,
            e,
            true,
        ),
    };

    // Textured nine-slices still honor per-part tint so focused edit boxes can
    // brighten their border/background while buttons opt into white explicitly.
    let color = if ns.texture.is_some() && !matches!(ns.texture, Some(TextureSource::None)) {
        let [r, g, b, a] = if is_border {
            ns.border_color
        } else {
            ns.bg_color
        };
        Color::srgba(r, g, b, a * frame.effective_alpha)
    } else {
        let [r, g, b, a] = if is_border {
            ns.border_color
        } else {
            ns.bg_color
        };
        Color::srgba(r, g, b, a * frame.effective_alpha)
    };

    let bx = cx - screen_w * 0.5;
    let by = screen_h * 0.5 - cy;
    (Transform::from_xyz(bx, by, z), Vec2::new(w, h), color)
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
        let mut q = app
            .world_mut()
            .query_filtered::<(), With<UiNineSlicePart>>();
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
        let mut q = app
            .world_mut()
            .query_filtered::<(), With<UiNineSlicePart>>();
        assert_eq!(q.iter(app.world()).count(), 0);
    }

    #[test]
    fn uv_rect_corners_and_center() {
        // 64x64 texture, 8px corners
        let tl = uv_rect_for_part(0, 64.0, 64.0, 8.0);
        assert_eq!(tl.min, Vec2::new(0.0, 0.0));
        assert_eq!(tl.max, Vec2::new(8.0, 8.0));

        let center = uv_rect_for_part(4, 64.0, 64.0, 8.0);
        assert_eq!(center.min, Vec2::new(8.0, 8.0));
        assert_eq!(center.max, Vec2::new(56.0, 56.0));

        let br = uv_rect_for_part(8, 64.0, 64.0, 8.0);
        assert_eq!(br.min, Vec2::new(56.0, 56.0));
        assert_eq!(br.max, Vec2::new(64.0, 64.0));
    }

    #[test]
    fn explicit_uv_rects_map_within_texture_rect() {
        let atlas_rect = Rect {
            min: Vec2::new(10.0, 20.0),
            max: Vec2::new(110.0, 220.0),
        };
        let mut uv_rects = [[0.0, 1.0, 0.0, 1.0]; 9];
        uv_rects[4] = [0.25, 0.75, 0.4, 0.6];
        let rect = explicit_uv_rect_for_part(&uv_rects, 4, atlas_rect);
        assert_eq!(rect.min, Vec2::new(35.0, 100.0));
        assert_eq!(rect.max, Vec2::new(85.0, 140.0));
    }

    #[test]
    fn part_textures_store_distinct_sources() {
        let ns = NineSlice {
            part_textures: Some(std::array::from_fn(|i| {
                TextureSource::File(format!("part-{i}.blp"))
            })),
            ..Default::default()
        };
        let Some(part_textures) = ns.part_textures.as_ref() else {
            panic!("expected part textures")
        };
        match &part_textures[4] {
            TextureSource::File(path) => assert_eq!(path, "part-4.blp"),
            other => panic!("unexpected texture source: {other:?}"),
        }
    }
}
