use bevy::ecs::system::RunSystemOnce;
use game_engine::network_runtime::messages::Inbox;
use shared::protocol::XpGainReason;

use super::*;

/// Server entity bits of the killed Kobold Vermin.
const KOBOLD_SERVER: u64 = 0x0000_0001_0000_0412;

fn fixture() -> App {
    let mut app = App::new();
    app.init_resource::<ExperienceState>()
        .init_resource::<ChatState>()
        .init_resource::<ReplicationMirrorMap>()
        .init_resource::<Inbox<PlayerXpUpdate>>()
        .init_resource::<Inbox<LogXpGain>>();
    let kobold = app
        .world_mut()
        .spawn(Npc {
            template_id: 6,
            name: "Kobold Vermin".into(),
        })
        .id();
    app.world_mut()
        .resource_mut::<ReplicationMirrorMap>()
        .insert(Entity::from_bits(KOBOLD_SERVER), kobold);
    app
}

fn receive(app: &mut App, updates: Vec<PlayerXpUpdate>, gains: Vec<LogXpGain>) {
    app.world_mut().insert_resource(Inbox::new(updates));
    app.world_mut().insert_resource(Inbox::new(gains));
    app.world_mut()
        .run_system_once(receive_experience)
        .expect("receive_experience runs");
}

fn chat_lines(app: &App) -> Vec<String> {
    app.world()
        .resource::<ChatState>()
        .messages
        .iter()
        .map(|message| message.text.clone())
        .collect()
}

#[test]
fn kill_updates_the_bar_and_prints_the_gain_with_the_victim_name() {
    let mut app = fixture();
    receive(
        &mut app,
        vec![PlayerXpUpdate {
            xp: 0,
            next_level_xp: 400,
            rested_xp: 0,
        }],
        vec![],
    );
    receive(
        &mut app,
        vec![PlayerXpUpdate {
            xp: 45,
            next_level_xp: 400,
            rested_xp: 0,
        }],
        vec![LogXpGain {
            victim: Some(KOBOLD_SERVER),
            original: 45,
            amount: 45,
            group_bonus: 1.0,
            reason: XpGainReason::Kill,
        }],
    );

    assert_eq!(
        app.world().resource::<ExperienceState>().0,
        Some(PlayerXpUpdate {
            xp: 45,
            next_level_xp: 400,
            rested_xp: 0,
        })
    );
    assert_eq!(
        chat_lines(&app),
        ["Kobold Vermin dies, you gain 45 experience."]
    );
}
