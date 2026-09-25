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

fn tiles(art: &FlightMapArt, canvas: &Canvas) -> Vec<FlightMapTile> {
    let tile = art.tile * canvas.scale;
    art.tiles
        .iter()
        .map(|entry| FlightMapTile {
            fdid: entry.fdid,
            rect: (
                canvas.origin.x + entry.col as f32 * tile.x,
                canvas.origin.y + entry.row as f32 * tile.y,
                tile.x,
                tile.y,
            ),
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
