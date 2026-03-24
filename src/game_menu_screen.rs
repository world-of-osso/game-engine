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
    ACTION_OPTIONS_APPLY, ACTION_OPTIONS_BACK, ACTION_OPTIONS_CANCEL, ACTION_OPTIONS_DEFAULTS,
    ACTION_OPTIONS_OKAY, OPTIONS_DRAG_HANDLE, OptionsCategory,
};
use ui_toolkit::screen::{Screen, SharedContext};

use crate::client_options::{self, CameraOptions, ClientOptionsUiState, HudOptions};
use crate::game_menu_options::{
    ApplySnapshot, DragCapture, OverlayModel, SliderField, apply_camera_snapshot,
    apply_hud_snapshot, apply_slider_value, apply_snapshot, apply_sound_snapshot, apply_step,
    apply_toggle, build_view_model, camera_draft, cancel_options, hud_draft,
    parse_category_action, parse_slider_action, parse_step_action, parse_toggle_action,
    reset_category_defaults, slider_bounds, sound_draft,
};
use crate::game_state::GameState;
use crate::sound::SoundSettings;

const DRAG_THRESHOLD: f32 = 4.0;
const OPTIONS_W: f32 = 860.0;
const OPTIONS_H: f32 = 580.0;

#[derive(Resource)]
pub struct UiModalOpen;

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
        if world.resource::<UiState>().registry.get_by_name(GAME_MENU_ROOT.0).is_some() {
            return;
        }
        let model = build_overlay_model(world, self.0);
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

fn build_overlay_model(world: &mut World, game_state: GameState) -> OverlayModel {
    let sound = world.get_resource::<SoundSettings>();
    let camera = world.resource::<CameraOptions>();
    let hud = world.resource::<HudOptions>();
    let ui_state = world.resource::<ClientOptionsUiState>();
    let sound_draft = sound_draft(sound.as_deref());
    let camera_d = camera_draft(&camera);
    let hud_d = hud_draft(&hud);
    OverlayModel {
        logged_in: game_state.is_logged_in(),
        view: GameMenuView::MainMenu,
        category: OptionsCategory::Sound,
        modal_position: ui_state.modal_position,
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
    }
}

fn open_menu_overlay(mut ui: ResMut<UiState>, mut commands: Commands) {
    open_game_menu(&mut ui, &mut commands, GameState::GameMenu);
}

fn close_menu_overlay(mut commands: Commands) {
    close_game_menu(&mut commands);
}

fn handle_overlay_input(
    mut overlay: ResMut<GameMenuOverlay>,
    mut ui: ResMut<UiState>,
    windows: Query<&Window, With<bevy::window::PrimaryWindow>>,
    mouse: Option<Res<ButtonInput<MouseButton>>>,
    keyboard: Option<Res<ButtonInput<KeyCode>>>,
    mut exit: MessageWriter<AppExit>,
    mut commands: Commands,
    state: Res<State<GameState>>,
) {
    handle_escape(&mut overlay, &mut ui, keyboard.as_deref(), &mut commands);
    let (Some(mouse), Ok(window)) = (mouse, windows.single()) else { return };
    let Some(cursor) = window.cursor_position() else { return };
    let cursor = Vec2::new(cursor.x, cursor.y);
    handle_press(&mouse, cursor, &mut overlay, &mut ui.registry);
    handle_drag(&mouse, cursor, &mut overlay, &mut ui.registry);
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
    ui: &mut UiState,
    keyboard: Option<&ButtonInput<KeyCode>>,
    commands: &mut Commands,
) {
    let Some(kb) = keyboard else { return };
    if !kb.just_pressed(KeyCode::Escape) {
        return;
    }
    if overlay.model.view == GameMenuView::Options {
        overlay.model.view = GameMenuView::MainMenu;
        overlay.model.drag_capture = DragCapture::None;
        overlay.model.pressed_action = None;
        sync_overlay(overlay, ui);
    } else {
        close_game_menu(commands);
    }
}

fn handle_press(
    mouse: &ButtonInput<MouseButton>,
    cursor: Vec2,
    overlay: &mut GameMenuOverlay,
    reg: &mut FrameRegistry,
) {
    if !mouse.just_pressed(MouseButton::Left) {
        return;
    }
    let Some(frame_id) = find_frame_at(reg, cursor.x, cursor.y) else { return };
    let action = walk_up_for_onclick(reg, frame_id);
    overlay.model.drag_origin = cursor;
    overlay.model.pressed_origin = cursor;
    overlay.model.pressed_action = action.clone();
    if let Some(slider) = action.as_deref().and_then(parse_slider_action) {
        begin_slider_drag(slider, cursor, overlay);
    } else if is_descendant_named(reg, frame_id, OPTIONS_DRAG_HANDLE.0) {
        begin_window_drag(cursor, overlay);
    } else {
        let _ = reg.click_frame(frame_id);
    }
}

fn begin_slider_drag(slider: SliderField, cursor: Vec2, overlay: &mut GameMenuOverlay) {
    overlay.model.drag_capture = DragCapture::Slider(slider);
    update_slider(slider, cursor, overlay);
}

fn begin_window_drag(cursor: Vec2, overlay: &mut GameMenuOverlay) {
    if overlay.model.view != GameMenuView::Options {
        return;
    }
    overlay.model.drag_capture = DragCapture::Window;
    overlay.model.drag_offset =
        cursor - Vec2::new(overlay.model.modal_position[0], overlay.model.modal_position[1]);
}

fn handle_drag(
    mouse: &ButtonInput<MouseButton>,
    cursor: Vec2,
    overlay: &mut GameMenuOverlay,
    reg: &mut FrameRegistry,
) {
    if !mouse.pressed(MouseButton::Left) {
        return;
    }
    match overlay.model.drag_capture {
        DragCapture::Window => drag_window(cursor, overlay, reg),
        DragCapture::Slider(slider) => update_slider(slider, cursor, overlay),
        DragCapture::None => return,
    }
    sync_overlay_model_only(overlay, reg);
}

fn drag_window(cursor: Vec2, overlay: &mut GameMenuOverlay, reg: &FrameRegistry) {
    let pos = cursor - overlay.model.drag_offset;
    overlay.model.modal_position = clamp_modal_position(pos, reg);
}

fn clamp_modal_position(pos: Vec2, reg: &FrameRegistry) -> [f32; 2] {
    let x = pos.x.clamp(0.0, (reg.screen_width - OPTIONS_W).max(0.0));
    let y = pos.y.clamp(0.0, (reg.screen_height - OPTIONS_H).max(0.0));
    [x, y]
}

fn update_slider(slider: SliderField, cursor: Vec2, overlay: &mut GameMenuOverlay) {
    let (min, max) = slider_bounds(slider);
    let track_x = 220.0 + 15.0 + 230.0;
    let row = slider_row(slider);
    let local_x = cursor.x - overlay.model.modal_position[0] - track_x;
    let pct = (local_x / 240.0).clamp(0.0, 1.0);
    let raw = min + (max - min) * pct;
    apply_slider_value(slider, raw, &mut overlay.model);
    overlay.model.drag_origin.y = row;
}

fn slider_row(slider: SliderField) -> f32 {
    match slider {
        SliderField::MasterVolume => 2.0,
        SliderField::MusicVolume => 3.0,
        SliderField::AmbientVolume => 4.0,
        SliderField::FootstepVolume => 5.0,
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
    let Some(action) = overlay.model.pressed_action.take() else { return };
    let Some(frame_id) = find_frame_at(reg, cursor.x, cursor.y) else { return };
    if walk_up_for_onclick(reg, frame_id).as_deref() == Some(action.as_str()) {
        dispatch_overlay_action(action.as_str(), overlay, exit, commands, state);
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
        let Some(frame) = reg.get(id) else { return false };
        if frame.name.as_deref() == Some(target) {
            return true;
        }
        let Some(parent) = frame.parent_id else { return false };
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
    if let Some((key, delta)) = parse_step_action(action) {
        apply_step(key, delta, &mut overlay.model);
        return;
    }
    if let Some(key) = parse_toggle_action(action) {
        apply_toggle(key, &mut overlay.model);
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
        ACTION_OPTIONS_BACK => overlay.model.view = GameMenuView::MainMenu,
        ACTION_OPTIONS_DEFAULTS => reset_category_defaults(&mut overlay.model),
        ACTION_OPTIONS_CANCEL => cancel_options(&mut overlay.model),
        ACTION_OPTIONS_APPLY => commands.queue(ApplyDraftOptionsCommand(apply_snapshot(&mut overlay.model))),
        ACTION_OPTIONS_OKAY => finish_apply_and_close(overlay, commands),
        _ if *state.get() == GameState::GameMenu => warn!("Unknown menu action: {action}"),
        _ => {}
    }
}

fn queue_logout(commands: &mut Commands) {
    close_game_menu(commands);
    commands.queue(SetStateCommand(GameState::Login));
}

fn finish_apply_and_close(overlay: &mut GameMenuOverlay, commands: &mut Commands) {
    let snapshot = apply_snapshot(&mut overlay.model);
    commands.queue(ApplyDraftOptionsCommand(snapshot));
    close_game_menu(commands);
}

struct ApplyDraftOptionsCommand(ApplySnapshot);

impl Command for ApplyDraftOptionsCommand {
    fn apply(self, world: &mut World) {
        apply_snapshot_to_world(world, &self.0);
        save_snapshot(world, self.0.modal_position);
    }
}

fn apply_snapshot_to_world(world: &mut World, snapshot: &ApplySnapshot) {
    if let Some(mut sound) = world.get_resource_mut::<SoundSettings>() {
        apply_sound_snapshot(&mut sound, &snapshot.sound);
    }
    apply_camera_snapshot(&mut world.resource_mut::<CameraOptions>(), &snapshot.camera);
    apply_hud_snapshot(&mut world.resource_mut::<HudOptions>(), &snapshot.hud);
    if let Some(mut fps) = world.get_resource_mut::<FpsOverlayConfig>() {
        fps.enabled = snapshot.hud.show_fps_overlay;
    }
    world.resource_mut::<ClientOptionsUiState>().modal_position = snapshot.modal_position;
    apply_ui_hud_visibility(world, &snapshot.hud);
    apply_target_marker_visibility(world, snapshot.hud.show_target_marker);
}

fn save_snapshot(world: &mut World, modal_position: [f32; 2]) {
    let sound = world.get_resource::<SoundSettings>();
    let camera = world.resource::<CameraOptions>();
    let hud = world.resource::<HudOptions>();
    if let Err(err) =
        client_options::save_client_options(sound.as_deref(), &camera, &hud, modal_position)
    {
        warn!("{err}");
    }
}

fn apply_ui_hud_visibility(world: &mut World, hud: &crate::game_menu_options::HudDraft) {
    let Some(mut ui) = world.get_resource_mut::<UiState>() else {
        return;
    };
    set_named_frames_visible(
        &mut ui.registry,
        &[
            "MinimapCluster",
            "MinimapHeader",
            "MinimapDisplay",
            "MinimapBorder",
            "MinimapArrow",
            "MinimapZoneName",
            "MinimapCoords",
        ],
        hud.show_minimap,
    );
    set_named_frames_visible(
        &mut ui.registry,
        &[
            "MainActionBar",
            "MultiBarRight",
            "MultiBarLeft",
            "MainActionBarMoverLabel",
            "MultiBarRightMoverLabel",
            "MultiBarLeftMoverLabel",
        ],
        hud.show_action_bars,
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
        let Some(id) = reg.get_by_name(name) else { continue };
        let Some(frame) = reg.get_mut(id) else { continue };
        frame.hidden = !visible;
        frame.visible = visible;
        frame.effective_alpha = if visible { frame.alpha } else { 0.0 };
    }
}

struct SaveModalPositionCommand([f32; 2]);

impl Command for SaveModalPositionCommand {
    fn apply(self, world: &mut World) {
        world.resource_mut::<ClientOptionsUiState>().modal_position = self.0;
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

fn sync_overlay(overlay: &mut GameMenuOverlay, ui: &mut UiState) {
    sync_overlay_model_only(overlay, &mut ui.registry);
}

fn sync_overlay_model_only(overlay: &mut GameMenuOverlay, reg: &mut FrameRegistry) {
    overlay.wrap.shared.insert(build_view_model(&overlay.model));
    overlay.wrap.screen.sync(&overlay.wrap.shared, reg);
}
