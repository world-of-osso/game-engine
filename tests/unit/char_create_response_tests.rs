use super::*;
use crate::networking_auth::{CharacterList, receive_create_character_response};
use game_engine::network_events::worker_relays;
use game_engine::network_runtime::worker::MainUpdate;
use lightyear::prelude::client::ClientPlugins;
use lightyear::prelude::{
    ChannelRegistry, Connected as TransportConnected, Link, Linked, MessageReceiver, MessageSender,
    PeerId, RemoteId, Transport,
};
use shared::protocol::{AuthChannel, CharacterListEntry, CreateCharacterResponse};
use std::sync::mpsc::{self, Receiver};

struct ResponseTransport {
    worker: App,
    peer: Entity,
    updates: Receiver<MainUpdate>,
}

fn response_fixture() -> (App, ResponseTransport) {
    let mut app = App::new();
    app.add_plugins(bevy::state::app::StatesPlugin);
    app.insert_resource(State::new(GameState::CharCreate));
    app.init_resource::<NextState<GameState>>();
    app.init_resource::<CharacterList>();
    app.init_resource::<CharCreateState>();
    app.add_observer(handle_create_response);
    game_engine::network_events::register_message_handler::<CreateCharacterResponse, _>(
        &mut app,
        receive_create_character_response,
        |_| true,
    );
    app.finish();
    app.cleanup();

    let mut worker = App::new();
    worker.add_plugins(bevy::state::app::StatesPlugin);
    worker.add_plugins(ClientPlugins::default());
    worker.add_plugins(shared::ProtocolPlugin);
    let (publish, updates) = mpsc::channel();
    for install in worker_relays(app.world()) {
        install(&mut worker, publish.clone());
    }
    worker.finish();
    worker.cleanup();
    let registry = worker.world().resource::<ChannelRegistry>();
    let mut transport = Transport::default();
    transport.add_sender_from_registry::<AuthChannel>(registry);
    transport.add_receiver_from_registry::<AuthChannel>(registry);
    let peer = worker
        .world_mut()
        .spawn((
            Link::default(),
            transport,
            Linked,
            TransportConnected,
            RemoteId(PeerId::Local(0)),
            MessageReceiver::<CreateCharacterResponse>::default(),
            MessageSender::<CreateCharacterResponse>::default(),
        ))
        .id();
    (
        app,
        ResponseTransport {
            worker,
            peer,
            updates,
        },
    )
}

fn deliver_response(
    app: &mut App,
    transport: &mut ResponseTransport,
    response: CreateCharacterResponse,
) {
    transport
        .worker
        .world_mut()
        .entity_mut(transport.peer)
        .get_mut::<MessageSender<CreateCharacterResponse>>()
        .unwrap()
        .send::<AuthChannel>(response);
    transport.worker.world_mut().run_schedule(PostUpdate);
    {
        let mut entity = transport.worker.world_mut().entity_mut(transport.peer);
        let mut link = entity.get_mut::<Link>().unwrap();
        let packets: Vec<_> = link.send.drain().collect();
        assert!(
            !packets.is_empty(),
            "response must serialize into transport packets"
        );
        for packet in packets {
            link.recv.push_raw(packet);
        }
    }
    transport.worker.world_mut().run_schedule(PreUpdate);
    transport.worker.world_mut().run_schedule(Update);
    transport.worker.world_mut().run_schedule(Last);
    let update = transport
        .updates
        .try_recv()
        .expect("decoded response must cross the worker relay");
    update(app.world_mut());
    for update in transport.updates.try_iter() {
        update(app.world_mut());
    }
    game_engine::network_events::dispatch_incoming(app.world_mut());
}

#[test]
fn creation_response_updates_roster_and_transitions_scene_once() {
    let (mut app, mut transport) = response_fixture();
    deliver_response(
        &mut app,
        &mut transport,
        CreateCharacterResponse {
            success: true,
            character: Some(CharacterListEntry {
                character_id: 42,
                name: "Alessio".to_owned(),
                level: 1,
                race: 1,
                class: 1,
                appearance: Default::default(),
                equipment_appearance: Default::default(),
            }),
            error: None,
        },
    );
    let roster = &app.world().resource::<CharacterList>().0;
    assert_eq!(roster.len(), 1);
    assert_eq!(
        (roster[0].character_id, roster[0].name.as_str()),
        (42, "Alessio")
    );
    assert!(matches!(
        app.world().resource::<NextState<GameState>>(),
        NextState::Pending(GameState::CharSelect)
    ));
    game_engine::network_events::dispatch_incoming(app.world_mut());
    assert_eq!(app.world().resource::<CharacterList>().0.len(), 1);
}

#[test]
fn creation_failure_displays_server_error_without_transition() {
    for error in [Some("Name already taken".to_owned()), None] {
        let (mut app, mut transport) = response_fixture();
        let expected = error
            .clone()
            .unwrap_or_else(|| "Creation failed".to_owned());
        deliver_response(
            &mut app,
            &mut transport,
            CreateCharacterResponse {
                success: false,
                character: None,
                error,
            },
        );
        assert_eq!(
            app.world()
                .resource::<CharCreateState>()
                .error_text
                .as_deref(),
            Some(expected.as_str())
        );
        assert!(app.world().resource::<CharacterList>().0.is_empty());
        assert!(matches!(
            app.world().resource::<NextState<GameState>>(),
            NextState::Unchanged
        ));
    }
}

#[test]
fn creation_response_after_scene_exit_does_not_require_scene_resources() {
    let (mut app, mut transport) = response_fixture();
    app.world_mut()
        .insert_resource(State::new(GameState::CharSelect));
    app.world_mut().remove_resource::<CharCreateState>();
    deliver_response(
        &mut app,
        &mut transport,
        CreateCharacterResponse {
            success: false,
            character: None,
            error: Some("Late response".to_owned()),
        },
    );
    assert!(matches!(
        app.world().resource::<NextState<GameState>>(),
        NextState::Unchanged
    ));
}
