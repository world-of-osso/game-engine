//! Flight master networking (docs/specs/flight-master.md): `TaxiMap` opens the flight
//! map ([`TaxiMapState`]), `TaxiNodeDiscovered` shows `ERR_NEWTAXIPATH`, `TaxiFailed`
//! its Retail error, and the end of the flight master interaction closes the map.
//! [`TaxiRequest`]s go out as `ActivateTaxi` and `CloseInteraction`.

use bevy::ecs::system::SystemParam;
use bevy::prelude::*;
use game_engine::network_runtime::messages::{MessageReceivers, MessageSenders};
use game_engine::quest_runtime::{NpcFrameEvent, NpcInteractionRequest};
use game_engine::taxi_state::{TaxiMapState, TaxiRequest};
use game_engine::ui::ui_errors::UiErrors;
use shared::protocol::{ActivateTaxi, TaxiChannel, TaxiFailed, TaxiMap, TaxiNodeDiscovered};

use crate::game_state::GameState;

/// `ERR_NEWTAXIPATH`.
const NEW_TAXI_PATH: &str = "New location discovered!";

pub struct TaxiNetworkPlugin;

impl Plugin for TaxiNetworkPlugin {
    fn build(&self, app: &mut App) {
        use game_engine::network_events::{add_message_route, register_message_handler};

        app.init_resource::<TaxiMapState>()
            .init_resource::<UiErrors>()
            .add_message::<TaxiRequest>()
            .add_message::<NpcFrameEvent>()
            .add_message::<NpcInteractionRequest>();
        let handler = register_message_handler::<TaxiMap, _>(app, receive_taxi, in_world);
        add_message_route::<TaxiNodeDiscovered>(app, handler);
        add_message_route::<TaxiFailed>(app, handler);
        app.add_systems(
            Update,
            (close_on_interaction_end, send_taxi_requests)
                .chain()
                .run_if(in_state(GameState::InWorld)),
        );
        app.add_systems(OnExit(GameState::InWorld), reset_taxi);
    }
}

fn in_world(world: &World) -> bool {
    *world.resource::<State<GameState>>().get() == GameState::InWorld
}

#[derive(SystemParam)]
struct TaxiReceivers<'w, 's> {
    maps: MessageReceivers<'w, 's, TaxiMap>,
    discovered: MessageReceivers<'w, 's, TaxiNodeDiscovered>,
    failed: MessageReceivers<'w, 's, TaxiFailed>,
}

fn receive_taxi(
    mut receivers: TaxiReceivers,
    mut taxi: ResMut<TaxiMapState>,
    mut errors: ResMut<UiErrors>,
) {
    for inbox in receivers.discovered.iter_mut() {
        if inbox.receive().count() > 0 {
            errors.add(NEW_TAXI_PATH);
        }
    }
    for inbox in receivers.maps.iter_mut() {
        for map in inbox.receive() {
            taxi.open(map);
        }
    }
    for inbox in receivers.failed.iter_mut() {
        for failed in inbox.receive() {
            errors.add(failed.error.message());
        }
    }
}

/// `InteractionClosed` for the flight master (take-off, walked away).
fn close_on_interaction_end(
    mut events: MessageReader<NpcFrameEvent>,
    mut taxi: ResMut<TaxiMapState>,
) {
    for event in events.read() {
        if let NpcFrameEvent::Closed { npc } = *event
            && taxi.npc == Some(npc)
        {
            taxi.close();
        }
    }
}

fn send_taxi_requests(
    mut requests: MessageReader<TaxiRequest>,
    mut taxi: ResMut<TaxiMapState>,
    mut activate: MessageSenders<ActivateTaxi>,
    mut interactions: MessageWriter<NpcInteractionRequest>,
) {
    for request in requests.read() {
        let Some(npc) = taxi.npc else {
            continue;
        };
        match *request {
            TaxiRequest::Fly { destination } => {
                for mut sender in activate.iter_mut() {
                    sender.send::<TaxiChannel>(ActivateTaxi { npc, destination });
                }
            }
            TaxiRequest::Close => {
                taxi.close();
                interactions.write(NpcInteractionRequest::Close { npc });
            }
        }
    }
}

fn reset_taxi(mut taxi: ResMut<TaxiMapState>) {
    taxi.close();
}
