use bevy::log::info;
use bevy::prelude::Vec2;
use game_engine::ui::options_menu_data;
pub use game_engine::ui::options_menu_data::{
    ApplySnapshot, BindingCapture, CameraDraft, GraphicsDraft, HudDraft, OptionsModel, SliderField,
    SoundDraft, apply_slider_value, apply_snapshot, apply_step, build_view_model,
    current_capture_action, parse_binding_clear_action, parse_binding_rebind_action,
    parse_binding_section_action, parse_category_action, parse_slider_action, parse_step_action,
    parse_toggle_action, reset_category_defaults, slider_bounds, slider_key,
};

use crate::client_options::{CameraOptions, GraphicsOptions, HudOptions};
use crate::sound::SoundSettings;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DragCapture {
    None,
    Window,
    Slider(SliderField),
}

#[derive(Debug, Clone)]
pub struct OverlayModel {
    pub options: OptionsModel,
    pub drag_capture: DragCapture,
    pub drag_origin: Vec2,
    pub drag_offset: Vec2,
    pub pressed_action: Option<String>,
    pub pressed_origin: Vec2,
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
        fov_degrees: camera.fov_degrees,
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
        nameplate_style: hud.nameplate_style,
        show_health_bars: hud.show_health_bars,
        show_target_marker: hud.show_target_marker,
        auto_loot: hud.auto_loot,
        soft_target_interact: hud.soft_target_interact,
        show_fps_overlay: hud.show_fps_overlay,
        chat_font_size: hud.chat_font_size,
    }
}

pub fn apply_toggle(key: &str, model: &mut OptionsModel) {
    if options_menu_data::apply_toggle(key, model) && key == "muted" {
        info!("Options toggle: muted -> {}", model.draft_sound.muted);
    }
}

pub fn apply_graphics_snapshot(graphics: &mut GraphicsOptions, draft: &GraphicsDraft) {
    let mut file = game_engine::client_options_data::GraphicsOptionsFile::default();
    options_menu_data::apply_graphics_file_snapshot(&mut file, draft);
    graphics.particle_density = file.particle_density;
    graphics.render_scale = file.render_scale;
    graphics.ui_scale = file.ui_scale;
    graphics.vsync_enabled = file.vsync_enabled;
    graphics.frame_rate_limit_enabled = file.frame_rate_limit_enabled;
    graphics.frame_rate_limit = file.frame_rate_limit;
    graphics.colorblind_mode = file.colorblind_mode;
    graphics.bloom_enabled = file.bloom_enabled;
    graphics.bloom_intensity = file.bloom_intensity;
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
    let mut file = game_engine::client_options_data::CameraOptionsFile::default();
    options_menu_data::apply_camera_file_snapshot(&mut file, d);
    c.mouse_sensitivity = file.mouse_sensitivity;
    c.look_sensitivity = file.look_sensitivity;
    c.invert_y = file.invert_y;
    c.fov_degrees = file.fov_degrees;
    c.zoom_speed = file.zoom_speed;
    c.follow_speed = file.follow_speed;
    c.min_distance = file.min_distance;
    c.max_distance = file.max_distance;
}

pub fn apply_hud_snapshot(h: &mut HudOptions, d: &HudDraft) {
    let mut file = game_engine::client_options_data::HudOptionsFile::default();
    options_menu_data::apply_hud_file_snapshot(&mut file, d);
    h.show_minimap = file.show_minimap;
    h.show_action_bars = file.show_action_bars;
    h.show_nameplates = file.show_nameplates;
    h.nameplate_style = file.nameplate_style;
    h.nameplate_distance = file.nameplate_distance;
    h.show_health_bars = file.show_health_bars;
    h.show_target_marker = file.show_target_marker;
    h.auto_loot = file.auto_loot;
    h.soft_target_interact = file.soft_target_interact;
    h.show_fps_overlay = file.show_fps_overlay;
    h.chat_font_size = file.chat_font_size;
}

#[cfg(test)]
#[path = "options_tests.rs"]
mod tests;
