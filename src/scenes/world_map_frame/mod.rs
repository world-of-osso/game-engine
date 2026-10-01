//! Bevy host for the shared world map (docs/specs/world-map.md): `M` toggles it,
//! breadcrumbs and right-click navigate, and the view model comes from
//! [`game_engine::world_map_view_data`].

use std::path::Path;

use bevy::ecs::system::SystemParam;
use bevy::prelude::*;
use bevy::window::PrimaryWindow;
use game_engine::input_bindings::InputAction;
use game_engine::ui::input::{find_frame_at, ui_cursor_position};
use game_engine::ui::plugin::{UiState, sync_registry_to_primary_window};
use game_engine::ui::screens::world_map_frame_component::{
    ACTION_WORLD_MAP_CLOSE, ACTION_WORLD_MAP_NAV_PREFIX, WorldMapFrameState,
    apply_world_map_postsetup, world_map_frame_screen,
};
use game_engine::world_map_view_data::{
    WorldMapData, WorldMapPlayer, WorldMapRequest, engine_to_world, player_map,
    world_map_frame_state, zoom_out,
};
use ui_toolkit::screen::{Screen, SharedContext};

use crate::game::inworld_scene_stage::inworld_scene_stage_allows_ui;
use crate::game_state::GameState;
use crate::networking::LocalPlayer;
use crate::terrain::AdtManager;
use crate::ui_input::walk_up_for_onclick;
use crate::window_manager::{WindowId, WindowManager};

const DB2_DIR: &str = "data/db2/12.1.0.69933";

struct WorldMapFrameRes {
    screen: Screen,
    shared: SharedContext,
}

unsafe impl Send for WorldMapFrameRes {}
unsafe impl Sync for WorldMapFrameRes {}

#[derive(Resource)]
struct WorldMapFrameWrap(WorldMapFrameRes);

#[derive(Resource, Clone, PartialEq)]
struct WorldMapFrameModel(WorldMapFrameState);

/// Static map data, loaded once; `None` after a load failure (logged).
#[derive(Resource)]
struct WorldMapCatalog(Option<WorldMapData>);

/// The displayed `UiMap`; set to the player's map whenever the frame opens.
#[derive(Resource, Default)]
struct WorldMapNav(u32);

pub struct WorldMapFramePlugin;

impl Plugin for WorldMapFramePlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<WorldMapNav>();
        app.add_systems(
            OnEnter(GameState::InWorld),
            build_world_map_frame_ui.run_if(inworld_scene_stage_allows_ui),
        );
        app.add_systems(OnExit(GameState::InWorld), teardown_world_map_frame_ui);
        app.add_systems(
            Update,
            (
                toggle_world_map_frame,
                handle_world_map_frame_input,
                sync_world_map_frame_state,
            )
                .chain()
                .run_if(in_state(GameState::InWorld))
                .run_if(inworld_scene_stage_allows_ui),
        );
    }
}

fn load_catalog() -> WorldMapCatalog {
    match WorldMapData::load(Path::new(DB2_DIR)) {
        Ok(data) => WorldMapCatalog(Some(data)),
        Err(error) => {
            warn!("World map data unavailable: {error}");
            WorldMapCatalog(None)
        }
    }
}

fn build_world_map_frame_ui(
    mut ui: ResMut<UiState>,
    mut commands: Commands,
    windows: Query<&Window, With<PrimaryWindow>>,
) {
    sync_registry_to_primary_window(&mut ui.registry, &windows);
    let state = WorldMapFrameState {
        viewport: [ui.registry.screen_width, ui.registry.screen_height],
        ..Default::default()
    };
    let mut shared = SharedContext::new();
    shared.insert(state.clone());
    let mut screen = Screen::new(world_map_frame_screen);
    screen.sync(&shared, &mut ui.registry);
    commands.insert_resource(WorldMapFrameWrap(WorldMapFrameRes { screen, shared }));
    commands.insert_resource(WorldMapFrameModel(state));
    commands.insert_resource(load_catalog());
}

fn teardown_world_map_frame_ui(
    mut ui: ResMut<UiState>,
    mut commands: Commands,
    mut wrap: Option<ResMut<WorldMapFrameWrap>>,
) {
    if let Some(res) = wrap.as_mut() {
        res.0.screen.teardown(&mut ui.registry);
    }
    commands.remove_resource::<WorldMapFrameWrap>();
    commands.remove_resource::<WorldMapFrameModel>();
}

type LocalPlayerQuery<'w, 's> = Query<
    'w,
    's,
    (&'static Transform, &'static crate::camera::CharacterFacing),
    (With<crate::camera::Player>, With<LocalPlayer>),
>;

/// Map data, navigation and the local player the view model reads.
#[derive(SystemParam)]
struct MapContext<'w, 's> {
    catalog: Option<Res<'w, WorldMapCatalog>>,
    nav: ResMut<'w, WorldMapNav>,
    player_q: LocalPlayerQuery<'w, 's>,
    adt: Option<Res<'w, AdtManager>>,
}

impl MapContext<'_, '_> {
    fn data(&self) -> Option<&WorldMapData> {
        self.catalog.as_ref().and_then(|catalog| catalog.0.as_ref())
    }

    fn player(&self) -> Option<WorldMapPlayer> {
        let (transform, facing) = self.player_q.single().ok()?;
        let adt = self.adt.as_ref()?;
        Some(WorldMapPlayer {
            map_id: crate::light_lookup::map_name_to_id(&adt.map_name)?,
            position: engine_to_world(transform.translation.to_array()),
            yaw: facing.yaw,
            faction: None,
        })
    }

    fn state(&self, visible: bool, viewport: [f32; 2]) -> WorldMapFrameState {
        let Some(data) = self.data() else {
            return WorldMapFrameState {
                visible,
                viewport,
                ..Default::default()
            };
        };
        let player = self.player();
        world_map_frame_state(
            data,
            WorldMapRequest {
                visible,
                viewport,
                map_id: self.nav.0,
                hovered: None,
                player: player.as_ref(),
                quests: &[],
                quest_areas: &[],
            },
        )
    }
}

fn toggle_world_map_frame(
    keybinds: crate::ui_input_mode::WorldKeybinds,
    mut window_manager: ResMut<WindowManager>,
    mut map: MapContext,
) {
    if !keybinds.just_pressed(InputAction::ToggleWorldMap) {
        return;
    }
    window_manager.toggle(WindowId::WorldMap);
    if !window_manager.is_open(WindowId::WorldMap) {
        return;
    }
    let opened = map
        .data()
        .zip(map.player())
        .and_then(|(data, player)| player_map(data, &player));
    if let Some(opened) = opened {
        map.nav.0 = opened;
    }
}

fn sync_world_map_frame_state(
    mut ui: ResMut<UiState>,
    mut wrap: Option<ResMut<WorldMapFrameWrap>>,
    mut last_model: Option<ResMut<WorldMapFrameModel>>,
    window_manager: Res<WindowManager>,
    map: MapContext,
) {
    let (Some(wrap), Some(last_model)) = (wrap.as_mut(), last_model.as_mut()) else {
        return;
    };
    let viewport = [ui.registry.screen_width, ui.registry.screen_height];
    let state = map.state(window_manager.is_open(WindowId::WorldMap), viewport);
    if last_model.0 == state {
        return;
    }
    last_model.0 = state.clone();
    let res = &mut wrap.0;
    res.shared.insert(state.clone());
    res.screen.sync(&res.shared, &mut ui.registry);
    apply_world_map_postsetup(&state, &mut ui.registry);
}

fn handle_world_map_frame_input(
    windows: Query<&Window, With<PrimaryWindow>>,
    mouse: Option<Res<ButtonInput<MouseButton>>>,
    reconnect: Option<Res<crate::networking::ReconnectState>>,
    modal_open: Option<Res<crate::scenes::game_menu::UiModalOpen>>,
    ui: Res<UiState>,
    mut map: MapContext,
    mut window_manager: ResMut<WindowManager>,
) {
    if !window_manager.is_open(WindowId::WorldMap)
        || !crate::networking::gameplay_input_allowed(reconnect)
        || modal_open.is_some()
    {
        return;
    }
    let Some(mouse) = mouse else { return };
    if mouse.just_pressed(MouseButton::Right) {
        if let Some(parent) = map.data().and_then(|data| zoom_out(data, map.nav.0)) {
            map.nav.0 = parent;
        }
        return;
    }
    if !mouse.just_pressed(MouseButton::Left) {
        return;
    }
    let Ok(window) = windows.single() else { return };
    let Some(cursor) = ui_cursor_position(&ui.registry, window) else {
        return;
    };
    let Some(frame_id) = find_frame_at(&ui.registry, cursor.x, cursor.y) else {
        return;
    };
    let Some(action) = walk_up_for_onclick(&ui.registry, frame_id) else {
        return;
    };
    if action == ACTION_WORLD_MAP_CLOSE {
        window_manager.close(WindowId::WorldMap);
    } else if let Some(target) = action
        .strip_prefix(ACTION_WORLD_MAP_NAV_PREFIX)
        .and_then(|id| id.parse().ok())
    {
        map.nav.0 = target;
    }
}

#[cfg(test)]
mod automation_tests;
