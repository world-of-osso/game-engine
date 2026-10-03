//! Pet bar traffic on `CombatChannel` through the real loopback UDP/native bridge boundary.
use super::*;
use shared::protocol::{
    ACT_COMMAND, ACT_ENABLED, ACT_REACTION, COMMAND_ATTACK, COMMAND_FOLLOW, COMMAND_MOVE_TO,
    COMMAND_STAY, CombatChannel, PetAction, PetClearSpells, PetSpells, REACT_ASSIST,
    REACT_DEFENSIVE, REACT_PASSIVE, pet_action_button,
};

const WOLF: u64 = 0x0000_0002_0000_0031;

/// The server's hunter pet bar: Attack, Follow, Move To, Dash, Bite, Growl, an empty slot,
/// Assist, Defensive, Passive.
fn wolf_bar(command_state: u32, react_state: u32) -> PetSpells {
    PetSpells {
        pet: WOLF,
        command_state,
        react_state,
        action_buttons: [
            pet_action_button(COMMAND_ATTACK, ACT_COMMAND),
            pet_action_button(COMMAND_FOLLOW, ACT_COMMAND),
            pet_action_button(COMMAND_MOVE_TO, ACT_COMMAND),
            pet_action_button(61_684, ACT_ENABLED),
            pet_action_button(17_253, ACT_ENABLED),
            pet_action_button(2_649, ACT_ENABLED),
            0,
            pet_action_button(REACT_ASSIST, ACT_REACTION),
            pet_action_button(REACT_DEFENSIVE, ACT_REACTION),
            pet_action_button(REACT_PASSIVE, ACT_REACTION),
        ],
    }
}

fn await_connected(server: &mut App, bridge: &mut NetworkBridge) {
    let deadline = Instant::now() + Duration::from_secs(10);
    loop {
        assert!(Instant::now() < deadline, "pet connection timed out");
        server.update();
        if bridge
            .drain_events()
            .expect("poll pet connection")
            .iter()
            .any(|event| matches!(event, Event::Connected))
        {
            return;
        }
        thread::sleep(Duration::from_millis(5));
    }
}

/// Dismiss and call again in one worker frame: the bar must end shown, so the clear may
/// not overtake the bar sent before it, nor the new bar the clear.
#[test]
fn native_bridge_receives_pet_bar_messages_in_channel_order() {
    let (mut server, address) = crate::wire_tests::start_fixture_server();
    let mut bridge = NetworkBridge::connect(address, 8251).expect("start pet bridge");
    await_connected(&mut server, &mut bridge);
    let (held, hold_confirmed) = mpsc::channel();
    let (resume, resumed) = mpsc::channel();
    bridge
        .enqueue(move |_| {
            held.send(()).expect("confirm pet worker hold");
            resumed
                .recv_timeout(Duration::from_secs(10))
                .expect("release pet worker hold");
        })
        .expect("hold pet worker");
    hold_confirmed
        .recv_timeout(Duration::from_secs(10))
        .expect("pet worker entered hold");
    let following = wolf_bar(COMMAND_FOLLOW, REACT_ASSIST);
    let staying = wolf_bar(COMMAND_STAY, REACT_DEFENSIVE);
    macro_rules! send_and_flush {
        ($ty:ty, $value:expr) => {{
            let world = server.world_mut();
            world
                .query::<&mut MessageSender<$ty>>()
                .single_mut(world)
                .expect("one connected pet sender")
                .send::<CombatChannel>($value);
            server.update();
        }};
    }
    send_and_flush!(PetSpells, following);
    send_and_flush!(PetClearSpells, PetClearSpells);
    send_and_flush!(PetSpells, staying);
    let flush_deadline = Instant::now() + Duration::from_millis(100);
    while Instant::now() < flush_deadline {
        server.update();
        thread::sleep(Duration::from_millis(5));
    }
    resume.send(()).expect("resume pet worker");

    let deadline = Instant::now() + Duration::from_secs(10);
    let mut messages = Vec::new();
    while messages.len() < 3 && Instant::now() < deadline {
        server.update();
        for event in bridge.drain_events().expect("poll pet messages") {
            match event {
                Event::Message(message) => messages.push(message),
                Event::Disconnected(reason) => panic!("pet disconnected: {reason:?}"),
                Event::ProtocolRejected(reason) => panic!("pet protocol rejected: {reason}"),
                _ => {}
            }
        }
        thread::sleep(Duration::from_millis(5));
    }
    bridge.stop().expect("join pet worker");
    assert_eq!(messages.len(), 3, "all three pet messages delivered");
    let mut received = messages.into_iter();
    assert_eq!(
        received.next().unwrap().downcast::<PetSpells>().ok(),
        Some(following)
    );
    assert_eq!(
        received.next().unwrap().downcast::<PetClearSpells>().ok(),
        Some(PetClearSpells)
    );
    assert_eq!(
        received.next().unwrap().downcast::<PetSpells>().ok(),
        Some(staying)
    );
}

#[derive(Resource, Default)]
struct PetActions(Vec<PetAction>);

fn capture_pet_actions(
    mut receivers: Query<&mut MessageReceiver<PetAction>>,
    mut actions: ResMut<PetActions>,
) {
    for mut receiver in &mut receivers {
        actions.0.extend(receiver.receive());
    }
}

/// `CMSG_PET_ACTION`: Attack carries the owner's target, Move To the ground point.
#[test]
fn native_bridge_sends_pet_actions_on_the_combat_channel() {
    fn install(app: &mut App) {
        app.init_resource::<PetActions>();
        app.add_systems(Update, capture_pet_actions);
    }
    let (mut server, address) = crate::wire_tests::start_fixture_server_with(install);
    let mut bridge = NetworkBridge::connect(address, 8252).expect("start pet bridge");
    await_connected(&mut server, &mut bridge);
    let attack = PetAction {
        pet: WOLF,
        action: pet_action_button(COMMAND_ATTACK, ACT_COMMAND),
        target: Some(0x0000_0002_0000_0044),
        position: None,
    };
    let move_to = PetAction {
        pet: WOLF,
        action: pet_action_button(COMMAND_MOVE_TO, ACT_COMMAND),
        target: None,
        position: Some([-8_913.5, 82.25, -553.0]),
    };
    bridge.send::<_, CombatChannel>(attack).unwrap();
    bridge.send::<_, CombatChannel>(move_to).unwrap();
    let deadline = Instant::now() + Duration::from_secs(10);
    while server.world().resource::<PetActions>().0.len() < 2 && Instant::now() < deadline {
        server.update();
        bridge.drain_events().expect("poll pet bridge");
        thread::sleep(Duration::from_millis(5));
    }
    assert_eq!(
        server.world().resource::<PetActions>().0,
        vec![attack, move_to]
    );
    bridge.stop().expect("join pet worker");
}
