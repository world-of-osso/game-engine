use bevy::log::info;
use game_engine::ui::screens::game_menu_component::{GameMenuView, GameMenuViewModel};
use game_engine::ui::screens::options_menu_component::{
    CameraOptionsView, HudOptionsView, KeybindingRowView, KeybindingsView, OptionsCategory,
    OptionsViewModel, SoundOptionsView,
};

use crate::client_options::{CameraOptions, HudOptions};
use game_engine::input_bindings::{
    BindingSection, InputAction, InputBinding, InputBindings, actions_for_section,
};
use crate::sound::SoundSettings;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DragCapture {
    None,
    Window,
    Slider(SliderField),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SliderField {
    MasterVolume,
    MusicVolume,
    AmbientVolume,
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
    pub draft_sound: SoundDraft,
    pub draft_camera: CameraDraft,
    pub draft_hud: HudDraft,
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
}

#[derive(Debug, Clone)]
pub struct CameraDraft {
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
    pub show_health_bars: bool,
    pub show_target_marker: bool,
    pub show_fps_overlay: bool,
}

#[derive(Clone)]
pub struct ApplySnapshot {
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
    if let Some(sound) = sound {
        SoundDraft {
            muted: sound.muted,
            music_enabled: sound.music_enabled,
            master_volume: sound.master_volume,
            music_volume: sound.music_volume,
            ambient_volume: sound.ambient_volume,
        }
    } else {
        let defaults = SoundSettings::default();
        SoundDraft {
            muted: defaults.muted,
            music_enabled: defaults.music_enabled,
            master_volume: defaults.master_volume,
            music_volume: defaults.music_volume,
            ambient_volume: defaults.ambient_volume,
        }
    }
}

pub fn camera_draft(camera: &CameraOptions) -> CameraDraft {
    CameraDraft {
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
        show_health_bars: hud.show_health_bars,
        show_target_marker: hud.show_target_marker,
        show_fps_overlay: hud.show_fps_overlay,
    }
}

pub fn build_view_model(model: &OverlayModel) -> GameMenuViewModel {
    GameMenuViewModel {
        logged_in: model.logged_in,
        view: model.view,
        options: OptionsViewModel {
            category: model.category,
            position: model.modal_position,
            sound: sound_view(&model.draft_sound),
            camera: camera_view(&model.draft_camera),
            hud: hud_view(&model.draft_hud),
            bindings: bindings_view(
                &model.draft_bindings,
                model.binding_section,
                current_capture_action(model.binding_capture),
            ),
        },
    }
}

fn sound_view(draft: &SoundDraft) -> SoundOptionsView {
    SoundOptionsView {
        muted: draft.muted,
        music_enabled: draft.music_enabled,
        master_volume: draft.master_volume,
        music_volume: draft.music_volume,
        ambient_volume: draft.ambient_volume,
    }
}

fn camera_view(draft: &CameraDraft) -> CameraOptionsView {
    CameraOptionsView {
        look_sensitivity: draft.look_sensitivity,
        invert_y: draft.invert_y,
        zoom_speed: draft.zoom_speed,
        follow_speed: draft.follow_speed,
        min_distance: draft.min_distance,
        max_distance: draft.max_distance,
    }
}

fn hud_view(draft: &HudDraft) -> HudOptionsView {
    HudOptionsView {
        show_minimap: draft.show_minimap,
        show_action_bars: draft.show_action_bars,
        show_nameplates: draft.show_nameplates,
        show_health_bars: draft.show_health_bars,
        show_target_marker: draft.show_target_marker,
        show_fps_overlay: draft.show_fps_overlay,
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
    match action.strip_prefix("options_slider:")? {
        "master_volume" => Some(SliderField::MasterVolume),
        "music_volume" => Some(SliderField::MusicVolume),
        "ambient_volume" => Some(SliderField::AmbientVolume),
        "look_sensitivity" => Some(SliderField::LookSensitivity),
        "zoom_speed" => Some(SliderField::ZoomSpeed),
        "follow_speed" => Some(SliderField::FollowSpeed),
        "min_distance" => Some(SliderField::MinDistance),
        "max_distance" => Some(SliderField::MaxDistance),
        _ => None,
    }
}

pub fn slider_bounds(field: SliderField) -> (f32, f32) {
    match field {
        SliderField::MasterVolume | SliderField::MusicVolume | SliderField::AmbientVolume => {
            (0.0, 1.0)
        }
        SliderField::LookSensitivity => (0.002, 0.03),
        SliderField::ZoomSpeed | SliderField::FollowSpeed => (2.0, 20.0),
        SliderField::MinDistance => (1.0, 10.0),
        SliderField::MaxDistance => (10.0, 60.0),
    }
}

pub fn apply_slider_value(field: SliderField, value: f32, model: &mut OverlayModel) {
    match field {
        SliderField::MasterVolume => model.draft_sound.master_volume = value,
        SliderField::MusicVolume => model.draft_sound.music_volume = value,
        SliderField::AmbientVolume => model.draft_sound.ambient_volume = value,
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
    match action.strip_prefix("options_category:")? {
        "graphics" => Some(OptionsCategory::Graphics),
        "sound" => Some(OptionsCategory::Sound),
        "camera" => Some(OptionsCategory::Camera),
        "interface" => Some(OptionsCategory::Interface),
        "hud" => Some(OptionsCategory::Hud),
        "controls" => Some(OptionsCategory::Controls),
        "accessibility" => Some(OptionsCategory::Accessibility),
        "keybindings" => Some(OptionsCategory::Keybindings),
        "macros" => Some(OptionsCategory::Macros),
        "socialaddons" => Some(OptionsCategory::SocialAddons),
        "advanced" => Some(OptionsCategory::Advanced),
        "support" => Some(OptionsCategory::Support),
        _ => None,
    }
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
    match key {
        "master_volume" => {
            model.draft_sound.master_volume =
                clamp_step(model.draft_sound.master_volume, 0.05 * step, 0.0, 1.0)
        }
        "music_volume" => {
            model.draft_sound.music_volume =
                clamp_step(model.draft_sound.music_volume, 0.05 * step, 0.0, 1.0)
        }
        "ambient_volume" => {
            model.draft_sound.ambient_volume =
                clamp_step(model.draft_sound.ambient_volume, 0.05 * step, 0.0, 1.0)
        }
        "look_sensitivity" => {
            model.draft_camera.look_sensitivity = clamp_step(
                model.draft_camera.look_sensitivity,
                0.001 * step,
                0.002,
                0.03,
            )
        }
        "zoom_speed" => {
            model.draft_camera.zoom_speed =
                clamp_step(model.draft_camera.zoom_speed, 0.5 * step, 2.0, 20.0)
        }
        "follow_speed" => {
            model.draft_camera.follow_speed =
                clamp_step(model.draft_camera.follow_speed, 0.5 * step, 2.0, 20.0)
        }
        "min_distance" => {
            model.draft_camera.min_distance =
                clamp_step(model.draft_camera.min_distance, 0.5 * step, 1.0, 10.0)
        }
        "max_distance" => {
            model.draft_camera.max_distance =
                clamp_step(model.draft_camera.max_distance, 1.0 * step, 10.0, 60.0)
        }
        _ => {}
    }
    normalize_camera_limits(&mut model.draft_camera);
}

pub fn apply_toggle(key: &str, model: &mut OverlayModel) {
    match key {
        "muted" => {
            model.draft_sound.muted = !model.draft_sound.muted;
            info!("Options toggle: muted -> {}", model.draft_sound.muted);
        }
        "music_enabled" => model.draft_sound.music_enabled = !model.draft_sound.music_enabled,
        "invert_y" => model.draft_camera.invert_y = !model.draft_camera.invert_y,
        "show_minimap" => model.draft_hud.show_minimap = !model.draft_hud.show_minimap,
        "show_action_bars" => model.draft_hud.show_action_bars = !model.draft_hud.show_action_bars,
        "show_nameplates" => model.draft_hud.show_nameplates = !model.draft_hud.show_nameplates,
        "show_health_bars" => model.draft_hud.show_health_bars = !model.draft_hud.show_health_bars,
        "show_target_marker" => {
            model.draft_hud.show_target_marker = !model.draft_hud.show_target_marker
        }
        "show_fps_overlay" => model.draft_hud.show_fps_overlay = !model.draft_hud.show_fps_overlay,
        _ => {}
    }
}

pub fn reset_category_defaults(model: &mut OverlayModel) {
    match model.category {
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
    model.committed_sound = model.draft_sound.clone();
    model.committed_camera = model.draft_camera.clone();
    model.committed_hud = model.draft_hud.clone();
    model.committed_bindings = model.draft_bindings.clone();
    ApplySnapshot {
        sound: model.draft_sound.clone(),
        camera: model.draft_camera.clone(),
        hud: model.draft_hud.clone(),
        bindings: model.draft_bindings.clone(),
        modal_position: model.modal_position,
    }
}

pub fn apply_sound_snapshot(sound: &mut SoundSettings, draft: &SoundDraft) {
    sound.muted = draft.muted;
    sound.music_enabled = draft.music_enabled;
    sound.master_volume = draft.master_volume;
    sound.music_volume = draft.music_volume;
    sound.ambient_volume = draft.ambient_volume;
}

pub fn apply_camera_snapshot(camera: &mut CameraOptions, draft: &CameraDraft) {
    camera.look_sensitivity = draft.look_sensitivity;
    camera.invert_y = draft.invert_y;
    camera.zoom_speed = draft.zoom_speed;
    camera.follow_speed = draft.follow_speed;
    camera.min_distance = draft.min_distance;
    camera.max_distance = draft.max_distance;
}

pub fn apply_hud_snapshot(hud: &mut HudOptions, draft: &HudDraft) {
    hud.show_minimap = draft.show_minimap;
    hud.show_action_bars = draft.show_action_bars;
    hud.show_nameplates = draft.show_nameplates;
    hud.show_health_bars = draft.show_health_bars;
    hud.show_target_marker = draft.show_target_marker;
    hud.show_fps_overlay = draft.show_fps_overlay;
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
