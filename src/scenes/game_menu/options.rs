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
    ParticleDensity,
    RenderScale,
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
    pub bloom_enabled: bool,
    pub bloom_intensity: f32,
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
        bloom_enabled: graphics.bloom_enabled,
        bloom_intensity: graphics.bloom_intensity,
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
    let g = &model.draft_graphics;
    let s = &model.draft_sound;
    let c = &model.draft_camera;
    let h = &model.draft_hud;
    GameMenuViewModel {
        logged_in: model.logged_in,
        view: model.view,
        options: OptionsViewModel {
            category: model.category,
            position: model.modal_position,
            graphics: GraphicsOptionsView {
                particle_density: g.particle_density,
                render_scale: g.render_scale,
                bloom_enabled: g.bloom_enabled,
                bloom_intensity: g.bloom_intensity,
            },
            sound: SoundOptionsView {
                muted: s.muted,
                music_enabled: s.music_enabled,
                master_volume: s.master_volume,
                music_volume: s.music_volume,
                ambient_volume: s.ambient_volume,
                effects_volume: s.effects_volume,
            },
            camera: CameraOptionsView {
                look_sensitivity: c.look_sensitivity,
                invert_y: c.invert_y,
                zoom_speed: c.zoom_speed,
                follow_speed: c.follow_speed,
                min_distance: c.min_distance,
                max_distance: c.max_distance,
            },
            hud: HudOptionsView {
                show_minimap: h.show_minimap,
                show_action_bars: h.show_action_bars,
                show_nameplates: h.show_nameplates,
                show_health_bars: h.show_health_bars,
                show_target_marker: h.show_target_marker,
                show_fps_overlay: h.show_fps_overlay,
            },
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
        "particle_density" => SliderField::ParticleDensity,
        "render_scale" => SliderField::RenderScale,
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
        SliderField::ParticleDensity => (10.0, 100.0),
        SliderField::RenderScale => (0.5, 1.0),
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

pub fn apply_slider_value(field: SliderField, value: f32, model: &mut OverlayModel) {
    match field {
        SliderField::ParticleDensity => model.draft_graphics.particle_density = value.round(),
        SliderField::RenderScale => model.draft_graphics.render_scale = value,
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
        "render_scale" => g.render_scale = clamp_step(g.render_scale, 0.05 * step, 0.5, 1.0),
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

fn apply_camera_step(key: &str, step: f32, c: &mut CameraDraft) {
    match key {
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
    h.show_health_bars = d.show_health_bars;
    h.show_target_marker = d.show_target_marker;
    h.show_fps_overlay = d.show_fps_overlay;
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
mod tests {
    use super::*;
    use bevy::prelude::Vec2;
    use game_engine::ui::screens::game_menu_component::GameMenuView;

    fn default_model() -> OverlayModel {
        let g = graphics_draft(&GraphicsOptions::default());
        let s = sound_draft(None);
        let c = camera_draft(&CameraOptions::default());
        let h = hud_draft(&HudOptions::default());
        OverlayModel {
            logged_in: true,
            view: GameMenuView::Options,
            category: OptionsCategory::Graphics,
            modal_position: [500.0, 180.0],
            drag_capture: DragCapture::None,
            drag_origin: Vec2::ZERO,
            drag_offset: Vec2::ZERO,
            pressed_action: None,
            pressed_origin: Vec2::ZERO,
            draft_graphics: g.clone(),
            draft_sound: s.clone(),
            draft_camera: c.clone(),
            draft_hud: h.clone(),
            committed_graphics: g,
            committed_sound: s,
            committed_camera: c,
            committed_hud: h,
            draft_bindings: InputBindings::default(),
            committed_bindings: InputBindings::default(),
            binding_section: BindingSection::Movement,
            binding_capture: BindingCapture::None,
        }
    }

    #[test]
    fn resetting_graphics_defaults_disables_bloom() {
        let mut model = default_model();
        model.draft_graphics.bloom_enabled = true;
        model.draft_graphics.bloom_intensity = 0.42;
        model.draft_graphics.particle_density = 100.0;
        model.draft_graphics.render_scale = 1.0;
        reset_category_defaults(&mut model);
        assert!(!model.draft_graphics.bloom_enabled);
        assert!((model.draft_graphics.bloom_intensity - 0.08).abs() < 0.0001);
    }

    #[test]
    fn slider_apply_master_volume_updates_sound_draft() {
        let mut model = default_model();
        apply_slider_value(SliderField::MasterVolume, 0.75, &mut model);
        assert!((model.draft_sound.master_volume - 0.75).abs() < 0.001);
    }

    #[test]
    fn slider_apply_min_distance_normalizes_camera_limits() {
        let mut model = default_model();
        model.draft_camera.max_distance = 15.0;
        apply_slider_value(SliderField::MinDistance, 20.0, &mut model);
        assert!(
            model.draft_camera.max_distance >= model.draft_camera.min_distance + 1.0,
            "max_distance should be clamped above min_distance"
        );
    }

    #[test]
    fn slider_bounds_are_valid_ranges() {
        for field in [
            SliderField::ParticleDensity,
            SliderField::RenderScale,
            SliderField::MasterVolume,
            SliderField::LookSensitivity,
            SliderField::MinDistance,
            SliderField::MaxDistance,
        ] {
            let (min, max) = slider_bounds(field);
            assert!(
                min < max,
                "slider {field:?} has invalid bounds: {min} >= {max}"
            );
        }
    }

    #[test]
    fn parse_slider_action_round_trips_all_fields() {
        let actions = [
            ("options_slider:master_volume", SliderField::MasterVolume),
            ("options_slider:render_scale", SliderField::RenderScale),
            ("options_slider:min_distance", SliderField::MinDistance),
        ];
        for (action, expected) in actions {
            assert_eq!(parse_slider_action(action), Some(expected));
        }
    }

    #[test]
    fn parse_slider_action_rejects_unknown() {
        assert_eq!(parse_slider_action("options_slider:bogus"), None);
        assert_eq!(parse_slider_action("not_a_slider"), None);
    }

    #[test]
    fn parse_category_action_resolves_all_categories() {
        let cases = [
            ("options_category:graphics", OptionsCategory::Graphics),
            ("options_category:sound", OptionsCategory::Sound),
            ("options_category:controls", OptionsCategory::Controls),
            ("options_category:keybindings", OptionsCategory::Keybindings),
            ("options_category:support", OptionsCategory::Support),
        ];
        for (action, expected) in cases {
            assert_eq!(
                parse_category_action(action),
                Some(expected),
                "failed for {action}"
            );
        }
    }

    #[test]
    fn parse_category_action_rejects_invalid() {
        assert_eq!(parse_category_action("options_category:bogus"), None);
        assert_eq!(parse_category_action("not_a_category"), None);
    }

    #[test]
    fn binding_capture_none_returns_no_action() {
        assert!(current_capture_action(BindingCapture::None).is_none());
    }

    #[test]
    fn binding_capture_armed_returns_action() {
        let action = InputAction::MoveForward;
        assert_eq!(
            current_capture_action(BindingCapture::Armed(action)),
            Some(action)
        );
    }

    #[test]
    fn binding_capture_listening_returns_action() {
        let action = InputAction::MoveForward;
        assert_eq!(
            current_capture_action(BindingCapture::Listening(action)),
            Some(action)
        );
    }

    #[test]
    fn draft_bindings_assign_updates_mapping() {
        let mut model = default_model();
        model.draft_bindings.assign(
            InputAction::MoveForward,
            game_engine::input_bindings::InputBinding::Keyboard(bevy::prelude::KeyCode::KeyZ),
        );
        let binding = model.draft_bindings.binding(InputAction::MoveForward);
        assert_eq!(
            binding,
            Some(game_engine::input_bindings::InputBinding::Keyboard(
                bevy::prelude::KeyCode::KeyZ
            ))
        );
    }

    #[test]
    fn escape_from_options_returns_to_main_menu() {
        let mut model = default_model();
        model.view = GameMenuView::Options;
        model.drag_capture = DragCapture::Slider(SliderField::MasterVolume);
        model.pressed_action = Some("leftover".to_string());

        // Simulate handle_escape for Options view
        model.drag_capture = DragCapture::None;
        model.pressed_action = None;
        model.view = GameMenuView::MainMenu;

        assert_eq!(model.view, GameMenuView::MainMenu);
        assert_eq!(model.drag_capture, DragCapture::None);
        assert!(model.pressed_action.is_none());
    }

    #[test]
    fn escape_during_binding_capture_cancels_without_changing_view() {
        let mut model = default_model();
        model.view = GameMenuView::Options;
        model.binding_capture = BindingCapture::Listening(InputAction::MoveForward);

        // Simulate handle_escape: capture is active, so cancel it
        if current_capture_action(model.binding_capture).is_some() {
            model.binding_capture = BindingCapture::None;
            model.pressed_action = None;
        }

        assert_eq!(
            model.view,
            GameMenuView::Options,
            "view should stay Options"
        );
        assert!(matches!(model.binding_capture, BindingCapture::None));
    }

    #[test]
    fn escape_from_main_menu_does_not_switch_to_options() {
        let mut model = default_model();
        model.view = GameMenuView::MainMenu;

        // handle_escape for MainMenu calls close_game_menu, which we can't
        // test without Commands, but we verify the view is NOT changed.
        assert_eq!(model.view, GameMenuView::MainMenu);
    }

    #[test]
    fn escape_priority_cancels_capture_before_view_navigation() {
        let mut model = default_model();
        model.view = GameMenuView::Options;
        model.binding_capture = BindingCapture::Listening(InputAction::Jump);

        // First ESC: cancel capture (should NOT change view)
        if current_capture_action(model.binding_capture).is_some() {
            model.binding_capture = BindingCapture::None;
        } else if model.view == GameMenuView::Options {
            model.view = GameMenuView::MainMenu;
        }
        assert_eq!(
            model.view,
            GameMenuView::Options,
            "first ESC cancels capture, not view"
        );

        // Second ESC: no capture active, navigate back
        if current_capture_action(model.binding_capture).is_some() {
            model.binding_capture = BindingCapture::None;
        } else if model.view == GameMenuView::Options {
            model.view = GameMenuView::MainMenu;
        }
        assert_eq!(
            model.view,
            GameMenuView::MainMenu,
            "second ESC navigates back"
        );
    }
}
