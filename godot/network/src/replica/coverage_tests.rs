//! Missing wire/state boundaries of the stock-client equivalence fixture.

use super::*;
use bevy_replicon::prelude::Signature;
use lightyear::prelude::{MessageReceiver, MessageSystems};
use shared::protocol::{ProtocolFingerprint, ProtocolRegistrationExt, ProtocolVerified};

#[derive(Resource, Default)]
struct Fingerprints(Vec<(Entity, ProtocolFingerprint)>);

fn capture_fingerprints(
    mut receivers: Query<(Entity, &mut MessageReceiver<ProtocolFingerprint>)>,
    mut received: ResMut<Fingerprints>,
) {
    for (entity, mut receiver) in &mut receivers {
        receiver.retain_messages(|fingerprint| {
            received.0.push((entity, *fingerprint));
            true
        });
    }
}

#[test]
fn full_wire_fingerprint_matches_stock_client_including_layout_hashes() {
    let (mut server, address) = start_fixture_server();
    server.init_resource::<Fingerprints>();
    server.add_systems(
        PreUpdate,
        capture_fingerprints
            .in_set(MessageSystems::Receive)
            .after(lightyear_messages::plugin::MessagePlugin::recv),
    );
    let mut stock = stock_client(address, 9190);
    let mut native = crate::client_app();
    connect_transport(native.world_mut(), address, 9191).expect("connect native decoder client");
    native.finish();
    native.cleanup();
    Schema::from_world(native.world()).expect("native protocol has every codec");

    let deadline = Instant::now() + Duration::from_secs(10);
    while server.world().resource::<Fingerprints>().0.len() < 2 {
        assert!(
            Instant::now() < deadline,
            "both fingerprints must arrive over UDP"
        );
        server.update();
        stock.update();
        native.update();
        std::thread::sleep(Duration::from_millis(5));
    }
    let received = &server.world().resource::<Fingerprints>().0;
    assert_eq!(received.len(), 2);
    assert_ne!(received[0].0, received[1].0, "independent connections");
    assert_eq!(received[0].1, received[1].1, "all five fingerprint fields");
    assert_ne!(received[0].1.message_layouts, 0);
    assert_ne!(received[0].1.component_layouts, 0);
    for (link, _) in received {
        assert!(server.world().entity(*link).contains::<ProtocolVerified>());
    }
}

#[test]
fn missing_registered_codec_names_the_type_before_decoding() {
    #[derive(Component, serde::Serialize, serde::Deserialize)]
    struct MissingCodec(u32);

    let mut app = crate::client_app();
    app.protocol_component::<MissingCodec>().replicate();
    let error = Schema::from_world(app.world())
        .err()
        .expect("missing codec fails");
    assert!(error.contains("MissingCodec"), "{error}");
    assert!(error.contains("no replication codec"), "{error}");
}

fn update_header(flags: u8) -> Bytes {
    let mut bytes = Vec::new();
    postcard_utils::to_extend_mut(&flags, &mut bytes).unwrap();
    postcard_utils::to_extend_mut(&bevy_replicon::prelude::RepliconTick::default(), &mut bytes)
        .unwrap();
    bytes.into()
}

#[test]
fn malformed_update_flags_fail_explicitly() {
    let stock = replay_client();
    for flags in [0, 0x10, 0x88] {
        let mut replica = Replica::new(schema_of(&stock));
        let error = replica
            .apply(ReplicationBatch {
                updates: vec![update_header(flags)],
                mutations: Vec::new(),
            })
            .expect_err("empty or unknown update flags must fail");
        assert!(error.to_string().contains("update flags"), "{error}");
        assert!(replica.is_empty());
    }
}

#[test]
fn unknown_wire_codec_and_truncated_entity_data_fail_explicitly() {
    let stock = replay_client();
    let schema = schema_of(&stock);
    let mut entities = World::new();
    let entity = entities.spawn_empty().id();
    let mut payload = Vec::new();
    postcard_utils::to_extend_mut(&usize::MAX, &mut payload).unwrap();
    let mut message = update_header(super::super::CHANGES).to_vec();
    postcard_utils::entity_to_extend_mut(&entity, &mut message).unwrap();
    postcard_utils::to_extend_mut(&payload.len(), &mut message).unwrap();
    message.extend(payload);
    let mut replica = Replica::new(schema.clone());
    let error = replica
        .apply(ReplicationBatch {
            updates: vec![message.clone().into()],
            mutations: Vec::new(),
        })
        .expect_err("unregistered FnsId must fail");
    assert!(
        error.to_string().contains("unknown replication FnsId"),
        "{error}"
    );
    message.pop();
    let mut replica = Replica::new(schema);
    let error = replica
        .apply(ReplicationBatch {
            updates: vec![message.into()],
            mutations: Vec::new(),
        })
        .expect_err("entity length must be bounded by its message");
    assert!(
        error.to_string().contains("entity data exceeds message"),
        "{error}"
    );
}

#[test]
fn real_signature_mapping_with_no_local_match_equals_stock_client() {
    let mut session = Session::start(9192);
    let entity = session.spawn((position(42.0), Signature::from(2718_u64)));
    session.until("signature entity spawn", entity, |unit| {
        x_of(unit) == Some(42.0)
    });
    assert!(
        session
            .frames()
            .iter()
            .flat_map(|frame| &frame.updates)
            .any(|message| {
                message
                    .first()
                    .is_some_and(|flags| flags & super::super::MAPPINGS != 0)
            }),
        "server encoded a real mapping array"
    );
    session.edit(entity, |entity| {
        entity.insert(position(43.0));
    });
    session.until("signature entity mutation", entity, |unit| {
        x_of(unit) == Some(43.0)
    });
    session.server.world_mut().despawn(entity);
    session.until("signature entity despawn", entity, |unit| unit.is_none());
}

#[test]
fn visibility_regain_before_draining_changes_emits_one_current_change() {
    let mut session = Session::start(9193);
    let entity = session.spawn((player("Visibility"), position(5.0), UnitLevel(7)));
    session.until("spawn", entity, |unit| x_of(unit) == Some(5.0));
    session
        .server
        .world_mut()
        .lose_visibility(entity, session.link);
    session.until("visibility loss", entity, |unit| unit.is_none());
    session.edit(entity, |entity| {
        entity.remove::<UnitLevel>();
    });
    session
        .server
        .world_mut()
        .gain_visibility(entity, session.link);
    session.until("visibility regain", entity, |unit| x_of(unit) == Some(5.0));
    assert!(!has::<UnitLevel>(session.replica.unit(entity.to_bits())));
    let changes = session.replica.drain_changes();
    assert_eq!(
        changes
            .iter()
            .filter(|change| matches!(change,
        UnitChange::Changed { server_id, .. } if *server_id == entity.to_bits()))
            .count(),
        1,
        "one coalesced change for current incarnation: {changes:?}"
    );
    assert!(changes.contains(&UnitChange::Despawned(entity.to_bits())));
    assert!(session.replica.drain_changes().is_empty());
}

#[test]
fn despawn_reuses_storage_without_leaking_components_to_new_server_entity() {
    let mut session = Session::start(9194);
    let old = session.spawn((
        position(1.0),
        Health {
            current: 90.0,
            max: 100.0,
        },
        UnitAuras::default(),
    ));
    session.until("old entity", old, has::<UnitAuras>);
    session.server.world_mut().despawn(old);
    session.until("old despawn", old, |unit| unit.is_none());
    let new = session.spawn((position(2.0), player("New")));
    session.until("new entity", new, |unit| x_of(unit) == Some(2.0));
    let unit = session.replica.unit(new.to_bits()).unwrap();
    assert!(!unit.has::<Health>());
    assert!(!unit.has::<UnitAuras>());
    assert!(unit.has::<Player>());
    assert!(session.replica.unit(old.to_bits()).is_none());
}

#[test]
fn packet_split_mutations_match_stock_when_same_tick_messages_arrive_separately() {
    let mut session = Session::start(9195);
    let entities: Vec<_> = (0..96)
        .map(|index| session.spawn(position(index as f32)))
        .collect();
    session.run_until("all initial positions", |session| {
        entities.iter().enumerate().all(|(index, entity)| {
            x_of(session.replica.unit(entity.to_bits())) == Some(index as f32)
        })
    });
    for (index, entity) in entities.iter().enumerate() {
        session.edit(*entity, |entity| {
            entity.insert(position(1000.0 + index as f32));
        });
    }
    session.run_until("all mutated positions", |session| {
        entities.iter().enumerate().all(|(index, entity)| {
            x_of(session.replica.unit(entity.to_bits())) == Some(1000.0 + index as f32)
        })
    });
    let mut mutations = all_mutations(session.frames());
    assert!(
        mutations.iter().any(|message| {
            let mut data = message.clone();
            let _: bevy_replicon::prelude::RepliconTick =
                postcard_utils::from_buf(&mut data).unwrap();
            let _: bevy_replicon::prelude::RepliconTick =
                postcard_utils::from_buf(&mut data).unwrap();
            postcard_utils::from_buf::<usize, _>(&mut data).unwrap() > 1
        }),
        "real server split one tick across multiple messages"
    );
    let mut frames = update_frames(session.frames());
    mutations.reverse();
    frames.extend(mutations.into_iter().map(|message| Frame {
        mutations: vec![message],
        ..Default::default()
    }));
    let replica = replay_both(&frames);
    for (index, entity) in entities.iter().enumerate() {
        assert_eq!(
            x_of(replica.unit(entity.to_bits())),
            Some(1000.0 + index as f32)
        );
    }
}

#[test]
fn malformed_mutation_headers_are_not_acknowledged() {
    let mut valid = Vec::new();
    for value in [0_usize, 1, 1] {
        postcard_utils::to_extend_mut(&value, &mut valid).unwrap();
    }
    valid.extend_from_slice(&[0x34, 0x12]);
    assert_eq!(
        acknowledgments(&[valid.clone().into()]).unwrap(),
        Bytes::from_static(&[0x34, 0x12])
    );
    for end in 0..valid.len() {
        assert!(
            acknowledgments(&[valid[..end].to_vec().into()]).is_err(),
            "truncation at {end}"
        );
    }
}
