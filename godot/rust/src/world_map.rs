//! In-world World Map (docs/specs/world-map.md) over the shared view model: the
//! `ToggleWorldMap` binding (M) opens the player's zone; right-click or wheel-down
//! on the canvas zooms out, left-click or wheel-up zooms into the map under the
//! cursor, breadcrumbs jump up the hierarchy, and Escape or M closes it. Keyboard
//! movement keeps working while it is open, like the Retail windowed map.

use std::collections::HashMap;

use game_engine_core::input_bindings_data::InputAction;
use game_engine_session::SessionScreen;
use game_engine_ui_model::world_map_frame_component::{
    ACTION_WORLD_MAP_CLOSE, ACTION_WORLD_MAP_NAV_PREFIX, WorldMapFrameState, WorldMapLayout,
    world_map_texture_fdids,
};
use game_engine_ui_model::world_map_view_data::{
    WorldMapData, WorldMapPlayer, WorldMapRequest, engine_to_world, player_map,
    world_map_frame_state, zoom_in, zoom_out,
};
use godot::classes::{InputEvent, InputEventMouseButton, InputEventMouseMotion};
use godot::global::MouseButton;
use godot::prelude::*;

use crate::{GameClient, ui::RegistryUi};

const DB2_DIR: &str = "db2/12.1.0.69933";

/// Map navigation and the open frame.
#[derive(Default)]
pub(crate) struct WorldMap {
    pub(crate) ui: Option<Gd<RegistryUi>>,
    data: Option<Result<WorldMapData, String>>,
    /// Displayed `UiMap`.
    map_id: u32,
    /// Cursor on the canvas, as a map UV.
    hovered: Option<[f32; 2]>,
    /// Whether each texture FDID is on disk (copied out of local CASC if needed).
    available: HashMap<u32, bool>,
    /// Art tiles of the displayed map missing from local CASC.
    missing_tiles: usize,
    /// Saved top-left in logical UI units; None uses Wide slot.
    position: Option<[f32; 2]>,
    drag: Option<WindowDrag>,
}

impl WorldMap {
    pub fn is_open(&self) -> bool {
        self.ui.is_some()
    }

    fn data(&self) -> Option<&WorldMapData> {
        self.data.as_ref()?.as_ref().ok()
    }

    fn reset_position(&mut self) {
        self.position = None;
        self.drag = None;
    }
}

fn placed_map_layout(viewport: [f32; 2], saved: Option<[f32; 2]>) -> WorldMapLayout {
    let mut layout = WorldMapLayout::for_viewport(viewport);
    let [_, _, width, height] = layout.frame_rect();
    let slot = [(viewport[0] - width) * 0.5, 104.0];
    let [x, y] = saved.unwrap_or(slot);
    layout.origin = [
        x.clamp(0.0, (viewport[0] - width).max(0.0)),
        y.clamp(0.0, (viewport[1] - height).max(0.0)),
    ];
    layout
}

pub(crate) fn title_hit(
    rect: [f32; 4],
    physical: Vector2,
    scale: f32,
    buttons: &[[f32; 4]],
) -> bool {
    let point = physical / scale;
    let [x, y, width, _] = rect;
    point.x >= x
        && point.x <= x + width
        && point.y >= y
        && point.y <= y + 24.0
        && !buttons.iter().any(|rect| inside(*rect, point))
}

pub(crate) struct WindowDrag {
    grab: Vector2,
}

impl WindowDrag {
    pub(crate) fn begin(point: Vector2, origin: [f32; 2]) -> Self {
        Self {
            grab: point - Vector2::new(origin[0], origin[1]),
        }
    }

    pub(crate) fn position(&self, point: Vector2, viewport: [f32; 2], size: [f32; 2]) -> [f32; 2] {
        let target = point - self.grab;
        [
            target.x.clamp(0.0, (viewport[0] - size[0]).max(0.0)),
            target.y.clamp(0.0, (viewport[1] - size[1]).max(0.0)),
        ]
    }
}

fn canvas_uv(layout: &WorldMapLayout, point: Vector2) -> Option<[f32; 2]> {
    let [x, y, w, h] = layout.canvas_rect();
    let uv = [(point.x - x) / w, (point.y - y) / h];
    uv.iter()
        .all(|value| (0.0..=1.0).contains(value))
        .then_some(uv)
}

fn inside([x, y, w, h]: [f32; 4], point: Vector2) -> bool {
    point.x >= x && point.x <= x + w && point.y >= y && point.y <= y + h
}

fn map_pointer(layout: &WorldMapLayout, physical: Vector2, scale: f32) -> (bool, Option<[f32; 2]>) {
    let logical = physical / scale;
    (
        inside(layout.frame_rect(), logical),
        canvas_uv(layout, logical),
    )
}

#[cfg(test)]
mod pointer_tests {
    use super::*;

    #[test]
    fn map_title_drag_clamps_at_effective_scale_without_grabbing_buttons_or_canvas() {
        let viewport = [1280.0, 720.0];
        let layout = placed_map_layout(viewport, None);
        let [x, y, width, height] = layout.frame_rect();
        assert_eq!(x, (viewport[0] - width) * 0.5);
        assert_eq!(y, 104.0_f32.min(viewport[1] - height));
        let scale = 1.25;
        let title = Vector2::new(x + 100.0, y + 12.0);
        let button = [x + 90.0, y + 3.0, 40.0, 20.0];
        assert!(!title_hit(
            layout.frame_rect(),
            title * scale,
            scale,
            &[button]
        ));
        assert!(title_hit(layout.frame_rect(), title * scale, scale, &[]));
        let canvas = layout.canvas_rect();
        assert!(!title_hit(
            layout.frame_rect(),
            Vector2::new(canvas[0] + 10.0, canvas[1] + 10.0) * scale,
            scale,
            &[]
        ));
        let drag = WindowDrag::begin(title, layout.origin);
        let moved = drag.position(Vector2::new(4000.0, 4000.0), viewport, [width, height]);
        assert_eq!(moved, [viewport[0] - width, viewport[1] - height]);
        let smaller = placed_map_layout([900.0, 600.0], Some(moved));
        let [sx, sy, sw, sh] = smaller.frame_rect();
        assert_eq!([sx, sy], [(900.0 - sw).max(0.0), (600.0 - sh).max(0.0)]);
    }

    #[test]
    fn reset_repositions_an_already_open_map_to_wide_slot() {
        let mut map = WorldMap::default();
        map.position = Some([15.0, 20.0]);
        let viewport = [1280.0, 720.0];
        assert_ne!(
            placed_map_layout(viewport, map.position).origin,
            placed_map_layout(viewport, None).origin
        );
        map.reset_position();
        assert_eq!(
            placed_map_layout(viewport, map.position).origin,
            placed_map_layout(viewport, None).origin
        );
    }

    #[test]
    fn scaled_physical_map_pointer_hits_logical_canvas_and_frame() {
        let layout = WorldMapLayout::for_viewport([2560.0, 1440.0]);
        let [x, y, w, h] = layout.canvas_rect();
        let center = Vector2::new(x + w * 0.5, y + h * 0.5);
        let (inside_frame, uv) = map_pointer(&layout, center * 0.5, 0.5);
        assert!(inside_frame);
        assert_eq!(uv, Some([0.5, 0.5]));
        assert_eq!(map_pointer(&layout, Vector2::ZERO, 0.5), (false, None));
    }
}

impl GameClient {
    fn world_map_viewport(&self) -> [f32; 2] {
        let size = self
            .base()
            .get_viewport()
            .map(|viewport| viewport.get_visible_rect().size)
            .unwrap_or_default();
        let scale = self.effective_ui_scale();
        [size.x / scale, size.y / scale]
    }

    fn world_map_player(&self) -> Option<WorldMapPlayer> {
        let transform = self.world.local_player_transform()?;
        let origin = transform.origin;
        let session = &self.account.session;
        let race = session
            .characters
            .iter()
            .find(|entry| Some(entry.character_id) == session.selected_character_id)
            .map(|entry| entry.race);
        Some(WorldMapPlayer {
            map_id: self.world_map_id?,
            position: engine_to_world([origin.x, origin.y, origin.z]),
            yaw: self.world.local_player_facing()?,
            faction: race.and_then(|race| self.world_map.data()?.race_faction(race)),
        })
    }

    fn world_map_view(&self) -> Option<WorldMapFrameState> {
        let data = self.world_map.data()?;
        let player = self.world_map_player();
        Some(world_map_frame_state(
            data,
            WorldMapRequest {
                visible: true,
                viewport: self.world_map_viewport(),
                map_id: self.world_map.map_id,
                hovered: self.world_map.hovered,
                player: player.as_ref(),
                quests: &self.account.quests.log,
            },
        ))
    }

    fn load_world_map_data(&mut self) -> Result<(), String> {
        if self.world_map.data.is_none() {
            let dir = self.data_root.join(DB2_DIR);
            self.world_map.data = Some(WorldMapData::load(&dir));
        }
        match self.world_map.data.as_ref() {
            Some(Err(error)) => Err(format!("World map data: {error}")),
            _ => Ok(()),
        }
    }

    /// Copy every texture the state draws out of local CASC; record which exist.
    fn cache_world_map_textures(&mut self, state: &WorldMapFrameState) {
        let unknown: Vec<u32> = world_map_texture_fdids(state)
            .into_iter()
            .filter(|fdid| !self.world_map.available.contains_key(fdid))
            .collect();
        if unknown.is_empty() {
            return;
        }
        let resolver = crate::assets::creature::local_resolver(&self.data_root);
        let textures = self.data_root.join("textures");
        for fdid in unknown {
            let path = textures.join(format!("{fdid}.blp"));
            let found = resolver.ensure_cached(fdid, &path).is_some() || path.exists();
            if !found {
                godot_warn!("World map texture FDID {fdid} is not in local CASC");
            }
            self.world_map.available.insert(fdid, found);
        }
    }

    /// The view model restricted to textures on disk: art missing from the local
    /// install stays black instead of failing the frame.
    fn drawable_world_map(&mut self) -> Result<WorldMapFrameState, String> {
        let mut state = self.world_map_view().ok_or("World map data not loaded")?;
        self.cache_world_map_textures(&state);
        let available = |fdid: &u32| self.world_map.available.get(fdid) == Some(&true);
        let tiles = state.tiles.len();
        state.tiles.retain(|tile| available(&tile.fdid));
        self.world_map.missing_tiles = tiles - state.tiles.len();
        if let Some(highlight) = state.highlight.as_mut()
            && !available(&highlight.fdid)
        {
            highlight.fdid = 0;
        }
        let chrome_missing = world_map_texture_fdids(&state)
            .into_iter()
            .find(|fdid| !available(fdid));
        match chrome_missing {
            Some(fdid) => Err(format!("World map texture FDID {fdid} missing")),
            None => Ok(state),
        }
    }

    fn open_world_map(&mut self) -> Result<(), String> {
        self.load_world_map_data()?;
        let player = self.world_map_player();
        let data = self.world_map.data().ok_or("World map data not loaded")?;
        let opened = player.as_ref().and_then(|player| player_map(data, player));
        self.world_map.map_id = opened.ok_or("World map: no map under the player")?;
        self.world_map.hovered = None;
        let id = self
            .account
            .session
            .selected_character_id
            .ok_or("World map requires selected server character ID")?;
        let path =
            game_engine_core::client_options_data::options_path().with_file_name("ui_layout.ron");
        self.world_map.position =
            game_engine_core::ui_layout_data::window_position(&path, id, "WorldMapFrame")?;
        let state = self.drawable_world_map()?;
        let mut ui = RegistryUi::new_alloc();
        ui.set_name("WorldMapUI");
        ui.set_layer(5);
        self.base_mut().add_child(&ui);
        ui.bind_mut().set_ui_scale(self.effective_ui_scale())?;
        let shown = ui.bind_mut().show_world_map(state);
        if let Err(error) = shown {
            ui.free();
            return Err(error);
        }
        self.world_map.ui = Some(ui);
        self.sync_world_map()?;
        Ok(())
    }

    /// `ToggleWorldMap`: the M binding and the minimap zone text button.
    pub(super) fn toggle_world_map(&mut self) -> Result<(), String> {
        if self.world_map.is_open() {
            self.close_world_map();
            Ok(())
        } else {
            self.open_world_map()
        }
    }

    pub(super) fn close_world_map(&mut self) {
        if let Some(ui) = self.world_map.ui.take() {
            ui.free();
        }
        self.world_map.hovered = None;
        self.world_map.position = None;
        self.world_map.drag = None;
    }

    fn world_map_toggle_pressed(&self) -> bool {
        let Some(viewport) = self.base().get_viewport() else {
            return false;
        };
        let keyboard = !viewport
            .gui_get_focus_owner()
            .is_some_and(|focus| focus.is_class("LineEdit") || focus.is_class("TextEdit"));
        let input = self.physical_input.gameplay_state(keyboard);
        self.client_options
            .bindings
            .is_just_pressed(InputAction::ToggleWorldMap, &input)
    }

    /// Per frame: the toggle binding, frame actions, and the live view model. A map
    /// failure is reported and closes the map; it does not end the session.
    pub(super) fn update_world_map(&mut self) -> Result<(), String> {
        if let Err(error) = self.drive_world_map() {
            godot_error!("World map: {error}");
            self.close_world_map();
        }
        Ok(())
    }

    fn drive_world_map(&mut self) -> Result<(), String> {
        let in_world = self.account.session.screen == SessionScreen::InWorld;
        if !in_world {
            self.close_world_map();
            return Ok(());
        }
        if self.game_menu_ui.is_some() {
            return Ok(());
        }
        if self.world_map_toggle_pressed() {
            self.toggle_world_map()?;
        }
        self.poll_world_map_actions()?;
        self.sync_world_map()
    }

    fn poll_world_map_actions(&mut self) -> Result<(), String> {
        let Some(ui) = self.world_map.ui.as_mut() else {
            return Ok(());
        };
        let action = ui.bind_mut().pop_action().to_string();
        if action == ACTION_WORLD_MAP_CLOSE {
            self.close_world_map();
        } else if let Some(id) = action.strip_prefix(ACTION_WORLD_MAP_NAV_PREFIX) {
            self.world_map.map_id = id
                .parse()
                .map_err(|_| format!("Bad world map breadcrumb action {action}"))?;
            self.world_map.hovered = None;
        } else if !action.is_empty() {
            return Err(format!("Unknown world map action: {action}"));
        }
        Ok(())
    }

    pub(super) fn sync_world_map(&mut self) -> Result<(), String> {
        if !self.world_map.is_open() {
            return Ok(());
        }
        let state = self.drawable_world_map()?;
        let scale = self.effective_ui_scale();
        let layout = placed_map_layout(self.world_map_viewport(), self.world_map.position);
        let ui = self.world_map.ui.as_mut().ok_or("World map UI vanished")?;
        ui.bind_mut().set_ui_scale(scale)?;
        ui.bind_mut().set_state(state)?;
        ui.bind_mut()
            .set_window_position("WorldMapBorderFrame", layout.origin)
    }

    pub(super) fn reset_open_world_map_position(&mut self) {
        self.world_map.reset_position();
    }

    /// Mouse over the open frame drives the map instead of the camera. Returns whether
    /// `event` was consumed; clicks on frame buttons still reach their controls.
    pub(super) fn world_map_pointer(&mut self, event: &Gd<InputEvent>) -> bool {
        if !self.world_map.is_open() {
            return false;
        }
        if self.game_menu_ui.is_some() {
            return false;
        }
        let viewport = self.world_map_viewport();
        let layout = placed_map_layout(viewport, self.world_map.position);
        let scale = self.effective_ui_scale();
        if let Ok(motion) = event.clone().try_cast::<InputEventMouseMotion>() {
            return self.world_map_motion(&motion, &layout, viewport, scale);
        }
        let Ok(button) = event.clone().try_cast::<InputEventMouseButton>() else {
            return false;
        };
        self.world_map_button(&button, &layout, scale)
    }

    fn world_map_motion(
        &mut self,
        motion: &Gd<InputEventMouseMotion>,
        layout: &WorldMapLayout,
        viewport: [f32; 2],
        scale: f32,
    ) -> bool {
        if let Some(drag) = &self.world_map.drag {
            let [_, _, width, height] = layout.frame_rect();
            self.world_map.position =
                Some(drag.position(motion.get_position() / scale, viewport, [width, height]));
            if let Err(error) = self.sync_world_map() {
                godot_error!("World map drag: {error}");
            }
            return true;
        }
        let (inside_frame, uv) = map_pointer(layout, motion.get_position(), scale);
        self.world_map.hovered = uv;
        inside_frame
    }

    fn world_map_button(
        &mut self,
        button: &Gd<InputEventMouseButton>,
        layout: &WorldMapLayout,
        scale: f32,
    ) -> bool {
        if button.get_button_index() == MouseButton::LEFT {
            if !button.is_pressed() && self.world_map.drag.take().is_some() {
                self.persist_world_map_position();
                return true;
            }
            if button.is_pressed() {
                let buttons = self.world_map_title_buttons(scale);
                if title_hit(layout.frame_rect(), button.get_position(), scale, &buttons) {
                    self.world_map.drag = Some(WindowDrag::begin(
                        button.get_position() / scale,
                        layout.origin,
                    ));
                    return true;
                }
            }
        }
        self.world_map_canvas_button(button, layout, scale)
    }

    fn persist_world_map_position(&self) {
        if let (Some(id), Some(position)) = (
            self.account.session.selected_character_id,
            self.world_map.position,
        ) {
            let path = game_engine_core::client_options_data::options_path()
                .with_file_name("ui_layout.ron");
            if let Err(error) = game_engine_core::ui_layout_data::save_window_position(
                &path,
                id,
                "WorldMapFrame",
                position,
            ) {
                godot_error!("World map placement: {error}");
            }
        }
    }

    fn world_map_canvas_button(
        &mut self,
        button: &Gd<InputEventMouseButton>,
        layout: &WorldMapLayout,
        scale: f32,
    ) -> bool {
        let (inside_frame, uv) = map_pointer(layout, button.get_position(), scale);
        // Releases always reach gameplay input so a drag begun outside cannot stick.
        if !button.is_pressed() || !inside_frame {
            return false;
        }
        if let Some(uv) = uv {
            self.navigate_world_map(button.get_button_index(), uv);
        }
        true
    }

    fn world_map_title_buttons(&self, scale: f32) -> Vec<[f32; 4]> {
        let Some(ui) = &self.world_map.ui else {
            return Vec::new();
        };
        let mut buttons = Vec::new();
        for name in ["WorldMapCloseButton", "WorldMapClose"] {
            if let Some(node) = ui.find_child_ex(name).owned(false).done()
                && let Ok(control) = node.try_cast::<godot::classes::Control>()
            {
                let rect = control.get_global_rect();
                buttons.push([
                    rect.position.x / scale,
                    rect.position.y / scale,
                    rect.size.x / scale,
                    rect.size.y / scale,
                ]);
            }
        }
        buttons
    }

    fn navigate_world_map(&mut self, button: MouseButton, uv: [f32; 2]) {
        let Some(data) = self.world_map.data() else {
            return;
        };
        let map = self.world_map.map_id;
        let target = match button {
            MouseButton::RIGHT | MouseButton::WHEEL_DOWN => zoom_out(data, map),
            MouseButton::LEFT | MouseButton::WHEEL_UP => zoom_in(data, map, uv),
            _ => None,
        };
        if let Some(target) = target {
            self.world_map.map_id = target;
            self.world_map.hovered = Some(uv);
        }
    }
}

#[godot_api(secondary)]
impl GameClient {
    /// Displayed map, the player's map and marker, for fixtures.
    #[func]
    fn world_map_state(&self) -> VarDictionary {
        let mut result = VarDictionary::new();
        let Some(data) = self.world_map.data() else {
            return result;
        };
        let Some(state) = self.world_map_view() else {
            return result;
        };
        let map = data.catalog.map(state.map_id);
        result.set("open", self.world_map.is_open());
        result.set("map_id", state.map_id as i64);
        result.set("map_name", state.map_name.as_str());
        result.set("map_kind", map.map_or(-1, |map| i64::from(map.kind)));
        let player_map_id = self
            .world_map_player()
            .and_then(|player| player_map(data, &player));
        result.set("player_map_id", player_map_id.map_or(-1, i64::from));
        result.set(
            "player_uv",
            &state
                .player
                .as_ref()
                .map(|player| Vector2::new(player.x, player.y).to_variant())
                .unwrap_or_default(),
        );
        let names: PackedStringArray = state
            .breadcrumbs
            .iter()
            .map(|crumb| GString::from(crumb.name.as_str()))
            .collect();
        let ids: PackedInt64Array = state
            .breadcrumbs
            .iter()
            .map(|crumb| i64::from(crumb.map_id))
            .collect();
        result.set("breadcrumbs", &names);
        result.set("breadcrumb_ids", &ids);
        result.set(
            "highlight",
            state
                .highlight
                .as_ref()
                .map_or("", |highlight| highlight.name.as_str()),
        );
        result.set("pin_count", state.pins.len() as i64);
        result.set("tile_count", state.tiles.len() as i64);
        result.set("missing_tiles", self.world_map.missing_tiles as i64);
        result
    }
}
