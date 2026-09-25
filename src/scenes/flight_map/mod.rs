//! FlightMapFrame scene (docs/specs/flight-master.md): builds the Retail flight map
//! from [`TaxiMapState`] on the continent art ([`FlightMapArt`]), tracks the hovered
//! pin and turns clicks into [`TaxiRequest`]s.
//!
//! Following FM_FlightPathDataProvider.lua: the current node and reachable nodes are
//! shown, unreachable ones only while they lie on the hovered route (:193); with
//! nothing hovered, a line runs from the current node to the first hop of every
//! reachable node (`ShowBackgroundRoutesFromCurrent`, :131-166); hovering a
//! reachable node turns it yellow and highlights its whole route (:66-100, :261); a
//! left click takes the flight (`TakeTaxiNode`, :233-238).

use std::collections::HashMap;
use std::path::Path;

use bevy::prelude::*;
use bevy::window::PrimaryWindow;
use game_engine::taxi_state::{FlightMapArt, TaxiMapState, TaxiRequest};
use game_engine::ui::input::{find_frame_at, ui_cursor_position};
use game_engine::ui::plugin::{UiState, sync_registry_to_primary_window};
use game_engine::ui::screens::flight_map_component::{
    ACTION_CLOSE, ACTION_NODE_PREFIX, CANVAS_H, CANVAS_W, CANVAS_X, CANVAS_Y, FRAME_H, FRAME_W,
    FlightMapFrameState, FlightMapLine, FlightMapPin, FlightMapTile, FlightMapTooltip, PinLook,
    TooltipDetail, flight_map_screen,
};
use shared::protocol::{TaxiNodeInfo, TaxiNodeState};
use ui_toolkit::screen::{Screen, SharedContext};

use crate::game::inworld_scene_stage::inworld_scene_stage_allows_ui;
use crate::game_state::GameState;
use crate::ui_input::walk_up_for_onclick;

const DB2_DIR: &str = "data/db2/12.1.0.69933";

struct FlightMapRes {
    screen: Screen,
    shared: SharedContext,
}

unsafe impl Send for FlightMapRes {}
unsafe impl Sync for FlightMapRes {}

#[derive(Resource)]
struct FlightMapWrap(FlightMapRes);

#[derive(Resource, Clone, PartialEq)]
struct FlightMapModel(FlightMapFrameState);

/// Continent art by `MapID`, loaded the first time a flight master opens it.
#[derive(Resource, Default)]
struct FlightMapArtCache(HashMap<u32, Option<FlightMapArt>>);

impl FlightMapArtCache {
    fn get(&mut self, continent: u32) -> Option<&FlightMapArt> {
        self.0
            .entry(continent)
            .or_insert_with(|| {
                FlightMapArt::load(Path::new(DB2_DIR), continent)
                    .map_err(|err| error!("flight map art for map {continent}: {err}"))
                    .ok()
            })
            .as_ref()
    }
}

pub struct FlightMapPlugin;

impl Plugin for FlightMapPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<TaxiMapState>()
            .init_resource::<FlightMapArtCache>()
            .add_message::<TaxiRequest>();
        app.add_systems(
            OnEnter(GameState::InWorld),
            build_flight_map_ui.run_if(inworld_scene_stage_allows_ui),
        );
        app.add_systems(OnExit(GameState::InWorld), teardown_flight_map_ui);
        app.add_systems(
            Update,
            (handle_flight_map_input, sync_flight_map_state)
                .chain()
                .run_if(in_state(GameState::InWorld))
                .run_if(inworld_scene_stage_allows_ui),
        );
    }
}

/// Map art placed in the canvas: origin and size in frame space.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct Canvas {
    origin: Vec2,
    size: Vec2,
    scale: f32,
}

impl Canvas {
    /// `ResetZoom`: the whole map fitted and centred in the scroll container.
    pub(crate) fn fit(art: &FlightMapArt) -> Self {
        let scale = (CANVAS_W / art.size.x).min(CANVAS_H / art.size.y);
        let size = art.size * scale;
        let origin = Vec2::new(CANVAS_X, CANVAS_Y) + (Vec2::new(CANVAS_W, CANVAS_H) - size) / 2.0;
        Self {
            origin,
            size,
            scale,
        }
    }

    pub(crate) fn point(&self, art: &FlightMapArt, node: &TaxiNodeInfo) -> Vec2 {
        self.origin + art.map_position(node.world_x, node.world_y) * self.size
    }
}

/// `AdventureMap_TileBg` (5350), atlas 723 (1270720) 512×512, tiled at canvas scale.
const TILE_BACKGROUND: u32 = 1_270_720;
const TILE_BACKGROUND_SIZE: f32 = 512.0;
const FULL_UV: (f32, f32, f32, f32) = (0.0, 1.0, 0.0, 1.0);

/// Whole-pixel edges of cell `index` of `size`, so neighbours meet without seams.
fn snapped(origin: f32, size: f32, index: u32) -> (f32, f32) {
    let start = (origin + index as f32 * size).round();
    let end = (origin + (index + 1) as f32 * size).round();
    (start, end - start)
}

fn tiles(art: &FlightMapArt, canvas: &Canvas) -> Vec<FlightMapTile> {
    let tile = art.tile * canvas.scale;
    art.tiles
        .iter()
        .map(|entry| {
            let (x, width) = snapped(canvas.origin.x, tile.x, entry.col);
            let (y, height) = snapped(canvas.origin.y, tile.y, entry.row);
            FlightMapTile {
                fdid: entry.fdid,
                rect: (x, y, width, height),
                uv: FULL_UV,
            }
        })
        .collect()
}

/// The background texture repeated over the canvas, the last row and column cropped.
fn background(canvas: &Canvas) -> Vec<FlightMapTile> {
    let size = TILE_BACKGROUND_SIZE * canvas.scale;
    let end = canvas.origin + canvas.size;
    let (cols, rows) = (
        (canvas.size.x / size).ceil() as u32,
        (canvas.size.y / size).ceil() as u32,
    );
    (0..rows)
        .flat_map(|row| (0..cols).map(move |col| (row, col)))
        .map(|(row, col)| {
            let (x, width) = snapped(canvas.origin.x, size, col);
            let (y, height) = snapped(canvas.origin.y, size, row);
            let width = width.min(end.x.round() - x);
            let height = height.min(end.y.round() - y);
            FlightMapTile {
                fdid: TILE_BACKGROUND,
                rect: (x, y, width, height),
                uv: (0.0, width / size.round(), 0.0, height / size.round()),
            }
        })
        .collect()
}

fn hovered_route(taxi: &TaxiMapState) -> &[u32] {
    taxi.hovered
        .and_then(|node| taxi.node(node))
        .filter(|node| node.state == TaxiNodeState::Reachable)
        .map_or(&[], |node| node.route.as_slice())
}

fn pin_look(node: &TaxiNodeInfo, hovered: Option<u32>) -> PinLook {
    match node.state {
        TaxiNodeState::Current => PinLook::Current,
        TaxiNodeState::Reachable if hovered == Some(node.node) => PinLook::Hovered,
        TaxiNodeState::Reachable => PinLook::Reachable,
        TaxiNodeState::Unreachable => PinLook::Unreachable,
    }
}

fn lines(taxi: &TaxiMapState, art: &FlightMapArt, canvas: &Canvas) -> Vec<FlightMapLine> {
    let at = |id: u32| taxi.node(id).map(|node| canvas.point(art, node));
    let segment = |from: u32, to: u32, highlight: bool| {
        Some(FlightMapLine {
            from: at(from)?.into(),
            to: at(to)?.into(),
            highlight,
        })
    };
    let route = hovered_route(taxi);
    if !route.is_empty() {
        return route
            .windows(2)
            .filter_map(|hop| segment(hop[0], hop[1], true))
            .collect();
    }
    let mut hops: Vec<(u32, u32)> = taxi
        .nodes
        .iter()
        .filter(|node| node.state == TaxiNodeState::Reachable && node.route.len() >= 2)
        .map(|node| (node.route[0], node.route[1]))
        .collect();
    hops.sort_unstable();
    hops.dedup();
    hops.into_iter()
        .filter_map(|(from, to)| segment(from, to, false))
        .collect()
}

fn tooltip(taxi: &TaxiMapState, art: &FlightMapArt, canvas: &Canvas) -> Option<FlightMapTooltip> {
    let node = taxi.node(taxi.hovered?)?;
    let center = canvas.point(art, node);
    let half = pin_look(node, taxi.hovered).size() / 2.0;
    let detail = match node.state {
        TaxiNodeState::Current => Some(TooltipDetail::YouAreHere),
        TaxiNodeState::Reachable => (node.cost > 0).then_some(TooltipDetail::Cost(node.cost)),
        TaxiNodeState::Unreachable => Some(TooltipDetail::NotDiscovered),
    };
    Some(FlightMapTooltip {
        x: center.x + half,
        y: center.y - half,
        name: node.name.clone(),
        detail,
    })
}

pub(crate) fn build_state(
    taxi: &TaxiMapState,
    art: Option<&FlightMapArt>,
    screen: Vec2,
) -> FlightMapFrameState {
    let left = ((screen.x - FRAME_W) / 2.0).max(0.0);
    let top = ((screen.y - FRAME_H) / 2.0).max(0.0);
    let (Some(art), true) = (art, taxi.is_open()) else {
        return FlightMapFrameState {
            left,
            top,
            ..Default::default()
        };
    };
    let canvas = Canvas::fit(art);
    let route = hovered_route(taxi);
    let pins = taxi
        .nodes
        .iter()
        .filter(|node| node.state != TaxiNodeState::Unreachable || route.contains(&node.node))
        .map(|node| {
            let at = canvas.point(art, node);
            FlightMapPin {
                node: node.node,
                x: at.x,
                y: at.y,
                look: pin_look(node, taxi.hovered),
            }
        })
        .collect();
    FlightMapFrameState {
        visible: true,
        left,
        top,
        background: background(&canvas),
        tiles: tiles(art, &canvas),
        lines: lines(taxi, art, &canvas),
        pins,
        tooltip: tooltip(taxi, art, &canvas),
    }
}

/// The frame state for the open map, with its continent art.
fn current_state(
    taxi: &TaxiMapState,
    cache: &mut FlightMapArtCache,
    screen: Vec2,
) -> FlightMapFrameState {
    let art = taxi
        .is_open()
        .then(|| cache.get(taxi.continent).cloned())
        .flatten();
    build_state(taxi, art.as_ref(), screen)
}

fn build_flight_map_ui(
    mut ui: ResMut<UiState>,
    mut commands: Commands,
    windows: Query<&Window, With<PrimaryWindow>>,
    taxi: Res<TaxiMapState>,
) {
    sync_registry_to_primary_window(&mut ui.registry, &windows);
    let screen = Vec2::new(ui.registry.screen_width, ui.registry.screen_height);
    let state = build_state(&taxi, None, screen);
    let mut shared = SharedContext::new();
    shared.insert(state.clone());
    let mut screen = Screen::new(flight_map_screen);
    screen.sync(&shared, &mut ui.registry);
    commands.insert_resource(FlightMapWrap(FlightMapRes { screen, shared }));
    commands.insert_resource(FlightMapModel(state));
}

fn teardown_flight_map_ui(
    mut ui: ResMut<UiState>,
    mut commands: Commands,
    mut wrap: Option<ResMut<FlightMapWrap>>,
) {
    if let Some(res) = wrap.as_mut() {
        res.0.screen.teardown(&mut ui.registry);
    }
    commands.remove_resource::<FlightMapWrap>();
    commands.remove_resource::<FlightMapModel>();
}

fn sync_flight_map_state(
    mut ui: ResMut<UiState>,
    wrap: Option<ResMut<FlightMapWrap>>,
    last_model: Option<ResMut<FlightMapModel>>,
    taxi: Res<TaxiMapState>,
    mut cache: ResMut<FlightMapArtCache>,
) {
    let (Some(mut wrap), Some(mut last_model)) = (wrap, last_model) else {
        return;
    };
    let screen = Vec2::new(ui.registry.screen_width, ui.registry.screen_height);
    let state = current_state(&taxi, &mut cache, screen);
    if last_model.0 == state {
        return;
    }
    last_model.0 = state.clone();
    let res = &mut wrap.0;
    res.shared.insert(state);
    res.screen.sync(&res.shared, &mut ui.registry);
}

/// The request a frame click sends.
pub(crate) fn request_for_action(action: &str) -> Option<TaxiRequest> {
    if action == ACTION_CLOSE {
        return Some(TaxiRequest::Close);
    }
    let destination = action.strip_prefix(ACTION_NODE_PREFIX)?.parse().ok()?;
    Some(TaxiRequest::Fly { destination })
}

fn node_under_cursor(ui: &UiState, window: &Window) -> Option<(u32, String)> {
    let cursor = ui_cursor_position(&ui.registry, window)?;
    let frame = find_frame_at(&ui.registry, cursor.x, cursor.y)?;
    let action = walk_up_for_onclick(&ui.registry, frame)?;
    let node = action
        .strip_prefix(ACTION_NODE_PREFIX)
        .and_then(|id| id.parse().ok());
    Some((node.unwrap_or(0), action))
}

fn handle_flight_map_input(
    windows: Query<&Window, With<PrimaryWindow>>,
    mouse: Option<Res<ButtonInput<MouseButton>>>,
    ui: Res<UiState>,
    mut taxi: ResMut<TaxiMapState>,
    mut requests: MessageWriter<TaxiRequest>,
) {
    if !taxi.is_open() {
        return;
    }
    let under = windows
        .single()
        .ok()
        .and_then(|window| node_under_cursor(&ui, window));
    let hovered = under
        .as_ref()
        .map(|(node, _)| *node)
        .filter(|node| taxi.node(*node).is_some());
    if taxi.hovered != hovered {
        taxi.hovered = hovered;
    }
    let clicked = mouse.is_some_and(|mouse| mouse.just_pressed(MouseButton::Left));
    if let Some(request) = under
        .filter(|_| clicked)
        .and_then(|(_, action)| request_for_action(&action))
    {
        requests.write(request);
    }
}

#[cfg(test)]
mod tests;
