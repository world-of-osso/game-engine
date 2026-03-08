use bevy::camera::visibility::RenderLayers;
use bevy::prelude::*;
use std::collections::{HashMap, HashSet};

use crate::asset;
use crate::ui::frame::{NineSlice, WidgetData};
use crate::ui::plugin::UiState;
use crate::ui::widgets::button::ButtonState;
use crate::ui::widgets::texture::TextureSource;

/// Marker component for the 2D UI overlay camera.
#[derive(Component)]
pub struct UiCamera;

/// Links a Bevy sprite entity to a UI frame by its ID.
#[derive(Component)]
pub struct UiQuad(pub u64);

/// Links a Bevy Text2d entity to a UI frame by its ID.
#[derive(Component)]
pub struct UiText(pub u64);

/// Marks a highlight overlay sprite entity for a button frame.
#[derive(Component)]
pub struct UiButtonHighlight(pub u64);

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
pub fn sync_ui_quads(
    mut state: ResMut<UiState>,
    mut commands: Commands,
    mut images: Option<ResMut<Assets<Image>>>,
    quads: Query<(Entity, &UiQuad)>,
    mut texture_cache: Local<HashMap<u32, Handle<Image>>>,
    mut file_texture_cache: Local<HashMap<String, Handle<Image>>>,
    mut missing_textures: Local<HashSet<u32>>,
    mut missing_file_textures: Local<HashSet<String>>,
) {
    let screen_w = state.registry.screen_width;
    let screen_h = state.registry.screen_height;

    let sorted_ids = build_sorted_frame_ids(&state);
    let sort_map: HashMap<u64, usize> = sorted_ids
        .iter()
        .copied()
        .enumerate()
        .map(|(i, id)| (id, i))
        .collect();

    update_or_despawn_quads(
        &state, &sort_map, screen_w, screen_h,
        &mut commands, &mut images, &mut texture_cache, &mut file_texture_cache,
        &mut missing_textures, &mut missing_file_textures, &quads,
    );

    let existing: HashSet<u64> = quads.iter().map(|(_, q)| q.0).collect();
    spawn_new_quads(
        &state, &sorted_ids, &sort_map, &existing, screen_w, screen_h,
        &mut commands, &mut images, &mut texture_cache, &mut file_texture_cache,
        &mut missing_textures, &mut missing_file_textures,
    );

    state.registry.render_dirty.clear();
}

fn update_or_despawn_quads(
    state: &UiState,
    sort_map: &HashMap<u64, usize>,
    screen_w: f32,
    screen_h: f32,
    commands: &mut Commands,
    images: &mut Option<ResMut<Assets<Image>>>,
    texture_cache: &mut HashMap<u32, Handle<Image>>,
    file_texture_cache: &mut HashMap<String, Handle<Image>>,
    missing_textures: &mut HashSet<u32>,
    missing_file_textures: &mut HashSet<String>,
    quads: &Query<(Entity, &UiQuad)>,
) {
    for (entity, ui_quad) in quads {
        if let Some(&sort_idx) = sort_map.get(&ui_quad.0) {
            update_quad(
                state, entity, ui_quad.0, sort_idx,
                screen_w, screen_h, commands, images,
                texture_cache, file_texture_cache,
                missing_textures, missing_file_textures,
            );
        } else {
            commands.entity(entity).despawn();
        }
    }
}

// --- Quad helpers ---

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
    // Frames with nine_slice are rendered by the nine-slice system, not as quads.
    if f.nine_slice.is_some() {
        return false;
    }
    f.visible
        && f.width > 0.0
        && f.height > 0.0
        && (f.background_color.is_some()
            || frame_texture_fdid(f).is_some()
            || frame_has_button_texture(f)
            || f.backdrop.as_ref().is_some_and(|b| b.bg_color.is_some())
            || matches!(f.widget_data, Some(WidgetData::StatusBar(_))))
}

fn frame_has_button_texture(f: &crate::ui::frame::Frame) -> bool {
    let Some(WidgetData::Button(btn)) = &f.widget_data else {
        return false;
    };
    btn.normal_texture.is_some()
        || btn.pushed_texture.is_some()
        || btn.disabled_texture.is_some()
}

fn frame_texture_fdid(f: &crate::ui::frame::Frame) -> Option<u32> {
    let WidgetData::Texture(texture) = f.widget_data.as_ref()? else {
        return None;
    };
    let TextureSource::FileDataId(fdid) = texture.source else {
        return None;
    };
    Some(fdid)
}

fn frame_transform(f: &crate::ui::frame::Frame, sort_idx: usize, sw: f32, sh: f32) -> Transform {
    let bx = f.width.mul_add(0.5, f.layout_rect.as_ref().map_or(0.0, |r| r.x)) - sw * 0.5;
    let by = sh * 0.5 - f.layout_rect.as_ref().map_or(0.0, |r| r.y) - f.height * 0.5;
    Transform::from_xyz(bx, by, sort_idx as f32 * 0.001)
}

fn frame_color(f: &crate::ui::frame::Frame) -> Color {
    let base = f
        .background_color
        .or_else(|| f.backdrop.as_ref().and_then(|b| b.bg_color));
    let [r, g, b, a] = base.unwrap_or([1.0, 1.0, 1.0, 1.0]);
    Color::srgba(r, g, b, a * f.effective_alpha)
}

/// Returns `(size, offset)` for the sprite quad.
///
/// For StatusBar, width is scaled by the fill fraction and the quad is
/// left-aligned by shifting right by half the difference between the full
/// frame width and the filled width.  All other frames use their full size
/// with no offset.
pub(crate) fn frame_sprite_params(f: &crate::ui::frame::Frame) -> (Vec2, Vec2) {
    if let Some(WidgetData::StatusBar(sb)) = &f.widget_data {
        let fill = ((sb.value - sb.min) / (sb.max - sb.min).max(f64::EPSILON))
            .clamp(0.0, 1.0) as f32;
        let filled_w = f.width * fill;
        let offset_x = (filled_w - f.width) * 0.5;
        (Vec2::new(filled_w, f.height), Vec2::new(offset_x, 0.0))
    } else {
        (Vec2::new(f.width, f.height), Vec2::ZERO)
    }
}

fn update_quad(
    state: &UiState,
    entity: Entity,
    frame_id: u64,
    sort_idx: usize,
    sw: f32,
    sh: f32,
    commands: &mut Commands,
    images: &mut Option<ResMut<Assets<Image>>>,
    texture_cache: &mut HashMap<u32, Handle<Image>>,
    file_texture_cache: &mut HashMap<String, Handle<Image>>,
    missing_textures: &mut HashSet<u32>,
    missing_file_textures: &mut HashSet<String>,
) {
    let Some(frame) = state.registry.get(frame_id) else {
        return;
    };
    let (sprite_size, sprite_offset) = frame_sprite_params(frame);
    let mut transform = frame_transform(frame, sort_idx, sw, sh);
    transform.translation.x += sprite_offset.x;
    transform.translation.y += sprite_offset.y;
    let (color, image) = frame_visual(
        frame, images, texture_cache, file_texture_cache,
        missing_textures, missing_file_textures,
    );
    commands.entity(entity).insert((
        transform,
        Sprite {
            color,
            custom_size: Some(sprite_size),
            image,
            ..default()
        },
    ));
}

fn spawn_new_quads(
    state: &UiState,
    sorted_ids: &[u64],
    sort_map: &HashMap<u64, usize>,
    existing: &HashSet<u64>,
    sw: f32,
    sh: f32,
    commands: &mut Commands,
    images: &mut Option<ResMut<Assets<Image>>>,
    texture_cache: &mut HashMap<u32, Handle<Image>>,
    file_texture_cache: &mut HashMap<String, Handle<Image>>,
    missing_textures: &mut HashSet<u32>,
    missing_file_textures: &mut HashSet<String>,
) {
    for &frame_id in sorted_ids {
        if existing.contains(&frame_id) {
            continue;
        }
        let Some(frame) = state.registry.get(frame_id) else {
            continue;
        };
        let sort_idx = sort_map[&frame_id];
        let (sprite_size, sprite_offset) = frame_sprite_params(frame);
        let mut transform = frame_transform(frame, sort_idx, sw, sh);
        transform.translation.x += sprite_offset.x;
        transform.translation.y += sprite_offset.y;
        let (color, image) = frame_visual(
            frame, images, texture_cache, file_texture_cache,
            missing_textures, missing_file_textures,
        );
        commands.spawn((
            Sprite {
                color,
                custom_size: Some(sprite_size),
                image,
                ..default()
            },
            transform,
            RenderLayers::layer(UI_RENDER_LAYER),
            UiQuad(frame_id),
        ));
    }
}

fn frame_visual(
    frame: &crate::ui::frame::Frame,
    images: &mut Option<ResMut<Assets<Image>>>,
    texture_cache: &mut HashMap<u32, Handle<Image>>,
    file_texture_cache: &mut HashMap<String, Handle<Image>>,
    missing_textures: &mut HashSet<u32>,
    missing_file_textures: &mut HashSet<String>,
) -> (Color, Handle<Image>) {
    if let Some(WidgetData::StatusBar(sb)) = &frame.widget_data {
        let [r, g, b, a] = sb.color;
        return (Color::srgba(r, g, b, a * frame.effective_alpha), Handle::default());
    }
    if let Some(WidgetData::Button(btn)) = &frame.widget_data {
        if let Some(handle) = button_texture(
            btn, frame.effective_alpha, images,
            texture_cache, file_texture_cache,
            missing_textures, missing_file_textures,
        ) {
            return handle;
        }
    }
    if let Some(fdid) = frame_texture_fdid(frame)
        && let Some(handle) = load_texture(fdid, images, texture_cache, missing_textures)
    {
        // TODO: additive blend requires custom pipeline
        return (texture_tint(frame), handle);
    }
    (frame_color(frame), Handle::default())
}

fn button_texture(
    btn: &crate::ui::widgets::button::ButtonData,
    effective_alpha: f32,
    images: &mut Option<ResMut<Assets<Image>>>,
    texture_cache: &mut HashMap<u32, Handle<Image>>,
    file_texture_cache: &mut HashMap<String, Handle<Image>>,
    missing_textures: &mut HashSet<u32>,
    missing_file_textures: &mut HashSet<String>,
) -> Option<(Color, Handle<Image>)> {
    let source = select_button_texture_source(btn)?;
    let handle = load_texture_source(
        source, images, texture_cache, file_texture_cache,
        missing_textures, missing_file_textures,
    )?;
    Some((Color::srgba(1.0, 1.0, 1.0, effective_alpha), handle))
}

fn select_button_texture_source(
    btn: &crate::ui::widgets::button::ButtonData,
) -> Option<&TextureSource> {
    let source = match btn.state {
        ButtonState::Disabled => btn.disabled_texture.as_ref().or(btn.normal_texture.as_ref()),
        ButtonState::Pushed => btn.pushed_texture.as_ref().or(btn.normal_texture.as_ref()),
        ButtonState::Normal => btn.normal_texture.as_ref(),
    }?;
    if matches!(source, TextureSource::None) {
        return None;
    }
    Some(source)
}

pub fn load_texture_source_pub(
    source: &TextureSource,
    images: &mut Option<ResMut<Assets<Image>>>,
    texture_cache: &mut HashMap<u32, Handle<Image>>,
    file_texture_cache: &mut HashMap<String, Handle<Image>>,
    missing_textures: &mut HashSet<u32>,
    missing_file_textures: &mut HashSet<String>,
) -> Option<Handle<Image>> {
    load_texture_source(source, images, texture_cache, file_texture_cache, missing_textures, missing_file_textures)
}

fn load_texture_source(
    source: &TextureSource,
    images: &mut Option<ResMut<Assets<Image>>>,
    texture_cache: &mut HashMap<u32, Handle<Image>>,
    file_texture_cache: &mut HashMap<String, Handle<Image>>,
    missing_textures: &mut HashSet<u32>,
    missing_file_textures: &mut HashSet<String>,
) -> Option<Handle<Image>> {
    match source {
        TextureSource::FileDataId(fdid) => {
            load_texture(*fdid, images, texture_cache, missing_textures)
        }
        TextureSource::File(path) => {
            load_file_texture(path, images, file_texture_cache, missing_file_textures)
        }
        _ => None,
    }
}

fn load_file_texture(
    path: &str,
    images: &mut Option<ResMut<Assets<Image>>>,
    file_texture_cache: &mut HashMap<String, Handle<Image>>,
    missing_file_textures: &mut HashSet<String>,
) -> Option<Handle<Image>> {
    if let Some(handle) = file_texture_cache.get(path) {
        return Some(handle.clone());
    }
    if missing_file_textures.contains(path) {
        return None;
    }
    let assets = images.as_mut().map(|images| &mut **images)?;
    let image = match asset::blp::load_blp_gpu_image(std::path::Path::new(path)) {
        Ok(image) => image,
        Err(_) => {
            missing_file_textures.insert(path.to_string());
            return None;
        }
    };
    let handle = assets.add(image);
    file_texture_cache.insert(path.to_string(), handle.clone());
    Some(handle)
}

/// Apply vertex_color tinting and effective_alpha to textured frames.
/// If the texture is desaturated, compute luminance and use grey.
pub fn texture_tint(frame: &crate::ui::frame::Frame) -> Color {
    let (vertex_color, desaturated) = match &frame.widget_data {
        Some(WidgetData::Texture(tex)) => (tex.vertex_color, tex.desaturated),
        _ => ([1.0, 1.0, 1.0, 1.0], false),
    };
    let [r, g, b, a] = vertex_color;
    if desaturated {
        let lum = 0.2126 * r + 0.7152 * g + 0.0722 * b;
        Color::srgba(lum, lum, lum, a * frame.effective_alpha)
    } else {
        Color::srgba(r, g, b, a * frame.effective_alpha)
    }
}

fn load_texture(
    fdid: u32,
    images: &mut Option<ResMut<Assets<Image>>>,
    texture_cache: &mut HashMap<u32, Handle<Image>>,
    missing_textures: &mut HashSet<u32>,
) -> Option<Handle<Image>> {
    if let Some(handle) = texture_cache.get(&fdid) {
        return Some(handle.clone());
    }
    if missing_textures.contains(&fdid) {
        return None;
    }
    let assets = images.as_mut().map(|images| &mut **images)?;
    let path = asset::casc_resolver::ensure_texture(fdid)?;
    let image = match asset::blp::load_blp_gpu_image(&path) {
        Ok(image) => image,
        Err(_) => {
            missing_textures.insert(fdid);
            return None;
        }
    };
    let handle = assets.add(image);
    texture_cache.insert(fdid, handle.clone());
    Some(handle)
}

// --- Button highlight overlay ---

/// Manages highlight overlay sprites for hovered buttons.
pub fn sync_ui_button_highlights(
    state: Res<UiState>,
    mut commands: Commands,
    mut images: Option<ResMut<Assets<Image>>>,
    highlights: Query<(Entity, &UiButtonHighlight)>,
    mut file_texture_cache: Local<HashMap<String, Handle<Image>>>,
    mut missing_file_textures: Local<HashSet<String>>,
) {
    let existing: HashMap<u64, Entity> = highlights.iter().map(|(e, h)| (h.0, e)).collect();
    let mut seen: HashSet<u64> = HashSet::new();
    let sw = state.registry.screen_width;
    let sh = state.registry.screen_height;

    for frame in state.registry.frames_iter() {
        let Some(path) = button_highlight_file_path(frame) else { continue };
        seen.insert(frame.id);
        let Some(WidgetData::Button(btn)) = &frame.widget_data else { continue };
        if !btn.hovered || btn.state == ButtonState::Disabled {
            if let Some(&entity) = existing.get(&frame.id) { commands.entity(entity).despawn(); }
            continue;
        }
        let Some(handle) = load_file_texture(path, &mut images, &mut file_texture_cache, &mut missing_file_textures) else { continue };
        upsert_highlight_sprite(frame, handle, sw, sh, &existing, &mut commands);
    }

    despawn_stale_highlights(&existing, &seen, &mut commands);
}

fn button_highlight_file_path(frame: &crate::ui::frame::Frame) -> Option<&str> {
    let WidgetData::Button(btn) = frame.widget_data.as_ref()? else { return None };
    match btn.highlight_texture.as_ref()? {
        TextureSource::File(p) => Some(p.as_str()),
        _ => None,
    }
}

fn upsert_highlight_sprite(
    frame: &crate::ui::frame::Frame,
    handle: Handle<Image>,
    sw: f32,
    sh: f32,
    existing: &HashMap<u64, Entity>,
    commands: &mut Commands,
) {
    let alpha = frame.effective_alpha * 0.5;
    let color = Color::srgba(1.0, 1.0, 1.0, alpha);
    let size = Vec2::new(frame.width, frame.height);
    let bx = frame.width.mul_add(0.5, frame.layout_rect.as_ref().map_or(0.0, |r| r.x)) - sw * 0.5;
    let by = sh * 0.5 - frame.layout_rect.as_ref().map_or(0.0, |r| r.y) - frame.height * 0.5;
    let transform = Transform::from_xyz(bx, by, 500.0);
    let sprite = Sprite { color, custom_size: Some(size), image: handle, ..default() };
    if let Some(&entity) = existing.get(&frame.id) {
        commands.entity(entity).insert((transform, sprite));
    } else {
        commands.spawn((sprite, transform, RenderLayers::layer(UI_RENDER_LAYER), UiButtonHighlight(frame.id)));
    }
}

fn despawn_stale_highlights(
    existing: &HashMap<u64, Entity>,
    seen: &HashSet<u64>,
    commands: &mut Commands,
) {
    for (&frame_id, &entity) in existing {
        if !seen.contains(&frame_id) {
            commands.entity(entity).despawn();
        }
    }
}

// --- Button nine-slice sync ---

const BUTTON_NINE_SLICE_EDGE: f32 = 4.0;

/// Converts button textures into nine-slice rendering based on current state.
pub fn sync_button_nine_slices(mut state: ResMut<UiState>) {
    let ids: Vec<u64> = state
        .registry
        .frames_iter()
        .filter(|f| matches!(&f.widget_data, Some(WidgetData::Button(_))))
        .map(|f| f.id)
        .collect();

    for id in ids {
        let texture = {
            let Some(frame) = state.registry.get(id) else { continue };
            let Some(WidgetData::Button(btn)) = &frame.widget_data else { continue };
            select_button_texture_source(btn).cloned()
        };
        let Some(frame) = state.registry.get_mut(id) else { continue };
        match texture {
            Some(tex) => {
                frame.nine_slice = Some(NineSlice {
                    edge_size: BUTTON_NINE_SLICE_EDGE,
                    texture: Some(tex),
                    ..Default::default()
                });
            }
            None => {
                frame.nine_slice = None;
            }
        }
    }
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
        assert_eq!(query.iter(app.world()).count(), 1);
    }

    #[test]
    fn creates_quad_for_visible_frame() {
        let mut app = App::new();
        app.add_plugins(MinimalPlugins);
        app.add_plugins(UiPlugin);
        app.update();
        {
            let mut ui = app.world_mut().resource_mut::<UiState>();
            let id = ui.registry.create_frame("Test", None);
            let frame = ui.registry.get_mut(id).unwrap();
            frame.width = 100.0;
            frame.height = 50.0;
            frame.background_color = Some([1.0, 0.0, 0.0, 1.0]);
        }
        app.update();
        let mut query = app.world_mut().query_filtered::<(), With<UiQuad>>();
        assert!(query.iter(app.world()).count() > 0);
    }

    #[test]
    fn no_quad_without_background_color() {
        let mut app = App::new();
        app.add_plugins(MinimalPlugins);
        app.add_plugins(UiPlugin);
        app.update();
        let baseline = {
            let mut q = app.world_mut().query_filtered::<(), With<UiQuad>>();
            q.iter(app.world()).count()
        };
        {
            let mut ui = app.world_mut().resource_mut::<UiState>();
            let id = ui.registry.create_frame("NoColor", None);
            let frame = ui.registry.get_mut(id).unwrap();
            frame.width = 100.0;
            frame.height = 50.0;
        }
        app.update();
        let mut q = app.world_mut().query_filtered::<(), With<UiQuad>>();
        assert_eq!(q.iter(app.world()).count(), baseline);
    }

    #[test]
    fn despawns_quad_when_hidden() {
        let mut app = App::new();
        app.add_plugins(MinimalPlugins);
        app.add_plugins(UiPlugin);
        app.update();
        let baseline = {
            let mut q = app.world_mut().query_filtered::<(), With<UiQuad>>();
            q.iter(app.world()).count()
        };
        let frame_id;
        {
            let mut ui = app.world_mut().resource_mut::<UiState>();
            frame_id = ui.registry.create_frame("HideMe", None);
            let frame = ui.registry.get_mut(frame_id).unwrap();
            frame.width = 100.0;
            frame.height = 50.0;
            frame.background_color = Some([0.0, 1.0, 0.0, 1.0]);
        }
        app.update();
        let mut q = app.world_mut().query_filtered::<(), With<UiQuad>>();
        assert_eq!(q.iter(app.world()).count(), baseline + 1);
        {
            let mut ui = app.world_mut().resource_mut::<UiState>();
            ui.registry.set_shown(frame_id, false);
        }
        app.update();
        let mut q = app.world_mut().query_filtered::<(), With<UiQuad>>();
        assert_eq!(q.iter(app.world()).count(), baseline);
    }

    #[test]
    fn backdrop_bg_color_renderable() {
        let mut app = App::new();
        app.add_plugins(MinimalPlugins);
        app.add_plugins(UiPlugin);
        app.update();
        let baseline = {
            let mut q = app.world_mut().query_filtered::<(), With<UiQuad>>();
            q.iter(app.world()).count()
        };
        {
            let mut ui = app.world_mut().resource_mut::<UiState>();
            let id = ui.registry.create_frame("Bd", None);
            let frame = ui.registry.get_mut(id).unwrap();
            frame.width = 100.0;
            frame.height = 50.0;
            frame.backdrop = Some(crate::ui::frame::Backdrop {
                bg_color: Some([0.1, 0.1, 0.1, 1.0]),
                ..Default::default()
            });
        }
        app.update();
        let mut q = app.world_mut().query_filtered::<(), With<UiQuad>>();
        assert_eq!(q.iter(app.world()).count(), baseline + 1);
    }

    #[test]
    fn statusbar_sprite_params_proportional_to_fill() {
        let mut frame = crate::ui::frame::Frame::new(1, None, crate::ui::frame::WidgetType::StatusBar);
        frame.width = 200.0;
        frame.height = 20.0;
        frame.widget_data = Some(WidgetData::StatusBar(
            crate::ui::widgets::slider::StatusBarData {
                value: 0.5,
                min: 0.0,
                max: 1.0,
                ..Default::default()
            },
        ));
        let (size, offset) = frame_sprite_params(&frame);
        assert!((size.x - 100.0).abs() < 0.01, "half fill → width 100, got {}", size.x);
        assert_eq!(size.y, 20.0);
        assert!((offset.x - (-50.0)).abs() < 0.01, "offset_x should be -50, got {}", offset.x);
        assert_eq!(offset.y, 0.0);
    }

    #[test]
    fn statusbar_sprite_params_full_fill() {
        let mut frame = crate::ui::frame::Frame::new(1, None, crate::ui::frame::WidgetType::StatusBar);
        frame.width = 200.0;
        frame.height = 20.0;
        frame.widget_data = Some(WidgetData::StatusBar(
            crate::ui::widgets::slider::StatusBarData {
                value: 1.0,
                min: 0.0,
                max: 1.0,
                ..Default::default()
            },
        ));
        let (size, offset) = frame_sprite_params(&frame);
        assert!((size.x - 200.0).abs() < 0.01);
        assert!((offset.x).abs() < 0.01);
    }

    #[test]
    fn button_disabled_text_grey() {
        let btn = crate::ui::widgets::button::ButtonData {
            state: ButtonState::Disabled,
            text: "Test".into(),
            ..Default::default()
        };
        let (_, _, color, _) = crate::ui::render_text::extract_button_text(&btn, 1.0);
        let Color::Srgba(srgba) = color else { panic!("expected srgba") };
        assert!(srgba.red < 0.6, "disabled should be grey");
    }
}
