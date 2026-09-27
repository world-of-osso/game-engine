use game_engine::network_runtime::messages::Inbox;

use super::*;

/// Hogger's server entity bits in the live Stockade proof; any bits do.
const HOGGER: u64 = 0x1_0000_2a17;
const ADD: u64 = 0x1_0000_2a18;

fn app() -> App {
    let mut app = App::new();
    app.add_plugins(MinimalPlugins)
        .add_plugins(bevy::state::app::StatesPlugin)
        .insert_state(GameState::InWorld)
        .add_plugins(EncounterNetworkPlugin);
    app.update();
    app
}

fn deliver<M: lightyear::prelude::Message>(app: &mut App, message: M) {
    app.insert_resource(Inbox::new(vec![message]));
    game_engine::network_events::dispatch_incoming(app.world_mut());
    app.update();
}

#[test]
fn hogger_encounter_fills_boss1_and_its_death_clears_it() {
    let mut app = app();
    deliver(
        &mut app,
        EncounterStart {
            encounter_id: 1144,
            difficulty_id: 1,
            group_size: 1,
        },
    );
    deliver(
        &mut app,
        EncounterEngageUnit {
            unit: HOGGER,
            target_frame_priority: 1,
        },
    );
    let frames = app.world().resource::<EncounterFrames>();
    assert_eq!(frames.encounter, Some(1144));
    assert_eq!(frames.boss_units().collect::<Vec<_>>(), [HOGGER]);
    deliver(&mut app, EncounterDisengageUnit { unit: HOGGER });
    deliver(
        &mut app,
        EncounterEnd {
            encounter_id: 1144,
            difficulty_id: 1,
            group_size: 1,
            success: true,
        },
    );
    let frames = app.world().resource::<EncounterFrames>();
    assert_eq!(frames.encounter, None);
    assert!(frames.bosses.is_empty());
}

#[test]
fn boss_units_sort_by_priority_and_ignore_a_repeated_engage() {
    let mut frames = EncounterFrames::default();
    frames.engage(ADD, 2);
    frames.engage(HOGGER, 1);
    frames.engage(ADD, 2);
    assert_eq!(frames.boss_units().collect::<Vec<_>>(), [HOGGER, ADD]);
}

#[test]
fn the_loading_screen_clears_the_boss_frames() {
    let mut app = app();
    deliver(
        &mut app,
        EncounterEngageUnit {
            unit: HOGGER,
            target_frame_priority: 1,
        },
    );
    app.world_mut()
        .resource_mut::<NextState<GameState>>()
        .set(GameState::Loading);
    app.update();
    assert!(app.world().resource::<EncounterFrames>().bosses.is_empty());
}
