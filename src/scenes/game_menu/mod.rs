use bevy::dev_tools::fps_overlay::FpsOverlayConfig;
use bevy::prelude::*;
use game_engine::ui::plugin::UiState;
use game_engine::ui::registry::FrameRegistry;
use game_engine::ui::screens::game_menu_component::{
    GAME_MENU_ROOT, GameMenuView, game_menu_screen,
};
use game_engine::ui::screens::options_menu_component::OptionsCategory;
use ui_toolkit::screen::{Screen, SharedContext};

use crate::client_options::{
    self, CameraOptions, ClientOptionsUiState, GraphicsOptions, HudOptions,
};
use crate::game_state::GameState;
use crate::scenes::game_menu::interaction::handle_overlay_input;
use crate::scenes::game_menu::options::{
    ApplySnapshot, BindingCapture, DragCapture, OverlayModel, apply_camera_snapshot,
    apply_graphics_snapshot, apply_hud_snapshot, apply_snapshot, apply_sound_snapshot,
    build_view_model, camera_draft, graphics_draft, hud_draft, sound_draft,
};
use crate::sound::SoundSettings;
use game_engine::input_bindings::{BindingSection, InputBindings};

mod escape_stack;
mod interaction;
pub mod options;

use self::escape_stack::{
    InWorldEscapePanelMut, InWorldEscapeStack, close_topmost_tracked_panel, close_tracked_panel,
    sync_inworld_escape_stack,
};

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
        app.init_resource::<InWorldEscapeStack>();
        app.add_systems(OnEnter(GameState::GameMenu), open_menu_overlay);
        app.add_systems(OnExit(GameState::GameMenu), close_menu_overlay);
        app.add_systems(
            Update,
            sync_inworld_escape_stack.run_if(in_state(GameState::InWorld)),
        );
        app.add_systems(
            Update,
            open_inworld_menu_on_escape
                .after(sync_inworld_escape_stack)
                .run_if(in_state(GameState::InWorld)),
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
    let (graphics_d, sound_draft, camera_d, hud_d, bindings) = overlay_runtime_drafts(world);
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
        draft_graphics: graphics_d.clone(),
        draft_sound: sound_draft.clone(),
        draft_camera: camera_d.clone(),
        draft_hud: hud_d.clone(),
        committed_graphics: graphics_d,
        committed_sound: sound_draft,
        committed_camera: camera_d,
        committed_hud: hud_d,
        draft_bindings: bindings.clone(),
        committed_bindings: bindings,
        binding_section: BindingSection::Movement,
        binding_capture: BindingCapture::None,
    }
}

fn overlay_runtime_drafts(
    world: &mut World,
) -> (
    options::GraphicsDraft,
    options::SoundDraft,
    options::CameraDraft,
    options::HudDraft,
    InputBindings,
) {
    let sound = world.get_resource::<SoundSettings>();
    let camera = world.resource::<CameraOptions>();
    let graphics = world.get_resource::<GraphicsOptions>();
    let hud = world.resource::<HudOptions>();
    let bindings = world
        .get_resource::<InputBindings>()
        .cloned()
        .unwrap_or_default();
    (
        graphics
            .map(graphics_draft)
            .unwrap_or_else(|| graphics_draft(&GraphicsOptions::default())),
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
    mut escape_stack: ResMut<InWorldEscapeStack>,
    mut panels: InWorldEscapePanelMut,
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
    if close_topmost_tracked_panel(&mut escape_stack, |panel| {
        close_tracked_panel(panel, &mut panels)
    })
    .is_some()
    {
        return;
    }
    open_game_menu(&mut ui, &mut commands, GameState::InWorld);
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

fn queue_logout(commands: &mut Commands) {
    commands.queue(crate::logout::RequestLogoutCommand);
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
    apply_graphics_snapshot(
        &mut world.resource_mut::<GraphicsOptions>(),
        &snapshot.graphics,
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

fn save_snapshot(world: &mut World, snapshot: &ApplySnapshot) {
    let camera = snapshot_camera_options(snapshot);
    let mut graphics = world.resource::<GraphicsOptions>().clone();
    apply_graphics_snapshot(&mut graphics, &snapshot.graphics);
    let hud = snapshot_hud_options(snapshot);
    let sound = snapshot_sound_settings(snapshot);
    if let Err(err) = client_options::save_client_options_values(
        &sound,
        &camera,
        &graphics,
        &hud,
        &snapshot.bindings,
        snapshot.modal_position,
    ) {
        warn!("{err}");
    }
}

fn snapshot_camera_options(snapshot: &ApplySnapshot) -> CameraOptions {
    CameraOptions {
        look_sensitivity: snapshot.camera.look_sensitivity,
        invert_y: snapshot.camera.invert_y,
        follow_speed: snapshot.camera.follow_speed,
        zoom_speed: snapshot.camera.zoom_speed,
        min_distance: snapshot.camera.min_distance,
        max_distance: snapshot.camera.max_distance,
    }
}

fn snapshot_hud_options(snapshot: &ApplySnapshot) -> HudOptions {
    HudOptions {
        show_minimap: snapshot.hud.show_minimap,
        show_action_bars: snapshot.hud.show_action_bars,
        show_nameplates: snapshot.hud.show_nameplates,
        show_health_bars: snapshot.hud.show_health_bars,
        show_target_marker: snapshot.hud.show_target_marker,
        show_fps_overlay: snapshot.hud.show_fps_overlay,
    }
}

fn snapshot_sound_settings(snapshot: &ApplySnapshot) -> SoundSettings {
    SoundSettings {
        master_volume: snapshot.sound.master_volume,
        ambient_volume: snapshot.sound.ambient_volume,
        effects_volume: snapshot.sound.effects_volume,
        music_volume: snapshot.sound.music_volume,
        music_enabled: snapshot.sound.music_enabled,
        muted: snapshot.sound.muted,
    }
}

fn apply_ui_hud_visibility(world: &mut World, hud: &options::HudDraft) {
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
    hud: &options::HudDraft,
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
#[path = "../../../tests/unit/game_menu_screen_tests.rs"]
mod tests;
