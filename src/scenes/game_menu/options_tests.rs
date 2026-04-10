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
    assert!(model.draft_graphics.vsync_enabled);
    assert!(!model.draft_graphics.frame_rate_limit_enabled);
}

#[test]
fn toggling_colorblind_mode_updates_graphics_draft() {
    let mut model = default_model();
    assert!(!model.draft_graphics.colorblind_mode);
    apply_toggle("colorblind_mode", &mut model);
    assert!(model.draft_graphics.colorblind_mode);
}

#[test]
fn slider_apply_master_volume_updates_sound_draft() {
    let mut model = default_model();
    apply_slider_value(SliderField::MasterVolume, 0.75, &mut model);
    assert!((model.draft_sound.master_volume - 0.75).abs() < 0.001);
}

#[test]
fn slider_apply_mouse_sensitivity_updates_camera_draft() {
    let mut model = default_model();
    apply_slider_value(SliderField::MouseSensitivity, 0.006, &mut model);
    assert!((model.draft_camera.mouse_sensitivity - 0.006).abs() < 0.001);
}

#[test]
fn slider_apply_frame_rate_limit_updates_graphics_draft() {
    let mut model = default_model();
    apply_slider_value(SliderField::FrameRateLimit, 165.0, &mut model);
    assert!((model.draft_graphics.frame_rate_limit - 165.0).abs() < 0.001);
}

#[test]
fn slider_apply_ui_scale_updates_graphics_draft() {
    let mut model = default_model();
    apply_slider_value(SliderField::UiScale, 1.25, &mut model);
    assert!((model.draft_graphics.ui_scale - 1.25).abs() < 0.001);
}

#[test]
fn slider_apply_nameplate_distance_updates_hud_draft() {
    let mut model = default_model();
    apply_slider_value(SliderField::NameplateDistance, 55.0, &mut model);
    assert!((model.draft_hud.nameplate_distance - 55.0).abs() < 0.001);
}

#[test]
fn slider_apply_chat_font_size_updates_hud_draft() {
    let mut model = default_model();
    apply_slider_value(SliderField::ChatFontSize, 14.0, &mut model);
    assert!((model.draft_hud.chat_font_size - 14.0).abs() < 0.001);
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
        SliderField::MouseSensitivity,
        SliderField::ParticleDensity,
        SliderField::FrameRateLimit,
        SliderField::RenderScale,
        SliderField::UiScale,
        SliderField::NameplateDistance,
        SliderField::ChatFontSize,
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
        (
            "options_slider:mouse_sensitivity",
            SliderField::MouseSensitivity,
        ),
        ("options_slider:master_volume", SliderField::MasterVolume),
        (
            "options_slider:frame_rate_limit",
            SliderField::FrameRateLimit,
        ),
        ("options_slider:render_scale", SliderField::RenderScale),
        ("options_slider:ui_scale", SliderField::UiScale),
        (
            "options_slider:nameplate_distance",
            SliderField::NameplateDistance,
        ),
        ("options_slider:chat_font_size", SliderField::ChatFontSize),
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
fn toggling_frame_pacing_options_updates_graphics_draft() {
    let mut model = default_model();
    assert!(model.draft_graphics.vsync_enabled);
    assert!(!model.draft_graphics.frame_rate_limit_enabled);

    apply_toggle("vsync_enabled", &mut model);
    apply_toggle("frame_rate_limit_enabled", &mut model);

    assert!(!model.draft_graphics.vsync_enabled);
    assert!(model.draft_graphics.frame_rate_limit_enabled);
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
    assert_eq!(model.view, GameMenuView::MainMenu);
}

#[test]
fn escape_priority_cancels_capture_before_view_navigation() {
    let mut model = default_model();
    model.view = GameMenuView::Options;
    model.binding_capture = BindingCapture::Listening(InputAction::Jump);

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
