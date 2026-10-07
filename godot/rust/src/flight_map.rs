//! Native taxi frame. Flights themselves remain replicated MovementControl.
use crate::{GameClient, frame_error::FrameError, ui::RegistryUi};
use game_engine_session::SessionScreen;
use game_engine_ui_model::flight_map::{self as model, FlightMapSession};
use game_engine_ui_model::flight_map_component::{
    CANVAS_SIZE, FRAME_SIZE, FlightMapView, flight_map_screen, frame_origin,
};
use game_engine_ui_model::world_map_view_data::{WorldMapRequest, world_map_frame_state};
use godot::classes::{InputEvent, InputEventMouseButton, InputEventMouseMotion};
use godot::prelude::*;

#[path = "flight_map_preview.rs"]
mod flight_map_preview;

#[derive(Default)]
pub(crate) struct FlightMap {
    pub session: FlightMapSession,
    ui: Option<Gd<RegistryUi>>,
    cached_textures: std::collections::HashSet<u32>,
    missing_tiles: usize,
}

impl GameClient {
    pub(super) fn open_flight_map(&mut self, map: shared::protocol::TaxiMap) -> Result<(), String> {
        self.load_world_map_data()?;
        self.close_world_map();
        self.flight_map.session.open(map);
        self.sync_flight_map()
    }

    fn flight_map_view(&self) -> Result<FlightMapView, String> {
        let map = self
            .flight_map
            .session
            .map
            .as_ref()
            .ok_or("Taxi map not open")?;
        let data = self
            .world_map
            .data()
            .ok_or("World map catalog not loaded")?;
        let map_id = model::continent_map(&data.catalog, map)
            .ok_or("No continent map for current taxi node")?;
        let size = self
            .base()
            .get_viewport()
            .ok_or("Flight map has no viewport")?
            .get_visible_rect()
            .size;
        let scale = self.effective_ui_scale();
        let state = world_map_frame_state(
            data,
            WorldMapRequest {
                visible: true,
                viewport: [size.x / scale, size.y / scale],
                map_id,
                hovered: None,
                player: None,
                quests: &[],
                quest_areas: &[],
                vignettes: &[],
            },
        );
        Ok(FlightMapView {
            map: state,
            projection: model::project(&data.catalog, map, self.flight_map.session.hovered),
            hovered: self.flight_map.session.hovered,
        })
    }

    fn cache_flight_map_textures(&mut self, view: &FlightMapView) {
        let fdids = crate::quests::screen_texture_fdids(view.clone(), flight_map_screen);
        let new: Vec<_> = fdids
            .into_iter()
            .filter(|id| self.flight_map.cached_textures.insert(*id))
            .collect();
        let resolver = crate::assets::creature::local_resolver(&self.data_root);
        for fdid in new {
            let path = self.data_root.join("textures").join(format!("{fdid}.blp"));
            if !path.exists() && resolver.ensure_cached(fdid, &path).is_none() {
                godot_warn!("Flight map texture FDID {fdid} is unavailable in local CASC");
            }
        }
    }

    fn sync_flight_map(&mut self) -> Result<(), String> {
        let mut view = self.flight_map_view()?;
        self.cache_flight_map_textures(&view);
        let tiles = view.map.tiles.len();
        view.map.tiles.retain(|tile| {
            self.data_root
                .join("textures")
                .join(format!("{}.blp", tile.fdid))
                .exists()
        });
        self.flight_map.missing_tiles = tiles - view.map.tiles.len();
        if self.flight_map.ui.is_none() {
            let mut ui = RegistryUi::new_alloc();
            ui.set_name("FlightMapUI");
            ui.set_layer(6);
            self.base_mut().add_child(&ui);
            if let Err(error) = ui
                .bind_mut()
                .show_quest_window(view.clone(), flight_map_screen)
            {
                ui.free();
                return Err(error);
            }
            self.flight_map.ui = Some(ui);
        }
        let scale = self.effective_ui_scale();
        let ui = self.flight_map.ui.as_mut().ok_or("Flight map UI missing")?;
        ui.bind_mut().set_ui_scale(scale)?;
        ui.bind_mut().set_state(view)
    }

    fn free_flight_map_ui(&mut self) {
        if let Some(ui) = self.flight_map.ui.take() {
            ui.free();
        }
    }

    pub(super) fn close_flight_map_interaction(&mut self) -> Result<(), String> {
        let npc = self.flight_map.session.map.as_ref().map(|map| map.npc);
        self.flight_map.session.close();
        self.free_flight_map_ui();
        if let Some(npc) = npc {
            self.account
                .send_close_interaction(npc)
                .map_err(|error| error.to_string())?;
        }
        Ok(())
    }

    pub(super) fn update_flight_map(&mut self) -> Result<(), FrameError> {
        if self.account.session.screen != SessionScreen::InWorld {
            self.flight_map.session.close();
        }
        if self.flight_map.session.map.is_none() {
            self.free_flight_map_ui();
            return Ok(());
        }
        let action = self
            .flight_map
            .ui
            .as_mut()
            .map(|ui| ui.bind_mut().pop_action().to_string())
            .unwrap_or_default();
        if action == model::CLOSE_ACTION {
            self.close_flight_map_interaction()?;
            return Ok(());
        }
        if let Some(request) = self.flight_map.session.click(&action)? {
            self.free_flight_map_ui();
            self.account.send_activate_taxi(request)?;
            return Ok(());
        }
        self.sync_flight_map()?;
        Ok(())
    }

    pub(super) fn flight_map_pointer(&mut self, event: &Gd<InputEvent>) -> bool {
        if self.flight_map.session.map.is_none() {
            return false;
        }
        let point = if let Ok(motion) = event.clone().try_cast::<InputEventMouseMotion>() {
            motion.get_position()
        } else if let Ok(button) = event.clone().try_cast::<InputEventMouseButton>() {
            button.get_position()
        } else {
            return false;
        };
        let Ok(view) = self.flight_map_view() else {
            return false;
        };
        let [x, y] = frame_origin(view.map.viewport);
        let point = point / self.effective_ui_scale();
        let [w, h] = FRAME_SIZE;
        let inside = point.x >= x && point.x <= x + w && point.y >= y && point.y <= y + h;
        let [cw, ch] = CANVAS_SIZE;
        self.flight_map.session.hovered = view
            .projection
            .pins
            .iter()
            .find(|pin| {
                let center = Vector2::new(x + 1.0 + pin.uv[0] * cw, y + 20.0 + pin.uv[1] * ch);
                let delta = point - center;
                delta.x.abs() <= pin.size() / 2.0 && delta.y.abs() <= pin.size() / 2.0
            })
            .map(|pin| pin.node);
        inside
    }
}
