use std::sync::mpsc;

use game_engine::network_runtime::messages::{ConnectionSender, Inbox};
use shared::protocol::TransferAbortReason;

use super::*;

/// `world_safe_locs` 3599 inside the Stockade, WoW (56.6821, 0.62376, -19.2691), in Bevy space.
const STOCKADE_ARRIVAL: [f32; 3] = [56.6821, -19.2691, -0.62376];

fn app() -> App {
    let mut app = App::new();
    app.add_plugins(MinimalPlugins)
        .add_plugins(bevy::state::app::StatesPlugin)
        .insert_state(GameState::InWorld)
        .init_resource::<AdtManager>()
        .init_resource::<TerrainHeightmap>()
        .add_plugins(TransferNetworkPlugin);
    app.update();
    app
}

fn deliver<M: lightyear::prelude::Message>(app: &mut App, message: M) {
    app.insert_resource(Inbox::new(vec![message]));
    game_engine::network_events::dispatch_incoming(app.world_mut());
    app.update();
}

fn stockade_new_world() -> NewWorld {
    NewWorld {
        map_id: 34,
        map_directory: "stormwindjail".into(),
        position: STOCKADE_ARRIVAL,
        facing: 0.0,
    }
}

#[test]
fn new_world_drops_the_old_map_and_loads_the_stockade_wmo_at_the_arrival_point() {
    let mut app = app();
    let old_tile = app.world_mut().spawn_empty().id();
    {
        let mut adt_manager = app.world_mut().resource_mut::<AdtManager>();
        adt_manager.map_name = "azeroth".into();
        adt_manager.loaded.insert((30, 48), old_tile);
    }
    let player = app
        .world_mut()
        .spawn((
            LocalPlayer,
            Transform::from_xyz(-8761.85, 87.8, -848.56),
            CharacterFacing::default(),
        ))
        .id();

    deliver(&mut app, stockade_new_world());

    let adt_manager = app.world().resource::<AdtManager>();
    assert_eq!(adt_manager.map_name, "stormwindjail");
    assert!(adt_manager.loaded.is_empty(), "Stormwind tiles unloaded");
    let crate::terrain::GlobalWmo::Pending(wmo) = &adt_manager.global_wmo else {
        panic!("the Stockade is a WMO-only map");
    };
    // world/wmo/dungeon/az_stormwindprisons/stormwindjail.wmo.
    assert_eq!(wmo.fdid, Some(108_631));
    assert!(
        app.world().get_entity(old_tile).is_err(),
        "old tile despawned"
    );
    assert_eq!(
        app.world().get::<Transform>(player).unwrap().translation,
        Vec3::from(STOCKADE_ARRIVAL)
    );
    assert_eq!(
        *app.world().resource::<State<GameState>>().get(),
        GameState::Loading
    );
    assert_eq!(app.world().resource::<PendingWorldPort>().0, Some(34));
}

#[test]
fn the_world_port_ack_goes_out_once_the_new_map_is_in_the_world() {
    let mut app = app();
    deliver(&mut app, stockade_new_world());
    let (commands, sent) = mpsc::channel();
    app.insert_resource(ConnectionSender::new(Some(commands)));

    game_engine::network_events::dispatch_outgoing(app.world_mut());
    assert_eq!(sent.try_iter().count(), 0, "still loading");

    app.world_mut()
        .resource_mut::<NextState<GameState>>()
        .set(GameState::InWorld);
    app.update();
    game_engine::network_events::dispatch_outgoing(app.world_mut());
    game_engine::network_events::dispatch_outgoing(app.world_mut());
    assert_eq!(sent.try_iter().count(), 1, "one WorldPortAck");
    assert_eq!(app.world().resource::<PendingWorldPort>().0, None);
}

#[test]
fn an_aborted_transfer_shows_the_retail_error() {
    let mut app = app();
    deliver(
        &mut app,
        TransferAborted {
            map_id: 34,
            reason: TransferAbortReason::MaxPlayers,
        },
    );
    let errors = app.world().resource::<UiErrors>();
    assert_eq!(errors.lines[0].text, "Transfer Aborted: instance is full");
}
