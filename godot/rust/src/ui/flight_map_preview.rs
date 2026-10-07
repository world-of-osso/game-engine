//! Offline authored flight-map chrome and destination tooltip, without a server.
use game_engine_ui_model::{
    flight_map::{FlightPin, FlightProjection},
    flight_map_component::{FlightMapView, flight_map_screen},
    world_map_frame_component::WorldMapFrameState,
};
use godot::prelude::*;
use shared::protocol::TaxiNodeState;

use super::{RegistryUi, party_preview};

#[godot_api(secondary)]
impl RegistryUi {
    #[func]
    fn show_flight_map_preview(&mut self) -> GString {
        let result = party_preview::load_data_root().and_then(|()| {
            ui_toolkit::atlas::set_thread_skin(ui_toolkit::atlas::ActiveSkin::Modern);
            self.set_ui_scale(1.0)?;
            self.show_quest_window(flight_map_preview(), flight_map_screen)
        });
        GString::from(result.err().unwrap_or_default().as_str())
    }
}

fn flight_map_preview() -> FlightMapView {
    FlightMapView {
        map: WorldMapFrameState {
            viewport: [1920.0, 1080.0],
            ..Default::default()
        },
        projection: FlightProjection {
            pins: vec![FlightPin {
                node: 4,
                name: "Sentinel Hill, Westfall".into(),
                state: TaxiNodeState::Reachable,
                cost: 5,
                uv: [0.4, 0.6],
            }],
            routes: vec![],
        },
        hovered: Some(4),
    }
}
