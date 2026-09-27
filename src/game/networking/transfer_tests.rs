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
    assert_eq!(
        *app.world().resource::<PendingWorldPort>(),
        PendingWorldPort::Loading {
            map_id: 34,
            arrival: 1
        }
    );
}

fn state(app: &App) -> GameState {
    *app.world().resource::<State<GameState>>().get()
}

/// Update, then flush outgoing messages as the network worker does each frame.
fn frame(app: &mut App) {
    app.update();
    game_engine::network_events::dispatch_outgoing(app.world_mut());
}

/// Retail keeps the loading screen until the destination's objects arrived: the loaded
/// Stockade answers one `WorldPortAck`, and the screen stays until the server's arrival
/// raises the player's `WorldArrival` from 2 to 3 alongside the creatures.
#[test]
fn the_loading_screen_ends_only_after_the_acked_arrival_brings_the_destination() {
    let mut app = app();
    app.add_systems(
        Update,
        crate::game_state::check_loading_complete.run_if(in_state(GameState::Loading)),
    );
    let (commands, sent) = mpsc::channel();
    app.insert_resource(ConnectionSender::new(Some(commands)));
    let player = app
        .world_mut()
        .spawn((LocalPlayer, Transform::default(), WorldArrival(2)))
        .id();
    app.insert_resource(Inbox::new(vec![stockade_new_world()]));
    game_engine::network_events::dispatch_incoming(app.world_mut());
    for _ in 0..3 {
        frame(&mut app);
    }
    assert_eq!(state(&app), GameState::Loading);
    assert_eq!(
        sent.try_iter().count(),
        0,
        "the Stockade WMO is still loading"
    );

    app.world_mut().resource_mut::<AdtManager>().global_wmo =
        crate::terrain::GlobalWmo::Spawned(Entity::PLACEHOLDER);
    for _ in 0..3 {
        frame(&mut app);
    }
    assert_eq!(sent.try_iter().count(), 1, "one WorldPortAck");
    assert_eq!(state(&app), GameState::Loading, "no arrival yet");
    assert_eq!(
        *app.world().resource::<PendingWorldPort>(),
        PendingWorldPort::Arriving { arrival: 3 }
    );

    app.world_mut().entity_mut(player).insert(WorldArrival(3));
    for _ in 0..2 {
        frame(&mut app);
    }
    assert_eq!(state(&app), GameState::InWorld);
    assert_eq!(sent.try_iter().count(), 0, "still one WorldPortAck");
    assert_eq!(
        *app.world().resource::<PendingWorldPort>(),
        PendingWorldPort::None
    );
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
