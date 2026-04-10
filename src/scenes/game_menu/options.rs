use bevy::log::info;
use game_engine::ui::screens::game_menu_component::{GameMenuView, GameMenuViewModel};
use game_engine::ui::screens::options_menu_component::{
    CameraOptionsView, GraphicsOptionsView, HudOptionsView, KeybindingRowView, KeybindingsView,
    OptionsCategory, OptionsViewModel, SoundOptionsView,
};

use crate::client_options::{CameraOptions, GraphicsOptions, HudOptions};
use crate::sound::SoundSettings;
use game_engine::input_bindings::{
    BindingSection, InputAction, InputBinding, InputBindings, actions_for_section,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DragCapture {
    None,
    Window,
    Slider(SliderField),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SliderField {
    MouseSensitivity,
    ParticleDensity,
    FrameRateLimit,
    RenderScale,
    UiScale,
    NameplateDistance,
    ChatFontSize,
    BloomIntensity,
    MasterVolume,
    MusicVolume,
    AmbientVolume,
    EffectsVolume,
    LookSensitivity,
    ZoomSpeed,
    FollowSpeed,
    MinDistance,
    MaxDistance,
}

#[derive(Debug, Clone)]
pub struct OverlayModel {
    pub logged_in: bool,
    pub view: GameMenuView,
    pub category: OptionsCategory,
    pub modal_position: [f32; 2],
    pub drag_capture: DragCapture,
    pub drag_origin: bevy::prelude::Vec2,
    pub drag_offset: bevy::prelude::Vec2,
    pub pressed_action: Option<String>,
    pub pressed_origin: bevy::prelude::Vec2,
    pub draft_graphics: GraphicsDraft,
    pub draft_sound: SoundDraft,
    pub draft_camera: CameraDraft,
    pub draft_hud: HudDraft,
    pub committed_graphics: GraphicsDraft,
    pub committed_sound: SoundDraft,
    pub committed_camera: CameraDraft,
    pub committed_hud: HudDraft,
    pub draft_bindings: InputBindings,
    pub committed_bindings: InputBindings,
    pub binding_section: BindingSection,
    pub binding_capture: BindingCapture,
}

#[derive(Debug, Clone)]
pub struct SoundDraft {
    pub muted: bool,
    pub music_enabled: bool,
    pub master_volume: f32,
    pub music_volume: f32,
    pub ambient_volume: f32,
    pub effects_volume: f32,
}

#[derive(Debug, Clone)]
pub struct GraphicsDraft {
    pub particle_density: f32,
    pub render_scale: f32,
    pub ui_scale: f32,
    pub vsync_enabled: bool,
    pub frame_rate_limit_enabled: bool,
    pub frame_rate_limit: f32,
    pub colorblind_mode: bool,
    pub bloom_enabled: bool,
    pub bloom_intensity: f32,
}

#[derive(Debug, Clone)]
pub struct CameraDraft {
    pub mouse_sensitivity: f32,
    pub look_sensitivity: f32,
    pub invert_y: bool,
    pub zoom_speed: f32,
    pub follow_speed: f32,
    pub min_distance: f32,
    pub max_distance: f32,
}

#[derive(Debug, Clone)]
pub struct HudDraft {
    pub show_minimap: bool,
    pub show_action_bars: bool,
    pub show_nameplates: bool,
    pub nameplate_distance: f32,
    pub show_health_bars: bool,
    pub show_target_marker: bool,
    pub show_fps_overlay: bool,
    pub chat_font_size: f32,
}

#[derive(Clone)]
pub struct ApplySnapshot {
    pub graphics: GraphicsDraft,
    pub sound: SoundDraft,
    pub camera: CameraDraft,
    pub hud: HudDraft,
    pub bindings: InputBindings,
    pub modal_position: [f32; 2],
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BindingCapture {
    None,
    Armed(InputAction),
    Listening(InputAction),
}

pub fn sound_draft(sound: Option<&SoundSettings>) -> SoundDraft {
    let s = sound.cloned().unwrap_or_default();
    SoundDraft {
        muted: s.muted,
        music_enabled: s.music_enabled,
        master_volume: s.master_volume,
        music_volume: s.music_volume,
        ambient_volume: s.ambient_volume,
        effects_volume: s.effects_volume,
    }
}

pub fn graphics_draft(graphics: &GraphicsOptions) -> GraphicsDraft {
    GraphicsDraft {
        particle_density: graphics.particle_density as f32,
        render_scale: graphics.render_scale,
        ui_scale: graphics.ui_scale,
        vsync_enabled: graphics.vsync_enabled,
        frame_rate_limit_enabled: graphics.frame_rate_limit_enabled,
        frame_rate_limit: f32::from(graphics.frame_rate_limit),
        colorblind_mode: graphics.colorblind_mode,
        bloom_enabled: graphics.bloom_enabled,
        bloom_intensity: graphics.bloom_intensity,
    }
}

pub fn camera_draft(camera: &CameraOptions) -> CameraDraft {
    CameraDraft {
        mouse_sensitivity: camera.mouse_sensitivity,
        look_sensitivity: camera.look_sensitivity,
        invert_y: camera.invert_y,
        zoom_speed: camera.zoom_speed,
        follow_speed: camera.follow_speed,
        min_distance: camera.min_distance,
        max_distance: camera.max_distance,
    }
}

pub fn hud_draft(hud: &HudOptions) -> HudDraft {
    HudDraft {
        show_minimap: hud.show_minimap,
        show_action_bars: hud.show_action_bars,
        show_nameplates: hud.show_nameplates,
        nameplate_distance: hud.nameplate_distance,
        show_health_bars: hud.show_health_bars,
        show_target_marker: hud.show_target_marker,
        show_fps_overlay: hud.show_fps_overlay,
        chat_font_size: hud.chat_font_size,
    }
}

fn graphics_to_view(g: &GraphicsDraft) -> GraphicsOptionsView {
    GraphicsOptionsView {
        particle_density: g.particle_density,
        render_scale: g.render_scale,
        ui_scale: g.ui_scale,
        vsync_enabled: g.vsync_enabled,
        frame_rate_limit_enabled: g.frame_rate_limit_enabled,
        frame_rate_limit: g.frame_rate_limit,
        colorblind_mode: g.colorblind_mode,
        bloom_enabled: g.bloom_enabled,
        bloom_intensity: g.bloom_intensity,
    }
}

fn sound_to_view(s: &SoundDraft) -> SoundOptionsView {
    SoundOptionsView {
        muted: s.muted,
        music_enabled: s.music_enabled,
        master_volume: s.master_volume,
        music_volume: s.music_volume,
        ambient_volume: s.ambient_volume,
        effects_volume: s.effects_volume,
    }
}

fn camera_to_view(c: &CameraDraft) -> CameraOptionsView {
    CameraOptionsView {
        mouse_sensitivity: c.mouse_sensitivity,
        look_sensitivity: c.look_sensitivity,
        invert_y: c.invert_y,
        zoom_speed: c.zoom_speed,
        follow_speed: c.follow_speed,
        min_distance: c.min_distance,
        max_distance: c.max_distance,
    }
}

fn hud_to_view(h: &HudDraft) -> HudOptionsView {
    HudOptionsView {
        show_minimap: h.show_minimap,
        show_action_bars: h.show_action_bars,
        show_nameplates: h.show_nameplates,
        nameplate_distance: h.nameplate_distance,
        show_health_bars: h.show_health_bars,
        show_target_marker: h.show_target_marker,
        show_fps_overlay: h.show_fps_overlay,
        chat_font_size: h.chat_font_size,
    }
}

pub fn build_view_model(model: &OverlayModel) -> GameMenuViewModel {
    GameMenuViewModel {
        logged_in: model.logged_in,
        view: model.view,
        options: OptionsViewModel {
            category: model.category,
            position: model.modal_position,
            graphics: graphics_to_view(&model.draft_graphics),
            sound: sound_to_view(&model.draft_sound),
            camera: camera_to_view(&model.draft_camera),
            hud: hud_to_view(&model.draft_hud),
            bindings: bindings_view(
                &model.draft_bindings,
                model.binding_section,
                current_capture_action(model.binding_capture),
            ),
        },
    }
}

fn bindings_view(
    bindings: &InputBindings,
    section: BindingSection,
    capture_action: Option<InputAction>,
) -> KeybindingsView {
    let rows = actions_for_section(section)
        .iter()
        .map(|action| KeybindingRowView {
            action: *action,
            label: action.label().to_string(),
            binding_text: bindings
                .binding(*action)
                .map(InputBinding::display)
                .unwrap_or_else(|| "Unbound".to_string()),
            capturing: capture_action == Some(*action),
            can_clear: bindings.binding(*action).is_some(),
        })
        .collect();
    KeybindingsView {
        section,
        capture_action,
        rows,
    }
}

pub fn parse_slider_action(action: &str) -> Option<SliderField> {
    Some(match action.strip_prefix("options_slider:")? {
        "mouse_sensitivity" => SliderField::MouseSensitivity,
        "particle_density" => SliderField::ParticleDensity,
        "frame_rate_limit" => SliderField::FrameRateLimit,
        "render_scale" => SliderField::RenderScale,
        "ui_scale" => SliderField::UiScale,
        "nameplate_distance" => SliderField::NameplateDistance,
        "chat_font_size" => SliderField::ChatFontSize,
        "bloom_intensity" => SliderField::BloomIntensity,
        "master_volume" => SliderField::MasterVolume,
        "music_volume" => SliderField::MusicVolume,
        "ambient_volume" => SliderField::AmbientVolume,
        "effects_volume" => SliderField::EffectsVolume,
        "look_sensitivity" => SliderField::LookSensitivity,
        "zoom_speed" => SliderField::ZoomSpeed,
        "follow_speed" => SliderField::FollowSpeed,
        "min_distance" => SliderField::MinDistance,
        "max_distance" => SliderField::MaxDistance,
        _ => return None,
    })
}

pub fn slider_bounds(field: SliderField) -> (f32, f32) {
    match field {
        SliderField::MouseSensitivity => mouse_sensitivity_range(),
        SliderField::ParticleDensity => (10.0, 100.0),
        SliderField::FrameRateLimit => frame_rate_limit_range(),
        SliderField::RenderScale => (0.5, 1.0),
        SliderField::UiScale => ui_scale_range(),
        SliderField::NameplateDistance => nameplate_distance_range(),
        SliderField::ChatFontSize => chat_font_size_range(),
        SliderField::BloomIntensity => (0.0, 1.0),
        SliderField::MasterVolume
        | SliderField::MusicVolume
        | SliderField::AmbientVolume
        | SliderField::EffectsVolume => (0.0, 1.0),
        SliderField::LookSensitivity => (0.002, 0.03),
        SliderField::ZoomSpeed | SliderField::FollowSpeed => (2.0, 20.0),
        SliderField::MinDistance => (1.0, 10.0),
        SliderField::MaxDistance => (10.0, 60.0),
    }
}

fn mouse_sensitivity_range() -> (f32, f32) {
    (
        crate::client_options::MIN_MOUSE_SENSITIVITY,
        crate::client_options::MAX_MOUSE_SENSITIVITY,
    )
}

fn frame_rate_limit_range() -> (f32, f32) {
    (
        f32::from(crate::client_options::MIN_FRAME_RATE_LIMIT),
        f32::from(crate::client_options::MAX_FRAME_RATE_LIMIT),
    )
}

fn ui_scale_range() -> (f32, f32) {
    (
        crate::client_options::MIN_UI_SCALE,
        crate::client_options::MAX_UI_SCALE,
    )
}

fn nameplate_distance_range() -> (f32, f32) {
    (
        crate::client_options::MIN_NAMEPLATE_DISTANCE,
        crate::client_options::MAX_NAMEPLATE_DISTANCE,
    )
}

fn chat_font_size_range() -> (f32, f32) {
    (
        crate::client_options::MIN_CHAT_FONT_SIZE,
        crate::client_options::MAX_CHAT_FONT_SIZE,
    )
}

pub fn apply_slider_value(field: SliderField, value: f32, model: &mut OverlayModel) {
    match field {
        SliderField::MouseSensitivity => model.draft_camera.mouse_sensitivity = value,
        SliderField::ParticleDensity => model.draft_graphics.particle_density = value.round(),
        SliderField::FrameRateLimit => model.draft_graphics.frame_rate_limit = value.round(),
        SliderField::RenderScale => model.draft_graphics.render_scale = value,
        SliderField::UiScale => model.draft_graphics.ui_scale = value,
        SliderField::NameplateDistance => model.draft_hud.nameplate_distance = value.round(),
        SliderField::ChatFontSize => model.draft_hud.chat_font_size = value.round(),
        SliderField::BloomIntensity => model.draft_graphics.bloom_intensity = value,
        SliderField::MasterVolume => model.draft_sound.master_volume = value,
        SliderField::MusicVolume => model.draft_sound.music_volume = value,
        SliderField::AmbientVolume => model.draft_sound.ambient_volume = value,
        SliderField::EffectsVolume => model.draft_sound.effects_volume = value,
        SliderField::LookSensitivity => model.draft_camera.look_sensitivity = value,
        SliderField::ZoomSpeed => model.draft_camera.zoom_speed = value,
        SliderField::FollowSpeed => model.draft_camera.follow_speed = value,
        SliderField::MinDistance => {
            model.draft_camera.min_distance = value;
            normalize_camera_limits(&mut model.draft_camera);
        }
        SliderField::MaxDistance => {
            model.draft_camera.max_distance = value;
            normalize_camera_limits(&mut model.draft_camera);
        }
    }
}

pub fn parse_category_action(action: &str) -> Option<OptionsCategory> {
    let key = action.strip_prefix("options_category:")?;
    OptionsCategory::ALL
        .iter()
        .find(|c| c.key() == key)
        .copied()
}

pub fn parse_binding_section_action(action: &str) -> Option<BindingSection> {
    BindingSection::from_key(action.strip_prefix("options_binding_section:")?)
}

pub fn parse_binding_rebind_action(action: &str) -> Option<InputAction> {
    InputAction::from_key(action.strip_prefix("options_binding_rebind:")?)
}

pub fn parse_binding_clear_action(action: &str) -> Option<InputAction> {
    InputAction::from_key(action.strip_prefix("options_binding_clear:")?)
}

pub fn parse_step_action(action: &str) -> Option<(&str, i32)> {
    let mut parts = action.strip_prefix("options_step:")?.split(':');
    let key = parts.next()?;
    let delta = parts.next()?.parse().ok()?;
    Some((key, delta))
}

pub fn parse_toggle_action(action: &str) -> Option<&str> {
    action.strip_prefix("options_toggle:")
}

pub fn apply_step(key: &str, delta: i32, model: &mut OverlayModel) {
    let step = delta as f32;
    if apply_graphics_step(key, step, &mut model.draft_graphics) {
        return;
    }
    if apply_hud_step(key, step, &mut model.draft_hud) {
        return;
    }
    if apply_sound_step(key, step, &mut model.draft_sound) {
        return;
    }
    apply_camera_step(key, step, &mut model.draft_camera);
}

fn apply_graphics_step(key: &str, step: f32, g: &mut GraphicsDraft) -> bool {
    match key {
        "particle_density" => {
            g.particle_density = clamp_step(g.particle_density, 5.0 * step, 10.0, 100.0).round()
        }
        "frame_rate_limit" => {
            g.frame_rate_limit = clamp_step(
                g.frame_rate_limit,
                10.0 * step,
                f32::from(crate::client_options::MIN_FRAME_RATE_LIMIT),
                f32::from(crate::client_options::MAX_FRAME_RATE_LIMIT),
            )
            .round()
        }
        "render_scale" => g.render_scale = clamp_step(g.render_scale, 0.05 * step, 0.5, 1.0),
        "ui_scale" => {
            g.ui_scale = clamp_step(
                g.ui_scale,
                0.05 * step,
                crate::client_options::MIN_UI_SCALE,
                crate::client_options::MAX_UI_SCALE,
            )
        }
        "bloom_intensity" => {
            g.bloom_intensity = clamp_step(g.bloom_intensity, 0.05 * step, 0.0, 1.0)
        }
        _ => return false,
    }
    true
}

fn apply_sound_step(key: &str, step: f32, sound: &mut SoundDraft) -> bool {
    let field = match key {
        "master_volume" => &mut sound.master_volume,
        "music_volume" => &mut sound.music_volume,
        "ambient_volume" => &mut sound.ambient_volume,
        "effects_volume" => &mut sound.effects_volume,
        _ => return false,
    };
    *field = clamp_step(*field, 0.05 * step, 0.0, 1.0);
    true
}

fn apply_hud_step(key: &str, step: f32, hud: &mut HudDraft) -> bool {
    match key {
        "nameplate_distance" => {
            hud.nameplate_distance = clamp_step(
                hud.nameplate_distance,
                5.0 * step,
                crate::client_options::MIN_NAMEPLATE_DISTANCE,
                crate::client_options::MAX_NAMEPLATE_DISTANCE,
            )
            .round();
            true
        }
        "chat_font_size" => {
            hud.chat_font_size = clamp_step(
                hud.chat_font_size,
                step,
                crate::client_options::MIN_CHAT_FONT_SIZE,
                crate::client_options::MAX_CHAT_FONT_SIZE,
            )
            .round();
            true
        }
        _ => false,
    }
}

fn apply_camera_step(key: &str, step: f32, c: &mut CameraDraft) {
    match key {
        "mouse_sensitivity" => {
            c.mouse_sensitivity = clamp_step(
                c.mouse_sensitivity,
                0.0005 * step,
                crate::client_options::MIN_MOUSE_SENSITIVITY,
                crate::client_options::MAX_MOUSE_SENSITIVITY,
            )
        }
        "look_sensitivity" => {
            c.look_sensitivity = clamp_step(c.look_sensitivity, 0.001 * step, 0.002, 0.03)
        }
        "zoom_speed" => c.zoom_speed = clamp_step(c.zoom_speed, 0.5 * step, 2.0, 20.0),
        "follow_speed" => c.follow_speed = clamp_step(c.follow_speed, 0.5 * step, 2.0, 20.0),
        "min_distance" => c.min_distance = clamp_step(c.min_distance, 0.5 * step, 1.0, 10.0),
        "max_distance" => c.max_distance = clamp_step(c.max_distance, step, 10.0, 60.0),
        _ => return,
    }
    normalize_camera_limits(c);
}

pub fn apply_toggle(key: &str, model: &mut OverlayModel) {
    let toggled = match key {
        "bloom_enabled" => {
            model.draft_graphics.bloom_enabled = !model.draft_graphics.bloom_enabled;
            true
        }
        "vsync_enabled" => {
            model.draft_graphics.vsync_enabled = !model.draft_graphics.vsync_enabled;
            true
        }
        "frame_rate_limit_enabled" => {
            model.draft_graphics.frame_rate_limit_enabled =
                !model.draft_graphics.frame_rate_limit_enabled;
            true
        }
        "colorblind_mode" => {
            model.draft_graphics.colorblind_mode = !model.draft_graphics.colorblind_mode;
            true
        }
        "muted" => {
            model.draft_sound.muted = !model.draft_sound.muted;
            true
        }
        "music_enabled" => {
            model.draft_sound.music_enabled = !model.draft_sound.music_enabled;
            true
        }
        "invert_y" => {
            model.draft_camera.invert_y = !model.draft_camera.invert_y;
            true
        }
        _ => apply_hud_toggle(key, &mut model.draft_hud),
    };
    if toggled && key == "muted" {
        info!("Options toggle: muted -> {}", model.draft_sound.muted);
    }
}

fn apply_hud_toggle(key: &str, hud: &mut HudDraft) -> bool {
    match key {
        "show_minimap" => hud.show_minimap = !hud.show_minimap,
        "show_action_bars" => hud.show_action_bars = !hud.show_action_bars,
        "show_nameplates" => hud.show_nameplates = !hud.show_nameplates,
        "show_health_bars" => hud.show_health_bars = !hud.show_health_bars,
        "show_target_marker" => hud.show_target_marker = !hud.show_target_marker,
        "show_fps_overlay" => hud.show_fps_overlay = !hud.show_fps_overlay,
        _ => return false,
    }
    true
}

pub fn reset_category_defaults(model: &mut OverlayModel) {
    match model.category {
        OptionsCategory::Graphics => {
            model.draft_graphics = graphics_draft(&GraphicsOptions::default())
        }
        OptionsCategory::Sound => model.draft_sound = sound_draft(None),
        OptionsCategory::Camera => model.draft_camera = camera_draft(&CameraOptions::default()),
        OptionsCategory::Interface | OptionsCategory::Hud => {
            model.draft_hud = hud_draft(&HudOptions::default())
        }
        OptionsCategory::Keybindings => model.draft_bindings.reset_section(model.binding_section),
        _ => {}
    }
}

pub fn apply_snapshot(model: &mut OverlayModel) -> ApplySnapshot {
    model.committed_graphics = model.draft_graphics.clone();
    model.committed_sound = model.draft_sound.clone();
    model.committed_camera = model.draft_camera.clone();
    model.committed_hud = model.draft_hud.clone();
    model.committed_bindings = model.draft_bindings.clone();
    ApplySnapshot {
        graphics: model.draft_graphics.clone(),
        sound: model.draft_sound.clone(),
        camera: model.draft_camera.clone(),
        hud: model.draft_hud.clone(),
        bindings: model.draft_bindings.clone(),
        modal_position: model.modal_position,
    }
}

pub fn apply_graphics_snapshot(graphics: &mut GraphicsOptions, draft: &GraphicsDraft) {
    graphics.particle_density = draft.particle_density.round().clamp(10.0, 100.0) as u8;
    graphics.render_scale = draft.render_scale.clamp(0.5, 1.0);
    graphics.ui_scale = draft.ui_scale.clamp(
        crate::client_options::MIN_UI_SCALE,
        crate::client_options::MAX_UI_SCALE,
    );
    graphics.vsync_enabled = draft.vsync_enabled;
    graphics.frame_rate_limit_enabled = draft.frame_rate_limit_enabled;
    graphics.frame_rate_limit = draft.frame_rate_limit.round().clamp(
        f32::from(crate::client_options::MIN_FRAME_RATE_LIMIT),
        f32::from(crate::client_options::MAX_FRAME_RATE_LIMIT),
    ) as u16;
    graphics.colorblind_mode = draft.colorblind_mode;
    graphics.bloom_enabled = draft.bloom_enabled;
    graphics.bloom_intensity = draft.bloom_intensity.clamp(0.0, 1.0);
}

pub fn apply_sound_snapshot(s: &mut SoundSettings, d: &SoundDraft) {
    s.muted = d.muted;
    s.music_enabled = d.music_enabled;
    s.master_volume = d.master_volume;
    s.music_volume = d.music_volume;
    s.ambient_volume = d.ambient_volume;
    s.effects_volume = d.effects_volume;
}

pub fn apply_camera_snapshot(c: &mut CameraOptions, d: &CameraDraft) {
    c.mouse_sensitivity = d.mouse_sensitivity.clamp(
        crate::client_options::MIN_MOUSE_SENSITIVITY,
        crate::client_options::MAX_MOUSE_SENSITIVITY,
    );
    c.look_sensitivity = d.look_sensitivity;
    c.invert_y = d.invert_y;
    c.zoom_speed = d.zoom_speed;
    c.follow_speed = d.follow_speed;
    c.min_distance = d.min_distance;
    c.max_distance = d.max_distance;
}

pub fn apply_hud_snapshot(h: &mut HudOptions, d: &HudDraft) {
    h.show_minimap = d.show_minimap;
    h.show_action_bars = d.show_action_bars;
    h.show_nameplates = d.show_nameplates;
    h.nameplate_distance = d
        .nameplate_distance
        .clamp(
            crate::client_options::MIN_NAMEPLATE_DISTANCE,
            crate::client_options::MAX_NAMEPLATE_DISTANCE,
        )
        .round();
    h.show_health_bars = d.show_health_bars;
    h.show_target_marker = d.show_target_marker;
    h.show_fps_overlay = d.show_fps_overlay;
    h.chat_font_size = d
        .chat_font_size
        .clamp(
            crate::client_options::MIN_CHAT_FONT_SIZE,
            crate::client_options::MAX_CHAT_FONT_SIZE,
        )
        .round();
}

pub fn current_capture_action(capture: BindingCapture) -> Option<InputAction> {
    match capture {
        BindingCapture::None => None,
        BindingCapture::Armed(action) | BindingCapture::Listening(action) => Some(action),
    }
}

fn clamp_step(value: f32, delta: f32, min: f32, max: f32) -> f32 {
    (value + delta).clamp(min, max)
}

fn normalize_camera_limits(camera: &mut CameraDraft) {
    camera.max_distance = camera.max_distance.max(camera.min_distance + 1.0);
}

#[cfg(test)]
#[path = "options_tests.rs"]
mod tests;
