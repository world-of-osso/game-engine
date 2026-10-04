use game_engine_core::client_options_data::{
    CameraOptionsFile, GraphicsOptionsFile, HudOptionsFile, SoundOptionsFile,
};
use game_engine_core::input_bindings_data::{BindingSection, InputAction, InputBindingsData};
use game_engine_ui_model::game_menu_component::GameMenuView;
use game_engine_ui_model::options_menu_component::OptionsCategory;
use game_engine_ui_model::options_menu_data::*;

fn model() -> OptionsModel {
    let graphics = graphics_draft_from_file(&GraphicsOptionsFile::default());
    let sound = sound_draft_from_file(&SoundOptionsFile::default());
    let camera = camera_draft_from_file(&CameraOptionsFile::default());
    let hud = hud_draft_from_file(&HudOptionsFile::default());
    OptionsModel {
        logged_in: true,
        view: GameMenuView::Options,
        category: OptionsCategory::Graphics,
        modal_position: [42.0, 24.0],
        draft_graphics: graphics.clone(),
        committed_graphics: graphics,
        draft_sound: sound.clone(),
        committed_sound: sound,
        draft_camera: camera.clone(),
        committed_camera: camera,
        draft_hud: hud.clone(),
        committed_hud: hud,
        draft_bindings: InputBindingsData::default(),
        committed_bindings: InputBindingsData::default(),
        binding_section: BindingSection::Movement,
        binding_capture: BindingCapture::None,
        layout: Default::default(),
    }
}

#[test]
fn graphics_actions_project_and_commit_clamped_file() {
    let mut m = model();
    apply_slider_value(
        parse_slider_action("options_slider:frame_rate_limit").unwrap(),
        165.3,
        &mut m,
    );
    apply_step("frame_rate_limit", 1, &mut m);
    assert!(apply_toggle("bloom_enabled", &mut m));
    let view = build_view_model(&m);
    assert_eq!(view.options.graphics.frame_rate_limit, 175.0);
    assert!(view.options.graphics.bloom_enabled);
    assert!(!m.committed_graphics.bloom_enabled);
    let snapshot = apply_snapshot(&mut m);
    let mut output = GraphicsOptionsFile::default();
    apply_graphics_file_snapshot(&mut output, &snapshot.graphics);
    assert_eq!(output.frame_rate_limit, 175);
    assert!(output.bloom_enabled);
    assert_eq!(snapshot.modal_position, [42.0, 24.0]);
}

#[test]
fn camera_and_nameplate_actions_preserve_existing_bounds_and_resets() {
    let mut m = model();
    m.category = OptionsCategory::Camera;
    apply_slider_value(SliderField::MinDistance, 45.0, &mut m);
    assert_eq!(m.draft_camera.max_distance, 46.0);
    reset_category_defaults(&mut m);
    assert_eq!(
        m.draft_camera.min_distance,
        CameraOptionsFile::default().min_distance
    );
    m.category = OptionsCategory::Nameplates;
    let slider = parse_slider_action("options_slider:nameplate_health_width").unwrap();
    apply_slider_value(slider, 150.3, &mut m);
    assert!(apply_toggle("nameplate_show_border", &mut m));
    let view = build_view_model(&m);
    assert_eq!(view.options.hud.nameplate_style.health_width, 150.0);
    let snapshot = apply_snapshot(&mut m);
    let mut hud = HudOptionsFile::default();
    apply_hud_file_snapshot(&mut hud, &snapshot.hud);
    assert_eq!(hud.nameplate_style.health_width, 150.0);
    reset_category_defaults(&mut m);
    assert_eq!(
        m.draft_hud.nameplate_style,
        HudOptionsFile::default().nameplate_style
    );
}

#[test]
fn sound_and_bindings_actions_project_reset_and_commit() {
    let mut m = model();
    m.category = OptionsCategory::Sound;
    assert!(apply_toggle("muted", &mut m));
    apply_step("master_volume", -2, &mut m);
    assert!(build_view_model(&m).options.sound.muted);
    reset_category_defaults(&mut m);
    assert!(!m.draft_sound.muted);
    assert_eq!(
        m.draft_sound.master_volume,
        SoundOptionsFile::default().master_volume
    );
    m.category = OptionsCategory::Keybindings;
    m.binding_section = BindingSection::Movement;
    m.binding_capture = BindingCapture::Listening(InputAction::MoveForward);
    let binding = build_view_model(&m).options.bindings;
    assert_eq!(binding.capture_action, Some(InputAction::MoveForward));
    m.draft_bindings.clear(InputAction::MoveForward);
    assert!(
        build_view_model(&m)
            .options
            .bindings
            .rows
            .iter()
            .any(|row| row.action == InputAction::MoveForward && row.binding_text.is_none())
    );
    reset_category_defaults(&mut m);
    assert_eq!(
        m.draft_bindings.binding(InputAction::MoveForward),
        InputBindingsData::default().binding(InputAction::MoveForward)
    );
    let snapshot = apply_snapshot(&mut m);
    assert_eq!(
        snapshot.bindings.binding(InputAction::MoveForward),
        m.committed_bindings.binding(InputAction::MoveForward)
    );
}

#[test]
fn nameplate_thickness_toggles_are_independent_and_commit_on_apply() {
    use game_engine_core::nameplate_style_data::NameplateBarThickness::{Thick, Thin};
    use game_engine_core::nameplate_style_data::NameplateStyle;
    let presets = |style: &NameplateStyle| (style.health_preset(), style.cast_preset());
    let mut m = model();
    m.category = OptionsCategory::Nameplates;
    assert_eq!(presets(&m.draft_hud.nameplate_style), (Thick, Thick));
    assert!(apply_toggle("nameplate_health_thickness", &mut m));
    assert_eq!(presets(&m.draft_hud.nameplate_style), (Thin, Thick));
    assert_eq!(m.draft_hud.nameplate_style.health_height, 10.0);
    assert_eq!(presets(&m.committed_hud.nameplate_style), (Thick, Thick));
    assert!(apply_toggle("nameplate_spellbar_thickness", &mut m));
    assert_eq!(
        presets(&build_view_model(&m).options.hud.nameplate_style),
        (Thin, Thin)
    );
    let snapshot = apply_snapshot(&mut m);
    let mut hud = HudOptionsFile::default();
    apply_hud_file_snapshot(&mut hud, &snapshot.hud);
    assert_eq!(presets(&hud.nameplate_style), (Thin, Thin));
    assert_eq!(hud.nameplate_style.cast_height, 6.0);
    reset_category_defaults(&mut m);
    assert_eq!(presets(&m.draft_hud.nameplate_style), (Thick, Thick));
}

#[test]
fn frame_pacing_toggles_flip_their_fields_and_commit_to_the_file() {
    let mut m = model();
    assert!(m.draft_graphics.vsync_enabled);
    assert!(!m.draft_graphics.frame_rate_limit_enabled);
    assert!(apply_toggle("vsync_enabled", &mut m));
    assert!(apply_toggle("frame_rate_limit_enabled", &mut m));
    let view = build_view_model(&m);
    assert!(!view.options.graphics.vsync_enabled);
    assert!(view.options.graphics.frame_rate_limit_enabled);
    assert!(m.committed_graphics.vsync_enabled);
    let snapshot = apply_snapshot(&mut m);
    let mut output = GraphicsOptionsFile::default();
    apply_graphics_file_snapshot(&mut output, &snapshot.graphics);
    assert!(!output.vsync_enabled);
    assert!(output.frame_rate_limit_enabled);
    assert!(!apply_toggle("bogus_toggle", &mut m));
}

#[test]
fn every_slider_action_round_trips_its_key_and_has_a_valid_range() {
    let keys = [
        "mouse_sensitivity",
        "fov_degrees",
        "particle_density",
        "frame_rate_limit",
        "render_scale",
        "ui_scale",
        "nameplate_distance",
        "chat_font_size",
        "bloom_intensity",
        "master_volume",
        "music_volume",
        "ambient_volume",
        "effects_volume",
        "look_sensitivity",
        "zoom_speed",
        "follow_speed",
        "min_distance",
        "max_distance",
        "nameplate_health_width",
        "nameplate_neutral_g",
    ];
    for key in keys {
        let field = parse_slider_action(&format!("options_slider:{key}"))
            .unwrap_or_else(|| panic!("{key} does not parse"));
        assert_eq!(slider_key(field), key);
        let (min, max) = slider_bounds(field);
        assert!(min < max, "{key} has invalid bounds: {min} >= {max}");
    }
    assert_eq!(parse_slider_action("options_slider:bogus"), None);
    assert_eq!(parse_slider_action("master_volume"), None);
}

#[test]
fn category_actions_resolve_every_category_and_reject_unknown() {
    for category in OptionsCategory::ALL {
        let action = format!("options_category:{}", category.key());
        assert_eq!(parse_category_action(&action), Some(category), "{action}");
    }
    assert_eq!(parse_category_action("options_category:bogus"), None);
    assert_eq!(parse_category_action("graphics"), None);
}

#[test]
fn interact_key_icons_choice_projects_and_commits_the_icon_cvars() {
    use game_engine_core::soft_target_data::InteractKeyIcons;
    let mut m = model();
    m.category = OptionsCategory::Accessibility;
    assert_eq!(
        build_view_model(&m).options.hud.interact_key_icons,
        InteractKeyIcons::Default
    );
    let action = parse_toggle_action("options_toggle:interact_key_icons:2").unwrap();
    assert!(apply_toggle(action, &mut m));
    assert_eq!(
        build_view_model(&m).options.hud.interact_key_icons,
        InteractKeyIcons::ShowAll
    );
    assert!(!apply_toggle("interact_key_icons:9", &mut m));
    let snapshot = apply_snapshot(&mut m);
    let mut output = HudOptionsFile::default();
    apply_hud_file_snapshot(&mut output, &snapshot.hud);
    assert!(output.soft_target.icon_game_object && output.soft_target.low_priority_icons);
    assert_eq!(
        output.soft_target.interact_key_icons(),
        InteractKeyIcons::ShowAll
    );
}

/// Combat "Personal Resource Display" is `nameplateShowSelf`, off by default (cvars.yaml
/// `nameplateShowSelf: '0'`); toggling it persists through Apply.
#[test]
fn personal_resource_display_toggle_persists() {
    let mut m = model();
    m.category = OptionsCategory::Hud;
    assert!(!build_view_model(&m).options.hud.personal_resource_display);
    assert!(apply_toggle("personal_resource_display", &mut m));
    assert!(build_view_model(&m).options.hud.personal_resource_display);
    let snapshot = apply_snapshot(&mut m);
    let mut hud = HudOptionsFile::default();
    apply_hud_file_snapshot(&mut hud, &snapshot.hud);
    assert!(hud.personal_resource_display);
}

#[test]
fn status_text_choice_projects_and_commits_status_text_display() {
    use game_engine_core::status_text_data::StatusTextDisplay;
    let mut m = model();
    m.category = OptionsCategory::Interface;
    assert_eq!(
        build_view_model(&m).options.hud.status_text_display,
        StatusTextDisplay::None
    );
    let action = parse_toggle_action("options_toggle:status_text_display:2").unwrap();
    assert!(apply_toggle(action, &mut m));
    assert_eq!(
        build_view_model(&m).options.hud.status_text_display,
        StatusTextDisplay::Percent
    );
    assert!(!apply_toggle("status_text_display:5", &mut m));
    let snapshot = apply_snapshot(&mut m);
    let mut output = HudOptionsFile::default();
    apply_hud_file_snapshot(&mut output, &snapshot.hud);
    assert_eq!(output.status_text_display, StatusTextDisplay::Percent);
}
