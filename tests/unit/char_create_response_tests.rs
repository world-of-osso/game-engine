use super::*;
use crate::networking_auth::{CharacterList, receive_create_character_response};
use lightyear::prelude::client::ClientPlugins;
use shared::protocol::{CharacterListEntry, CreateCharacterResponse};

fn response_fixture() -> (App, Entity) {
    let mut app = App::new();
    app.add_plugins(bevy::state::app::StatesPlugin);
    app.add_plugins(ClientPlugins::default());
    app.add_plugins(shared::ProtocolPlugin);
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
    let registry = app.world().resource::<ChannelRegistry>();
    let mut transport = Transport::default();
    transport.add_sender_from_registry::<AuthChannel>(registry);
    transport.add_receiver_from_registry::<AuthChannel>(registry);
    let peer = app
        .world_mut()
        .spawn((
            Link::default(),
            transport,
            Linked,
            Connected,
            RemoteId(PeerId::Local(0)),
            MessageReceiver::<CreateCharacterResponse>::default(),
            MessageSender::<CreateCharacterResponse>::default(),
        ))
        .id();
    (app, peer)
}

fn deliver_response(app: &mut App, peer: Entity, response: CreateCharacterResponse) {
    app.world_mut()
        .entity_mut(peer)
        .get_mut::<MessageSender<CreateCharacterResponse>>()
        .unwrap()
        .send::<AuthChannel>(response);
    app.world_mut().run_schedule(PostUpdate);
    {
        let mut entity = app.world_mut().entity_mut(peer);
        let mut link = entity.get_mut::<Link>().unwrap();
        let packets: Vec<_> = link.send.drain().collect();
        assert!(!packets.is_empty());
        for packet in packets {
            link.recv.push_raw(packet);
        }
    }
    app.world_mut().run_schedule(PreUpdate);
    game_engine::network_events::dispatch_incoming(app.world_mut());
}

#[test]
fn creation_response_updates_roster_and_transitions_scene_once() {
    let (mut app, peer) = response_fixture();
    deliver_response(
        &mut app,
        peer,
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
        let (mut app, peer) = response_fixture();
        let expected = error
            .clone()
            .unwrap_or_else(|| "Creation failed".to_owned());
        deliver_response(
            &mut app,
            peer,
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
    let (mut app, peer) = response_fixture();
    app.world_mut()
        .insert_resource(State::new(GameState::CharSelect));
    app.world_mut().remove_resource::<CharCreateState>();
    deliver_response(
        &mut app,
        peer,
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
