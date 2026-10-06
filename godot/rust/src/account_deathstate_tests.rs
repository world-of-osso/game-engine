use super::*;
use game_engine_network::deathstate_fixture::{DeathServerFixture, ReceivedDeathRequest};
use game_engine_ui_model::death_flow::DeathFlow;
use game_engine_ui_model::popup::PopupStack;
use game_engine_ui_model::static_popup_component::{
    StaticPopupState, parse_popup_action, static_popup_screen,
};
use shared::protocol::{
    DeathPositionSnapshot, DeathSnapshot, DeathStateSnapshot, DeathStateUpdate,
};
use std::time::{Duration, Instant};
use ui_toolkit::atlas::{ActiveSkin, set_thread_skin};
use ui_toolkit::registry::FrameRegistry;
use ui_toolkit::screen::{Screen, SharedContext};

fn position(x: f32) -> DeathPositionSnapshot {
    DeathPositionSnapshot {
        map_id: 0,
        x,
        y: 3.0,
        z: -7.0,
    }
}

fn snapshot(state: DeathStateSnapshot) -> DeathStateUpdate {
    DeathStateUpdate {
        snapshot: Some(DeathSnapshot {
            state,
            corpse: Some(position(12.0)),
            graveyard: Some(position(200.0)),
            can_resurrect_at_corpse: false,
            spirit_healer_available: false,
        }),
        message: None,
        error: None,
    }
}

fn await_connection(account: &mut Account, server: &mut DeathServerFixture) {
    let deadline = Instant::now() + Duration::from_secs(10);
    while Instant::now() < deadline {
        server.step();
        let events = account.bridge.as_mut().unwrap().drain_events().unwrap();
        if events
            .into_iter()
            .any(|event| matches!(event, Event::Connected))
        {
            return;
        }
        std::thread::sleep(Duration::from_millis(5));
    }
    panic!("death fixture connection timed out");
}

fn await_snapshot(account: &mut Account, server: &mut DeathServerFixture) -> DeathStateUpdate {
    let deadline = Instant::now() + Duration::from_secs(10);
    while Instant::now() < deadline {
        server.step();
        for event in account.poll().unwrap() {
            if let AccountEvent::Death(update) = event {
                return update;
            }
        }
        std::thread::sleep(Duration::from_millis(5));
    }
    panic!("death snapshot did not reach AccountEvent");
}

fn await_request(server: &mut DeathServerFixture) -> Vec<ReceivedDeathRequest> {
    let deadline = Instant::now() + Duration::from_secs(10);
    while Instant::now() < deadline {
        server.step();
        let requests = server.take_requests();
        if !requests.is_empty() {
            return requests;
        }
        std::thread::sleep(Duration::from_millis(5));
    }
    panic!("death popup acceptance did not reach the server");
}

fn click_accept(stack: &mut PopupStack, label: &str) {
    let mut registry = FrameRegistry::new(1920.0, 1080.0);
    let mut shared = SharedContext::new();
    shared.insert(StaticPopupState {
        popups: stack.visible(),
    });
    Screen::new(static_popup_screen).sync(&shared, &mut registry);
    let button = registry
        .get(registry.get_by_name("StaticPopup1Button1").unwrap())
        .unwrap();
    assert!(
        matches!(button.widget_data.as_ref(), Some(ui_toolkit::frame::WidgetData::Button(data)) if data.text == label)
    );
    let (id, outcome) = parse_popup_action(button.onclick.as_deref().unwrap()).unwrap();
    assert!(stack.resolve(id, outcome));
}

#[test]
fn deathstate_udp_snapshot_popup_account_request_both_skins() {
    for (index, skin) in [ActiveSkin::Modern, ActiveSkin::Forever]
        .into_iter()
        .enumerate()
    {
        set_thread_skin(skin);
        let mut server = DeathServerFixture::start();
        let mut account = Account::new(PathBuf::from("/unused-death-data"));
        account.bridge =
            Some(NetworkBridge::connect(server.address(), 38901 + index as u64).unwrap());
        await_connection(&mut account, &mut server);
        let mut flow = DeathFlow::default();
        let mut stack = PopupStack::default();
        let phases = [
            (
                DeathStateSnapshot::Dead,
                12.0,
                "DEATH",
                "Release Spirit",
                ReceivedDeathRequest::Release,
            ),
            (
                DeathStateSnapshot::Ghost,
                15.0,
                "RECOVER_CORPSE",
                "Accept",
                ReceivedDeathRequest::Corpse,
            ),
            (
                DeathStateSnapshot::Ghost,
                203.0,
                "XP_LOSS",
                "Accept",
                ReceivedDeathRequest::SpiritHealer,
            ),
        ];
        for (state, x, key, label, expected) in phases {
            server.send(snapshot(state));
            flow.receive(await_snapshot(&mut account, &mut server));
            if expected == ReceivedDeathRequest::SpiritHealer {
                assert!(flow.request_spirit_healer(&position(x)));
            }
            flow.sync_popups(&mut stack, Some(&position(x)), 60);
            assert!(stack.contains(key));
            click_accept(&mut stack, label);
            let request = flow
                .popup_results(&stack.drain_results())
                .expect("popup sends death action");
            account.send_death(request).unwrap();
            assert_eq!(await_request(&mut server), vec![expected]);
        }
        account.stop().unwrap();
    }
}
