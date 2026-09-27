use bevy::ecs::system::RunSystemOnce;
use game_engine::network_runtime::messages::Inbox;
use shared::protocol::InstanceLockInfo;

use super::*;

fn fixture() -> App {
    let mut app = App::new();
    app.add_plugins(MinimalPlugins)
        .add_message::<InstanceCommand>()
        .insert_resource(InstanceCatalog::load().unwrap())
        .init_resource::<InstanceState>()
        .init_resource::<ChatState>()
        .init_resource::<Inbox<DungeonDifficultySet>>()
        .init_resource::<Inbox<WorldServerInfo>>()
        .init_resource::<Inbox<InstanceInfo>>()
        .init_resource::<Inbox<InstanceSaveCreated>>()
        .init_resource::<Inbox<InstanceReset>>()
        .init_resource::<Inbox<InstanceResetFailed>>()
        .init_resource::<Inbox<RaidInstanceMessage>>();
    app.update();
    app
}

fn deliver<M: lightyear::prelude::Message>(app: &mut App, messages: Vec<M>) {
    app.world_mut().insert_resource(Inbox::new(messages));
    app.world_mut().run_system_once(receive_instance).unwrap();
}

fn chat_lines(app: &App) -> Vec<String> {
    app.world()
        .resource::<ChatState>()
        .messages
        .iter()
        .map(|message| message.text.clone())
        .collect()
}

fn state(app: &App) -> &InstanceState {
    app.world().resource::<InstanceState>()
}

#[test]
fn a_difficulty_change_prints_the_retail_line_but_the_login_value_does_not() {
    let mut app = fixture();
    deliver(&mut app, vec![DungeonDifficultySet { difficulty_id: 1 }]);
    assert!(chat_lines(&app).is_empty());
    deliver(&mut app, vec![DungeonDifficultySet { difficulty_id: 2 }]);
    assert_eq!(chat_lines(&app), ["Dungeon Difficulty set to Heroic."]);
    assert_eq!(state(&app).dungeon_difficulty, Some(2));
}

#[test]
fn entering_a_heroic_copy_records_its_difficulty_and_asks_for_raid_info() {
    let mut app = fixture();
    deliver(
        &mut app,
        vec![WorldServerInfo {
            map_id: 670,
            difficulty_id: 2,
        }],
    );
    assert!(state(&app).in_instance());
    let requested: Vec<InstanceCommand> = app
        .world_mut()
        .resource_mut::<Messages<InstanceCommand>>()
        .drain()
        .collect();
    assert_eq!(requested, vec![InstanceCommand::RequestRaidInfo]);
}

#[test]
fn saves_resets_and_expiries_print_their_system_lines() {
    let mut app = fixture();
    deliver(&mut app, vec![InstanceSaveCreated]);
    deliver(&mut app, vec![InstanceReset { map_id: 34 }]);
    deliver(
        &mut app,
        vec![InstanceResetFailed {
            map_id: 670,
            reason: InstanceResetFailedReason::PlayersInside,
        }],
    );
    deliver(
        &mut app,
        vec![RaidInstanceMessage {
            kind: RaidInstanceMessageType::Expired,
            map_id: 670,
            difficulty_id: 2,
        }],
    );
    assert_eq!(
        chat_lines(&app),
        [
            "You are now saved to this instance",
            "Stormwind Stockade has been reset.",
            "Cannot reset Grim Batol.  There are players still inside the instance.",
            "Your instance lock for Grim Batol has expired.",
        ]
    );
}

#[test]
fn the_lock_list_replaces_the_saved_instances() {
    let mut app = fixture();
    let lock = InstanceLockInfo {
        map_id: 670,
        difficulty_id: 2,
        instance_id: 3,
        time_remaining_secs: 10_800,
        completed_mask: 1 << 3,
        locked: true,
        extended: false,
    };
    deliver(&mut app, vec![InstanceInfo { locks: vec![lock] }]);
    assert_eq!(state(&app).saved, vec![lock]);
}
