use bevy::camera::visibility::RenderLayers;
use bevy::prelude::*;
use bevy::text::TextFont;
use std::collections::{HashMap, HashSet};

use crate::asset;
use crate::ui::frame::WidgetData;
use crate::ui::plugin::UiState;
use crate::ui::widgets::button::ButtonState;
use crate::ui::widgets::font_string::JustifyH;
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
    mut missing_textures: Local<HashSet<u32>>,
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
        &mut commands, &mut images, &mut texture_cache, &mut missing_textures, &quads,
    );

    let existing: HashSet<u64> = quads.iter().map(|(_, q)| q.0).collect();
    spawn_new_quads(
        &state, &sorted_ids, &sort_map, &existing, screen_w, screen_h,
        &mut commands, &mut images, &mut texture_cache, &mut missing_textures,
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
    missing_textures: &mut HashSet<u32>,
    quads: &Query<(Entity, &UiQuad)>,
) {
    for (entity, ui_quad) in quads {
        if let Some(&sort_idx) = sort_map.get(&ui_quad.0) {
            update_quad(
                state, entity, ui_quad.0, sort_idx,
                screen_w, screen_h, commands, images, texture_cache, missing_textures,
            );
        } else {
            commands.entity(entity).despawn();
        }
    }
}

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
    f.visible
        && f.width > 0.0
        && f.height > 0.0
        && (f.background_color.is_some()
            || frame_texture_fdid(f).is_some()
            || f.backdrop.as_ref().is_some_and(|b| b.bg_color.is_some())
            || matches!(f.widget_data, Some(WidgetData::StatusBar(_))))
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
fn frame_sprite_params(f: &crate::ui::frame::Frame) -> (Vec2, Vec2) {
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
    missing_textures: &mut HashSet<u32>,
) {
    let Some(frame) = state.registry.get(frame_id) else {
        return;
    };
    let (sprite_size, sprite_offset) = frame_sprite_params(frame);
    let mut transform = frame_transform(frame, sort_idx, sw, sh);
    transform.translation.x += sprite_offset.x;
    transform.translation.y += sprite_offset.y;
    let (color, image) = frame_visual(frame, images, texture_cache, missing_textures);
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
    missing_textures: &mut HashSet<u32>,
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
        let (color, image) = frame_visual(frame, images, texture_cache, missing_textures);
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
    missing_textures: &mut HashSet<u32>,
) -> (Color, Handle<Image>) {
    if let Some(WidgetData::StatusBar(sb)) = &frame.widget_data {
        let [r, g, b, a] = sb.color;
        return (Color::srgba(r, g, b, a * frame.effective_alpha), Handle::default());
    }
    if let Some(fdid) = frame_texture_fdid(frame)
        && let Some(handle) = load_texture(fdid, images, texture_cache, missing_textures)
    {
        // TODO: additive blend requires custom pipeline
        return (texture_tint(frame), handle);
    }
    (frame_color(frame), Handle::default())
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

// --- Text helpers ---

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
    fn button_disabled_text_grey() {
        let btn = crate::ui::widgets::button::ButtonData {
            state: ButtonState::Disabled,
            text: "Test".into(),
            ..Default::default()
        };
        let (_, _, color, _) = extract_button_text(&btn, 1.0);
        let Color::Srgba(srgba) = color else { panic!("expected srgba") };
        assert!(srgba.red < 0.6, "disabled should be grey");
    }
}
