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
            .any(|row| row.action == InputAction::MoveForward && row.binding_text == "Unbound")
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
