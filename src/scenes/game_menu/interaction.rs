use bevy::app::AppExit;
use bevy::input::ButtonState;
use bevy::input::keyboard::KeyboardInput;
use bevy::prelude::*;
use game_engine::input_bindings::{InputAction, InputBinding};
use game_engine::ui::input::find_frame_at;
use game_engine::ui::plugin::UiState;
use game_engine::ui::registry::FrameRegistry;
use game_engine::ui::screens::game_menu_component::{
    ACTION_ADDONS, ACTION_EXIT, ACTION_LOGOUT, ACTION_OPTIONS, ACTION_RESUME, ACTION_SUPPORT,
    GameMenuView,
};
use game_engine::ui::screens::options_menu_component::{
    ACTION_OPTIONS_DEFAULTS, ACTION_OPTIONS_OKAY, OPTIONS_DRAG_HANDLE,
};

use super::options::{
    BindingCapture, DragCapture, SliderField, apply_slider_value, apply_step, apply_toggle,
    current_capture_action, parse_binding_clear_action, parse_binding_rebind_action,
    parse_binding_section_action, parse_category_action, parse_slider_action, parse_step_action,
    parse_toggle_action, reset_category_defaults, slider_bounds,
};
use super::{
    DRAG_THRESHOLD, GameMenuOverlay, GameState, SaveModalPositionCommand, close_game_menu,
    modal_top_left, queue_apply_current_options, queue_logout, sync_overlay_model_only,
};

pub(super) fn handle_overlay_input(
    mut overlay: ResMut<GameMenuOverlay>,
    mut ui: ResMut<UiState>,
    windows: Query<&Window, With<bevy::window::PrimaryWindow>>,
    mouse: Option<Res<ButtonInput<MouseButton>>>,
    keyboard: Option<Res<ButtonInput<KeyCode>>>,
    mut key_events: Option<MessageReader<KeyboardInput>>,
    mut exit: MessageWriter<AppExit>,
    mut commands: Commands,
    state: Res<State<GameState>>,
) {
    let capture_changed =
        handle_binding_capture_keys(&mut overlay, key_events.as_mut(), &mut commands);
    if capture_changed {
        sync_overlay_model_only(&mut overlay, &mut ui.registry);
    }
    handle_escape(&mut overlay, keyboard.as_deref(), &mut commands);
    let (Some(mouse), Ok(window)) = (mouse, windows.single()) else {
        return;
    };
    let Some(cursor) = window.cursor_position() else {
        return;
    };
    let cursor = Vec2::new(cursor.x, cursor.y);
    handle_press(
        &mouse,
        cursor,
        &mut overlay,
        &mut ui.registry,
        &mut commands,
    );
    handle_drag(
        &mouse,
        cursor,
        &mut overlay,
        &mut ui.registry,
        &mut commands,
    );
    handle_release(
        &mouse,
        cursor,
        &mut overlay,
        &mut ui.registry,
        &mut exit,
        &mut commands,
        &state,
    );
}

fn handle_escape(
    overlay: &mut GameMenuOverlay,
    keyboard: Option<&ButtonInput<KeyCode>>,
    commands: &mut Commands,
) {
    let Some(kb) = keyboard else { return };
    if !kb.just_pressed(KeyCode::Escape) {
        return;
    }
    if current_capture_action(overlay.model.binding_capture).is_some() {
        overlay.model.binding_capture = BindingCapture::None;
        overlay.model.pressed_action = None;
        return;
    }
    if overlay.model.view == GameMenuView::Options {
        overlay.model.drag_capture = DragCapture::None;
        overlay.model.pressed_action = None;
        close_game_menu(commands);
    } else {
        close_game_menu(commands);
    }
}

fn handle_press(
    mouse: &ButtonInput<MouseButton>,
    cursor: Vec2,
    overlay: &mut GameMenuOverlay,
    reg: &mut FrameRegistry,
    commands: &mut Commands,
) {
    if capture_mouse_binding(mouse, overlay, commands) {
        sync_overlay_model_only(overlay, reg);
        return;
    }
    if !mouse.just_pressed(MouseButton::Left) {
        return;
    }
    let Some(frame_id) = find_frame_at(reg, cursor.x, cursor.y) else {
        return;
    };
    let action = walk_up_for_onclick(reg, frame_id);
    overlay.model.drag_origin = cursor;
    overlay.model.pressed_origin = cursor;
    overlay.model.pressed_action = action.clone();
    if let Some(slider) = action.as_deref().and_then(parse_slider_action) {
        begin_slider_drag(slider, cursor, overlay, reg, commands);
    } else if is_descendant_named(reg, frame_id, OPTIONS_DRAG_HANDLE.0) {
        begin_window_drag(cursor, overlay, reg);
    } else {
        let _ = reg.click_frame(frame_id);
    }
}

fn handle_binding_capture_keys(
    overlay: &mut GameMenuOverlay,
    key_events: Option<&mut MessageReader<KeyboardInput>>,
    commands: &mut Commands,
) -> bool {
    let Some(action) = listening_binding_action(overlay) else {
        return false;
    };
    let Some(key_events) = key_events else {
        return false;
    };
    for event in key_events.read() {
        if event.state != ButtonState::Pressed {
            continue;
        }
        if event.key_code == KeyCode::Escape {
            overlay.model.binding_capture = BindingCapture::None;
            return true;
        }
        if event.key_code
            != KeyCode::Unidentified(bevy::input::keyboard::NativeKeyCode::Unidentified)
        {
            assign_binding(
                action,
                InputBinding::Keyboard(event.key_code),
                overlay,
                commands,
            );
            return true;
        }
    }
    false
}

fn listening_binding_action(overlay: &GameMenuOverlay) -> Option<InputAction> {
    match overlay.model.binding_capture {
        BindingCapture::Listening(action) => Some(action),
        _ => None,
    }
}

fn capture_mouse_binding(
    mouse: &ButtonInput<MouseButton>,
    overlay: &mut GameMenuOverlay,
    commands: &mut Commands,
) -> bool {
    let Some(action) = listening_binding_action(overlay) else {
        return false;
    };
    for button in [
        MouseButton::Left,
        MouseButton::Right,
        MouseButton::Middle,
        MouseButton::Back,
        MouseButton::Forward,
    ] {
        if mouse.just_pressed(button) {
            assign_binding(action, InputBinding::Mouse(button), overlay, commands);
            return true;
        }
    }
    false
}

fn assign_binding(
    action: InputAction,
    binding: InputBinding,
    overlay: &mut GameMenuOverlay,
    commands: &mut Commands,
) {
    overlay.model.draft_bindings.assign(action, binding);
    overlay.model.binding_capture = BindingCapture::None;
    queue_apply_current_options(overlay, commands);
}

fn begin_slider_drag(
    slider: SliderField,
    cursor: Vec2,
    overlay: &mut GameMenuOverlay,
    reg: &FrameRegistry,
    commands: &mut Commands,
) {
    overlay.model.drag_capture = DragCapture::Slider(slider);
    update_slider(slider, cursor, overlay, reg, commands);
}

fn begin_window_drag(cursor: Vec2, overlay: &mut GameMenuOverlay, reg: &FrameRegistry) {
    if overlay.model.view != GameMenuView::Options {
        return;
    }
    overlay.model.drag_capture = DragCapture::Window;
    overlay.model.drag_offset = cursor - modal_top_left(overlay.model.modal_position, reg);
}

fn handle_drag(
    mouse: &ButtonInput<MouseButton>,
    cursor: Vec2,
    overlay: &mut GameMenuOverlay,
    reg: &mut FrameRegistry,
    commands: &mut Commands,
) {
    if !mouse.pressed(MouseButton::Left) {
        return;
    }
    match overlay.model.drag_capture {
        DragCapture::Window => drag_window(cursor, overlay, reg),
        DragCapture::Slider(slider) => update_slider(slider, cursor, overlay, reg, commands),
        DragCapture::None => return,
    }
    sync_overlay_model_only(overlay, reg);
}

fn drag_window(cursor: Vec2, overlay: &mut GameMenuOverlay, reg: &FrameRegistry) {
    let pos = cursor - overlay.model.drag_offset;
    overlay.model.modal_position = super::clamp_top_left(pos, reg);
}

fn update_slider(
    slider: SliderField,
    cursor: Vec2,
    overlay: &mut GameMenuOverlay,
    reg: &FrameRegistry,
    commands: &mut Commands,
) {
    let (min, max) = slider_bounds(slider);
    let pct = slider_rect(slider, reg)
        .map(|rect| ((cursor.x - rect.x) / rect.width.max(f32::EPSILON)).clamp(0.0, 1.0))
        .unwrap_or(0.0);
    let raw = min + (max - min) * pct;
    apply_slider_value(slider, raw, &mut overlay.model);
    queue_apply_current_options(overlay, commands);
    overlay.model.drag_origin.y = slider_row(slider);
}

fn slider_rect(slider: SliderField, reg: &FrameRegistry) -> Option<ui_toolkit::layout::LayoutRect> {
    let frame_id = reg.get_by_name(slider_widget_name(slider))?;
    reg.get(frame_id)?.layout_rect.clone()
}

fn slider_widget_name(slider: SliderField) -> &'static str {
    match slider {
        SliderField::ParticleDensity => "Sliderparticle_density",
        SliderField::RenderScale => "Sliderrender_scale",
        SliderField::BloomIntensity => "Sliderbloom_intensity",
        SliderField::MasterVolume => "Slidermaster_volume",
        SliderField::MusicVolume => "Slidermusic_volume",
        SliderField::AmbientVolume => "Sliderambient_volume",
        SliderField::EffectsVolume => "Slidereffects_volume",
        SliderField::LookSensitivity => "Sliderlook_sensitivity",
        SliderField::ZoomSpeed => "Sliderzoom_speed",
        SliderField::FollowSpeed => "Sliderfollow_speed",
        SliderField::MinDistance => "Slidermin_distance",
        SliderField::MaxDistance => "Slidermax_distance",
    }
}

fn slider_row(slider: SliderField) -> f32 {
    match slider {
        SliderField::ParticleDensity => 3.0,
        SliderField::RenderScale => 1.0,
        SliderField::BloomIntensity => 2.0,
        SliderField::MasterVolume => 2.0,
        SliderField::MusicVolume => 3.0,
        SliderField::AmbientVolume => 4.0,
        SliderField::EffectsVolume => 5.0,
        SliderField::LookSensitivity => 1.0,
        SliderField::ZoomSpeed => 2.0,
        SliderField::FollowSpeed => 3.0,
        SliderField::MinDistance => 4.0,
        SliderField::MaxDistance => 5.0,
    }
}

fn handle_release(
    mouse: &ButtonInput<MouseButton>,
    cursor: Vec2,
    overlay: &mut GameMenuOverlay,
    reg: &mut FrameRegistry,
    exit: &mut MessageWriter<AppExit>,
    commands: &mut Commands,
    state: &State<GameState>,
) {
    if !mouse.just_released(MouseButton::Left) {
        return;
    }
    finish_drag(overlay, commands);
    if moved_far(cursor, overlay.model.pressed_origin) {
        overlay.model.pressed_action = None;
        return;
    }
    let Some(action) = overlay.model.pressed_action.take() else {
        return;
    };
    let Some(frame_id) = find_frame_at(reg, cursor.x, cursor.y) else {
        return;
    };
    if walk_up_for_onclick(reg, frame_id).as_deref() == Some(action.as_str()) {
        dispatch_overlay_action(action.as_str(), overlay, exit, commands, state);
        if let BindingCapture::Armed(action) = overlay.model.binding_capture {
            overlay.model.binding_capture = BindingCapture::Listening(action);
        }
        sync_overlay_model_only(overlay, reg);
    }
}

fn finish_drag(overlay: &mut GameMenuOverlay, commands: &mut Commands) {
    if overlay.model.drag_capture == DragCapture::Window {
        commands.queue(SaveModalPositionCommand(overlay.model.modal_position));
    }
    overlay.model.drag_capture = DragCapture::None;
}

fn moved_far(cursor: Vec2, origin: Vec2) -> bool {
    cursor.distance(origin) > DRAG_THRESHOLD
}

fn walk_up_for_onclick(reg: &FrameRegistry, mut id: u64) -> Option<String> {
    loop {
        let frame = reg.get(id)?;
        if let Some(ref onclick) = frame.onclick {
            return Some(onclick.clone());
        }
        id = frame.parent_id?;
    }
}

fn is_descendant_named(reg: &FrameRegistry, mut id: u64, target: &str) -> bool {
    loop {
        let Some(frame) = reg.get(id) else {
            return false;
        };
        if frame.name.as_deref() == Some(target) {
            return true;
        }
        let Some(parent) = frame.parent_id else {
            return false;
        };
        id = parent;
    }
}

fn dispatch_overlay_action(
    action: &str,
    overlay: &mut GameMenuOverlay,
    exit: &mut MessageWriter<AppExit>,
    commands: &mut Commands,
    state: &State<GameState>,
) {
    if handle_overlay_parsed_action(action, overlay, commands) {
        return;
    }
    handle_overlay_command_action(action, overlay, exit, commands, state);
}

fn handle_overlay_parsed_action(
    action: &str,
    overlay: &mut GameMenuOverlay,
    commands: &mut Commands,
) -> bool {
    if let Some(category) = parse_category_action(action) {
        overlay.model.category = category;
        return true;
    }
    if let Some(section) = parse_binding_section_action(action) {
        overlay.model.binding_section = section;
        overlay.model.binding_capture = BindingCapture::None;
        return true;
    }
    if let Some(action) = parse_binding_rebind_action(action) {
        overlay.model.binding_capture = BindingCapture::Armed(action);
        return true;
    }
    if let Some(action) = parse_binding_clear_action(action) {
        overlay.model.draft_bindings.clear(action);
        overlay.model.binding_capture = BindingCapture::None;
        queue_apply_current_options(overlay, commands);
        return true;
    }
    if let Some((key, delta)) = parse_step_action(action) {
        apply_step(key, delta, &mut overlay.model);
        queue_apply_current_options(overlay, commands);
        return true;
    }
    if let Some(key) = parse_toggle_action(action) {
        apply_toggle(key, &mut overlay.model);
        overlay.model.binding_capture = BindingCapture::None;
        queue_apply_current_options(overlay, commands);
        return true;
    }
    false
}

fn handle_overlay_command_action(
    action: &str,
    overlay: &mut GameMenuOverlay,
    exit: &mut MessageWriter<AppExit>,
    commands: &mut Commands,
    state: &State<GameState>,
) {
    match action {
        ACTION_EXIT => {
            exit.write(AppExit::Success);
        }
        ACTION_LOGOUT => queue_logout(commands),
        ACTION_RESUME => close_game_menu(commands),
        ACTION_OPTIONS => overlay.model.view = GameMenuView::Options,
        ACTION_SUPPORT | ACTION_ADDONS => info!("{action}: placeholder"),
        ACTION_OPTIONS_DEFAULTS => {
            reset_category_defaults(&mut overlay.model);
            queue_apply_current_options(overlay, commands);
        }
        ACTION_OPTIONS_OKAY => close_game_menu(commands),
        _ if *state.get() == GameState::GameMenu => warn!("Unknown menu action: {action}"),
        _ => {}
    }
}
