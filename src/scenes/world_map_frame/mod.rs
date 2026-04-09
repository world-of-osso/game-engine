use bevy::prelude::*;
use bevy::window::PrimaryWindow;
use game_engine::ui::input::find_frame_at;
use game_engine::ui::plugin::{UiState, sync_registry_to_primary_window};
use game_engine::ui::screens::world_map_frame_component::{
    ACTION_WORLD_MAP_CLOSE, FlightPathSegment, MapPin, MapPinType, WorldMapFrameState, ZoneOverlay,
    world_map_frame_screen,
};
use game_engine::world_map_data::{PinType, WorldMapState};
use ui_toolkit::screen::{Screen, SharedContext};

use crate::game_state::GameState;
use crate::networking::CurrentZone;
use crate::ui_input::walk_up_for_onclick;
use crate::zone_names::zone_id_to_name;

const ZONE_LABEL_X: f32 = 0.06;
const ZONE_LABEL_Y: f32 = 0.06;
const ZONE_LABEL_W: f32 = 0.28;
const ZONE_LABEL_H: f32 = 0.08;

#[derive(Resource, Default)]
pub struct WorldMapFrameOpen(pub bool);

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

pub struct WorldMapFramePlugin;

impl Plugin for WorldMapFramePlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<WorldMapFrameOpen>();
        app.add_systems(OnEnter(GameState::InWorld), build_world_map_frame_ui);
        app.add_systems(OnExit(GameState::InWorld), teardown_world_map_frame_ui);
        app.add_systems(
            Update,
            (
                toggle_world_map_frame,
                sync_world_map_frame_state,
                handle_world_map_frame_input,
            )
                .run_if(in_state(GameState::InWorld)),
        );
    }
}

fn build_world_map_frame_ui(
    mut ui: ResMut<UiState>,
    mut commands: Commands,
    windows: Query<&Window, With<PrimaryWindow>>,
    world_map: Res<WorldMapState>,
    current_zone: Res<CurrentZone>,
    open: Res<WorldMapFrameOpen>,
) {
    sync_registry_to_primary_window(&mut ui.registry, &windows);
    let state = build_state(&world_map, &current_zone, &open);
    let mut shared = SharedContext::new();
    shared.insert(state.clone());
    let mut screen = Screen::new(world_map_frame_screen);
    screen.sync(&shared, &mut ui.registry);
    commands.insert_resource(WorldMapFrameWrap(WorldMapFrameRes { screen, shared }));
    commands.insert_resource(WorldMapFrameModel(state));
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

fn toggle_world_map_frame(
    keys: Res<ButtonInput<KeyCode>>,
    reconnect: Option<Res<crate::networking::ReconnectState>>,
    modal_open: Option<Res<crate::scenes::game_menu::UiModalOpen>>,
    mut open: ResMut<WorldMapFrameOpen>,
) {
    if !crate::networking::gameplay_input_allowed(reconnect) || modal_open.is_some() {
        return;
    }
    let shift_pressed = keys.pressed(KeyCode::ShiftLeft) || keys.pressed(KeyCode::ShiftRight);
    if shift_pressed && keys.just_pressed(KeyCode::KeyM) {
        open.0 = !open.0;
    }
}

fn sync_world_map_frame_state(
    mut ui: ResMut<UiState>,
    mut wrap: Option<ResMut<WorldMapFrameWrap>>,
    mut last_model: Option<ResMut<WorldMapFrameModel>>,
    world_map: Res<WorldMapState>,
    current_zone: Res<CurrentZone>,
    open: Res<WorldMapFrameOpen>,
) {
    let (Some(mut wrap), Some(mut last_model)) = (wrap.take(), last_model.take()) else {
        return;
    };
    let state = build_state(&world_map, &current_zone, &open);
    if last_model.0 == state {
        return;
    }
    last_model.0 = state.clone();
    let res = &mut wrap.0;
    res.shared.insert(state);
    res.screen.sync(&res.shared, &mut ui.registry);
}

fn handle_world_map_frame_input(
    windows: Query<&Window, With<PrimaryWindow>>,
    mouse: Option<Res<ButtonInput<MouseButton>>>,
    reconnect: Option<Res<crate::networking::ReconnectState>>,
    modal_open: Option<Res<crate::scenes::game_menu::UiModalOpen>>,
    ui: Res<UiState>,
    mut open: ResMut<WorldMapFrameOpen>,
) {
    if !open.0 || !crate::networking::gameplay_input_allowed(reconnect) || modal_open.is_some() {
        return;
    }
    let Some(mouse) = mouse else { return };
    if !mouse.just_pressed(MouseButton::Left) {
        return;
    }
    let Ok(window) = windows.single() else { return };
    let Some(cursor) = window.cursor_position() else {
        return;
    };
    let Some(frame_id) = find_frame_at(&ui.registry, cursor.x, cursor.y) else {
        return;
    };
    let Some(action) = walk_up_for_onclick(&ui.registry, frame_id) else {
        return;
    };
    if action == ACTION_WORLD_MAP_CLOSE {
        open.0 = false;
    }
}

fn build_state(
    world_map: &WorldMapState,
    current_zone: &CurrentZone,
    open: &WorldMapFrameOpen,
) -> WorldMapFrameState {
    let zone_id = resolve_zone_id(world_map, current_zone);
    let zone_name = resolve_zone_name(world_map, zone_id);
    let explored = zone_id != 0 && world_map.fog.is_explored(zone_id);
    WorldMapFrameState {
        visible: open.0,
        zone_name: zone_name.clone(),
        player_x: world_map.player.x,
        player_y: world_map.player.y,
        continent_name: resolve_continent_name(world_map),
        zone_overlays: build_zone_overlays(&zone_name, explored),
        fog_overlays: build_fog_overlays(&zone_name, zone_id, explored),
        pins: build_pins(world_map),
        flight_paths: build_flight_paths(world_map),
        hovered_pin: None,
    }
}

fn resolve_zone_id(world_map: &WorldMapState, current_zone: &CurrentZone) -> u32 {
    world_map
        .current_zone
        .as_ref()
        .map(|zone| zone.zone_id)
        .filter(|zone_id| *zone_id != 0)
        .unwrap_or(current_zone.zone_id)
}

fn resolve_zone_name(world_map: &WorldMapState, zone_id: u32) -> String {
    world_map
        .current_zone
        .as_ref()
        .map(|zone| zone.name.clone())
        .filter(|name| !name.is_empty())
        .or_else(|| {
            (!world_map.player.zone_name.is_empty()).then(|| world_map.player.zone_name.clone())
        })
        .unwrap_or_else(|| {
            if zone_id == 0 {
                String::new()
            } else {
                zone_id_to_name(zone_id)
            }
        })
}

fn resolve_continent_name(world_map: &WorldMapState) -> String {
    if world_map.player.continent_name.is_empty() {
        "Azeroth".into()
    } else {
        world_map.player.continent_name.clone()
    }
}

fn build_zone_overlays(zone_name: &str, explored: bool) -> Vec<ZoneOverlay> {
    if !explored || zone_name.is_empty() {
        return Vec::new();
    }
    vec![ZoneOverlay {
        name: zone_name.into(),
        x: ZONE_LABEL_X,
        y: ZONE_LABEL_Y,
        w: ZONE_LABEL_W,
        h: ZONE_LABEL_H,
    }]
}

fn build_fog_overlays(zone_name: &str, zone_id: u32, explored: bool) -> Vec<ZoneOverlay> {
    if explored || zone_id == 0 {
        return Vec::new();
    }
    let label = if zone_name.is_empty() {
        "Unexplored".into()
    } else {
        format!("Unexplored: {zone_name}")
    };
    vec![ZoneOverlay {
        name: label,
        x: 0.0,
        y: 0.0,
        w: 1.0,
        h: 1.0,
    }]
}

fn build_pins(world_map: &WorldMapState) -> Vec<MapPin> {
    world_map
        .current_zone_pins()
        .iter()
        .map(|pin| MapPin {
            pin_type: map_pin_type(pin.pin_type),
            label: pin.label.clone(),
            x: pin.x,
            y: pin.y,
        })
        .collect()
}

fn map_pin_type(pin_type: PinType) -> MapPinType {
    match pin_type {
        PinType::Quest => MapPinType::Quest,
        PinType::FlightPath => MapPinType::FlightPath,
        PinType::PointOfInterest | PinType::Vendor | PinType::Innkeeper => {
            MapPinType::PointOfInterest
        }
    }
}

fn build_flight_paths(world_map: &WorldMapState) -> Vec<FlightPathSegment> {
    world_map
        .discovered_flights()
        .into_iter()
        .map(|segment| FlightPathSegment {
            x1: segment.from_x,
            y1: segment.from_y,
            x2: segment.to_x,
            y2: segment.to_y,
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use game_engine::world_map_data::{
        FlightConnection, FogOfWar, MapPlayerPosition, WorldMapPin, ZoneMapData,
    };

    fn sample_world_map(explored: bool) -> WorldMapState {
        WorldMapState {
            player: MapPlayerPosition {
                zone_id: 12,
                continent_name: "Eastern Kingdoms".into(),
                zone_name: "Elwynn Forest".into(),
                x: 0.42,
                y: 0.63,
                facing: 0.0,
            },
            fog: FogOfWar {
                explored_zones: if explored { vec![12] } else { Vec::new() },
            },
            continents: Vec::new(),
            current_zone: Some(ZoneMapData {
                zone_id: 12,
                name: "Elwynn Forest".into(),
                texture_fdid: 0,
                pins: vec![WorldMapPin {
                    pin_type: PinType::FlightPath,
                    label: "Goldshire".into(),
                    x: 0.2,
                    y: 0.3,
                    icon_fdid: 0,
                }],
                flight_connections: vec![FlightConnection {
                    from_name: "Goldshire".into(),
                    to_name: "Stormwind".into(),
                    from_x: 0.2,
                    from_y: 0.3,
                    to_x: 0.8,
                    to_y: 0.4,
                    discovered: true,
                }],
            }),
            selected_continent_idx: 0,
        }
    }

    #[test]
    fn build_state_adds_fog_for_unexplored_zone() {
        let state = build_state(
            &sample_world_map(false),
            &CurrentZone { zone_id: 12 },
            &WorldMapFrameOpen(true),
        );

        assert!(state.visible);
        assert_eq!(state.zone_name, "Elwynn Forest");
        assert_eq!(state.fog_overlays.len(), 1);
        assert_eq!(state.fog_overlays[0].name, "Unexplored: Elwynn Forest");
        assert!(state.zone_overlays.is_empty());
    }

    #[test]
    fn build_state_uses_live_exploration_to_clear_fog() {
        let state = build_state(
            &sample_world_map(true),
            &CurrentZone { zone_id: 12 },
            &WorldMapFrameOpen(true),
        );

        assert!(state.fog_overlays.is_empty());
        assert_eq!(state.zone_overlays.len(), 1);
        assert_eq!(state.zone_overlays[0].name, "Elwynn Forest");
        assert_eq!(state.pins.len(), 1);
        assert_eq!(state.pins[0].pin_type, MapPinType::FlightPath);
        assert_eq!(state.flight_paths.len(), 1);
    }
}
