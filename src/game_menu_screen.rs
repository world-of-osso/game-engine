use bevy::input::ButtonState;
use bevy::input::keyboard::KeyboardInput;
use bevy::dev_tools::fps_overlay::FpsOverlayConfig;
use bevy::prelude::*;
use game_engine::ui::input::find_frame_at;
use game_engine::ui::plugin::UiState;
use game_engine::ui::registry::FrameRegistry;
use game_engine::ui::screens::game_menu_component::{
    ACTION_ADDONS, ACTION_EXIT, ACTION_LOGOUT, ACTION_OPTIONS, ACTION_RESUME, ACTION_SUPPORT,
    GAME_MENU_ROOT, GameMenuView, game_menu_screen,
};
use game_engine::ui::screens::options_menu_component::{
    ACTION_OPTIONS_DEFAULTS, ACTION_OPTIONS_OKAY, OPTIONS_DRAG_HANDLE, OptionsCategory,
};
use ui_toolkit::screen::{Screen, SharedContext};

use crate::client_options::{self, CameraOptions, ClientOptionsUiState, HudOptions};
use crate::game_menu_options::{
    ApplySnapshot, BindingCapture, DragCapture, OverlayModel, SliderField, apply_camera_snapshot,
    apply_hud_snapshot, apply_slider_value, apply_snapshot, apply_sound_snapshot, apply_step,
    apply_toggle, build_view_model, camera_draft, current_capture_action, hud_draft,
    parse_binding_clear_action, parse_binding_rebind_action, parse_binding_section_action,
    parse_category_action, parse_slider_action, parse_step_action, parse_toggle_action,
    reset_category_defaults, slider_bounds, sound_draft,
};
use crate::game_state::GameState;
use game_engine::input_bindings::{BindingSection, InputBinding, InputBindings};
use crate::sound::SoundSettings;

const DRAG_THRESHOLD: f32 = 4.0;
const OPTIONS_W: f32 = 860.0;
const OPTIONS_H: f32 = 580.0;

#[derive(Resource)]
pub struct UiModalOpen;

#[derive(Resource)]
pub struct StartupGameMenuView(pub GameMenuView);

struct GameMenuScreenRes {
    screen: Screen,
    shared: SharedContext,
}

unsafe impl Send for GameMenuScreenRes {}
unsafe impl Sync for GameMenuScreenRes {}

#[derive(Resource)]
pub struct GameMenuOverlay {
    wrap: GameMenuScreenRes,
    model: OverlayModel,
}

pub struct GameMenuScreenPlugin;

impl Plugin for GameMenuScreenPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(OnEnter(GameState::GameMenu), open_menu_overlay);
        app.add_systems(OnExit(GameState::GameMenu), close_menu_overlay);
        app.add_systems(
            Update,
            open_inworld_menu_on_escape.run_if(in_state(GameState::InWorld)),
        );
        app.add_systems(
            Update,
            handle_overlay_input.run_if(resource_exists::<GameMenuOverlay>),
        );
    }
}

pub fn open_game_menu(ui: &mut UiState, commands: &mut Commands, game_state: GameState) {
    if ui.registry.get_by_name(GAME_MENU_ROOT.0).is_none() {
        commands.queue(OpenMenuCommand(game_state));
    }
}

pub fn close_game_menu(commands: &mut Commands) {
    commands.queue(CloseMenuCommand);
}

struct OpenMenuCommand(GameState);

impl Command for OpenMenuCommand {
    fn apply(self, world: &mut World) {
        if world
            .resource::<UiState>()
            .registry
            .get_by_name(GAME_MENU_ROOT.0)
            .is_some()
        {
            return;
        }
        let startup_view = world
            .remove_resource::<StartupGameMenuView>()
            .map(|res| res.0);
        let model = build_overlay_model(world, self.0, startup_view);
        let mut shared = SharedContext::new();
        shared.insert(build_view_model(&model));
        let mut screen = Screen::new(game_menu_screen);
        let mut ui = world.resource_mut::<UiState>();
        screen.sync(&shared, &mut ui.registry);
        world.insert_resource(UiModalOpen);
        world.insert_resource(GameMenuOverlay {
            wrap: GameMenuScreenRes { screen, shared },
            model,
        });
    }
}

struct CloseMenuCommand;

impl Command for CloseMenuCommand {
    fn apply(self, world: &mut World) {
        let Some(mut overlay) = world.remove_resource::<GameMenuOverlay>() else {
            return;
        };
        let mut ui = world.resource_mut::<UiState>();
        overlay.wrap.screen.teardown(&mut ui.registry);
        world.remove_resource::<UiModalOpen>();
    }
}

fn build_overlay_model(
    world: &mut World,
    game_state: GameState,
    startup_view: Option<GameMenuView>,
) -> OverlayModel {
    let (sound_draft, camera_d, hud_d, bindings) = overlay_runtime_drafts(world);
    let ui_state = world.resource::<ClientOptionsUiState>();
    let registry = &world.resource::<UiState>().registry;
    let view = startup_view.unwrap_or(GameMenuView::MainMenu);
    let modal_position = if view == GameMenuView::Options {
        [0.0, 0.0]
    } else {
        initial_modal_offset(ui_state, registry)
    };
    OverlayModel {
        logged_in: game_state.is_logged_in(),
        view,
        category: OptionsCategory::Sound,
        modal_position,
        drag_capture: DragCapture::None,
        drag_origin: Vec2::ZERO,
        drag_offset: Vec2::ZERO,
        pressed_action: None,
        pressed_origin: Vec2::ZERO,
        draft_sound: sound_draft.clone(),
        draft_camera: camera_d.clone(),
        draft_hud: hud_d.clone(),
        committed_sound: sound_draft,
        committed_camera: camera_d,
        committed_hud: hud_d,
        draft_bindings: bindings.clone(),
        committed_bindings: bindings,
        binding_section: BindingSection::Movement,
        binding_capture: BindingCapture::None,
    }
}

fn overlay_runtime_drafts(world: &mut World) -> (
    crate::game_menu_options::SoundDraft,
    crate::game_menu_options::CameraDraft,
    crate::game_menu_options::HudDraft,
    InputBindings,
) {
    let sound = world.get_resource::<SoundSettings>();
    let camera = world.resource::<CameraOptions>();
    let hud = world.resource::<HudOptions>();
    let bindings = world.get_resource::<InputBindings>().cloned().unwrap_or_default();
    (
        sound_draft(sound.as_deref()),
        camera_draft(&camera),
        hud_draft(&hud),
        bindings,
    )
}

fn initial_modal_offset(ui_state: &ClientOptionsUiState, reg: &FrameRegistry) -> [f32; 2] {
    let offset = ui_state
        .modal_offset
        .or_else(|| {
            ui_state
                .legacy_modal_position
                .map(|[x, y]| top_left_to_modal_offset(Vec2::new(x, y), reg))
        })
        .unwrap_or([0.0, 0.0]);
    clamp_modal_offset(Vec2::new(offset[0], offset[1]), reg)
}

fn open_menu_overlay(mut ui: ResMut<UiState>, mut commands: Commands) {
    open_game_menu(&mut ui, &mut commands, GameState::GameMenu);
}

fn close_menu_overlay(mut commands: Commands) {
    close_game_menu(&mut commands);
}

fn open_inworld_menu_on_escape(
    keys: Res<ButtonInput<KeyCode>>,
    reconnect: Option<Res<crate::networking::ReconnectState>>,
    overlay: Option<Res<GameMenuOverlay>>,
    spellbook_runtime: Option<NonSend<game_engine::ui::spellbook_runtime::SpellbookUiRuntime>>,
    mut ui: ResMut<UiState>,
    mut commands: Commands,
) {
    if !crate::networking::gameplay_input_allowed(reconnect) || overlay.is_some() {
        return;
    }
    if spellbook_runtime
        .as_ref()
        .is_some_and(|runtime| runtime.has_focus())
    {
        return;
    }
    if !keys.just_pressed(KeyCode::Escape) {
        return;
    }
    open_game_menu(&mut ui, &mut commands, GameState::InWorld);
}

fn handle_overlay_input(
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
            assign_binding(action, InputBinding::Keyboard(event.key_code), overlay, commands);
            return true;
        }
    }
    false
}

fn listening_binding_action(
    overlay: &GameMenuOverlay,
) -> Option<game_engine::input_bindings::InputAction> {
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
    action: game_engine::input_bindings::InputAction,
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
    overlay.model.modal_position = clamp_top_left(pos, reg);
}

fn clamp_top_left(pos: Vec2, reg: &FrameRegistry) -> [f32; 2] {
    let x = pos.x.clamp(0.0, (reg.screen_width - OPTIONS_W).max(0.0));
    let y = pos.y.clamp(0.0, (reg.screen_height - OPTIONS_H).max(0.0));
    top_left_to_modal_offset(Vec2::new(x, y), reg)
}

fn clamp_modal_offset(offset: Vec2, reg: &FrameRegistry) -> [f32; 2] {
    clamp_top_left(modal_top_left([offset.x, offset.y], reg), reg)
}

fn modal_top_left(offset: [f32; 2], reg: &FrameRegistry) -> Vec2 {
    Vec2::new(
        reg.screen_width * 0.5 + offset[0] - OPTIONS_W * 0.5,
        reg.screen_height * 0.5 - offset[1] - OPTIONS_H * 0.5,
    )
}

fn top_left_to_modal_offset(pos: Vec2, reg: &FrameRegistry) -> [f32; 2] {
    [
        pos.x - reg.screen_width * 0.5 + OPTIONS_W * 0.5,
        reg.screen_height * 0.5 - pos.y - OPTIONS_H * 0.5,
    ]
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
        SliderField::MasterVolume => "Slidermaster_volume",
        SliderField::MusicVolume => "Slidermusic_volume",
        SliderField::AmbientVolume => "Sliderambient_volume",
        SliderField::LookSensitivity => "Sliderlook_sensitivity",
        SliderField::ZoomSpeed => "Sliderzoom_speed",
        SliderField::FollowSpeed => "Sliderfollow_speed",
        SliderField::MinDistance => "Slidermin_distance",
        SliderField::MaxDistance => "Slidermax_distance",
    }
}

fn slider_row(slider: SliderField) -> f32 {
    match slider {
        SliderField::MasterVolume => 2.0,
        SliderField::MusicVolume => 3.0,
        SliderField::AmbientVolume => 4.0,
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
    if let Some(category) = parse_category_action(action) {
        overlay.model.category = category;
        return;
    }
    if let Some(section) = parse_binding_section_action(action) {
        overlay.model.binding_section = section;
        overlay.model.binding_capture = BindingCapture::None;
        return;
    }
    if let Some(action) = parse_binding_rebind_action(action) {
        overlay.model.binding_capture = BindingCapture::Armed(action);
        return;
    }
    if let Some(action) = parse_binding_clear_action(action) {
        overlay.model.draft_bindings.clear(action);
        overlay.model.binding_capture = BindingCapture::None;
        queue_apply_current_options(overlay, commands);
        return;
    }
    if let Some((key, delta)) = parse_step_action(action) {
        apply_step(key, delta, &mut overlay.model);
        queue_apply_current_options(overlay, commands);
        return;
    }
    if let Some(key) = parse_toggle_action(action) {
        apply_toggle(key, &mut overlay.model);
        overlay.model.binding_capture = BindingCapture::None;
        queue_apply_current_options(overlay, commands);
        return;
    }
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

fn queue_logout(commands: &mut Commands) {
    close_game_menu(commands);
    commands.queue(SetStateCommand(GameState::Login));
}

fn queue_apply_current_options(overlay: &mut GameMenuOverlay, commands: &mut Commands) {
    commands.queue(ApplyDraftOptionsCommand(apply_snapshot(&mut overlay.model)));
}

struct ApplyDraftOptionsCommand(ApplySnapshot);

impl Command for ApplyDraftOptionsCommand {
    fn apply(self, world: &mut World) {
        apply_snapshot_to_world(world, &self.0);
        save_snapshot(world, &self.0);
    }
}

fn apply_snapshot_to_world(world: &mut World, snapshot: &ApplySnapshot) {
    info!(
        "Applying options snapshot: muted={}, music_enabled={}",
        snapshot.sound.muted, snapshot.sound.music_enabled
    );
    if let Some(mut sound) = world.get_resource_mut::<SoundSettings>() {
        apply_sound_snapshot(&mut sound, &snapshot.sound);
        info!(
            "Sound settings after apply: muted={}, music_enabled={}",
            sound.muted, sound.music_enabled
        );
    }
    apply_camera_snapshot(&mut world.resource_mut::<CameraOptions>(), &snapshot.camera);
    apply_hud_snapshot(&mut world.resource_mut::<HudOptions>(), &snapshot.hud);
    *world.resource_mut::<InputBindings>() = snapshot.bindings.clone();
    if let Some(mut fps) = world.get_resource_mut::<FpsOverlayConfig>() {
        client_options::apply_fps_overlay_visibility(&mut fps, snapshot.hud.show_fps_overlay);
    }
    let mut ui_state = world.resource_mut::<ClientOptionsUiState>();
    ui_state.modal_offset = Some(snapshot.modal_position);
    ui_state.legacy_modal_position = None;
    apply_ui_hud_visibility(world, &snapshot.hud);
    apply_target_marker_visibility(world, snapshot.hud.show_target_marker);
}

fn save_snapshot(_world: &mut World, snapshot: &ApplySnapshot) {
    let camera = CameraOptions {
        look_sensitivity: snapshot.camera.look_sensitivity,
        invert_y: snapshot.camera.invert_y,
        follow_speed: snapshot.camera.follow_speed,
        zoom_speed: snapshot.camera.zoom_speed,
        min_distance: snapshot.camera.min_distance,
        max_distance: snapshot.camera.max_distance,
    };
    let hud = HudOptions {
        show_minimap: snapshot.hud.show_minimap,
        show_action_bars: snapshot.hud.show_action_bars,
        show_nameplates: snapshot.hud.show_nameplates,
        show_health_bars: snapshot.hud.show_health_bars,
        show_target_marker: snapshot.hud.show_target_marker,
        show_fps_overlay: snapshot.hud.show_fps_overlay,
    };
    let sound = SoundSettings {
        master_volume: snapshot.sound.master_volume,
        ambient_volume: snapshot.sound.ambient_volume,
        music_volume: snapshot.sound.music_volume,
        music_enabled: snapshot.sound.music_enabled,
        muted: snapshot.sound.muted,
    };
    if let Err(err) =
        client_options::save_client_options_values(
            &sound,
            &camera,
            &hud,
            &snapshot.bindings,
            snapshot.modal_position,
        )
    {
        warn!("{err}");
    }
}

fn apply_ui_hud_visibility(world: &mut World, hud: &crate::game_menu_options::HudDraft) {
    let current_state = world
        .get_resource::<State<GameState>>()
        .map(|state| *state.get())
        .unwrap_or(GameState::Login);
    let Some(mut ui) = world.get_resource_mut::<UiState>() else {
        return;
    };
    apply_ui_hud_visibility_for_state(&mut ui.registry, current_state, hud);
}

fn apply_ui_hud_visibility_for_state(
    reg: &mut FrameRegistry,
    current_state: GameState,
    hud: &crate::game_menu_options::HudDraft,
) {
    let in_world = current_state == GameState::InWorld;
    set_named_frames_visible(
        reg,
        &[
            "MinimapCluster",
            "MinimapHeader",
            "MinimapDisplay",
            "MinimapBorder",
            "MinimapArrow",
            "MinimapZoneName",
            "MinimapCoords",
        ],
        in_world && hud.show_minimap,
    );
    set_named_frames_visible(
        reg,
        &[
            "MainActionBar",
            "MultiBarBottomLeft",
            "MultiBarBottomRight",
            "MultiBarRight",
            "MultiBarLeft",
            "MainActionBarMoverLabel",
            "MultiBarBottomLeftMoverLabel",
            "MultiBarBottomRightMoverLabel",
            "MultiBarRightMoverLabel",
            "MultiBarLeftMoverLabel",
        ],
        in_world && hud.show_action_bars,
    );
}

fn apply_target_marker_visibility(world: &mut World, visible: bool) {
    let mut query = world.query_filtered::<&mut Visibility, With<crate::target::TargetMarker>>();
    for mut value in query.iter_mut(world) {
        *value = if visible {
            Visibility::Visible
        } else {
            Visibility::Hidden
        };
    }
}

fn set_named_frames_visible(reg: &mut FrameRegistry, names: &[&str], visible: bool) {
    for name in names {
        let Some(id) = reg.get_by_name(name) else {
            continue;
        };
        let Some(frame) = reg.get_mut(id) else {
            continue;
        };
        frame.hidden = !visible;
        frame.visible = visible;
        frame.effective_alpha = if visible { frame.alpha } else { 0.0 };
    }
}

struct SaveModalPositionCommand([f32; 2]);

impl Command for SaveModalPositionCommand {
    fn apply(self, world: &mut World) {
        let mut ui_state = world.resource_mut::<ClientOptionsUiState>();
        ui_state.modal_offset = Some(self.0);
        ui_state.legacy_modal_position = None;
    }
}

struct SetStateCommand(GameState);

impl Command for SetStateCommand {
    fn apply(self, world: &mut World) {
        if let Some(mut next) = world.get_resource_mut::<NextState<GameState>>() {
            next.set(self.0);
        }
    }
}

fn sync_overlay_model_only(overlay: &mut GameMenuOverlay, reg: &mut FrameRegistry) {
    overlay.wrap.shared.insert(build_view_model(&overlay.model));
    overlay.wrap.screen.sync(&overlay.wrap.shared, reg);
}

#[cfg(test)]
#[path = "game_menu_screen_tests.rs"]
mod tests;
