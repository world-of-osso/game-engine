//! Real UDP → Account → native StaticPopup registry → wire response proof.
use super::*;
use crate::meeting_stones::meeting_stone_request;
use game_engine_network::meetingstones_fixture::{MeetingStoneRequest, MeetingStoneServerFixture};
use game_engine_ui_model::static_popup_component::{
    StaticPopupState, parse_popup_action, static_popup_screen,
};
use game_engine_ui_model::{
    popup::{PopupOutcome, PopupStack},
    summon::SummonPopup,
};
use shared::protocol::{
    GAMEOBJECT_TYPE_MEETINGSTONE, GAMEOBJECT_TYPE_RITUAL, GameObjectInfo, SummonRequest,
    SummonResponse, UseGameObject,
};
use std::time::{Duration, Instant};
use ui_toolkit::{
    atlas::{ActiveSkin, set_thread_skin},
    registry::FrameRegistry,
    screen::{Screen, SharedContext},
};

fn await_connected(account: &mut Account, server: &mut MeetingStoneServerFixture) {
    let deadline = Instant::now() + Duration::from_secs(10);
    while Instant::now() < deadline {
        server.step();
        if account
            .bridge
            .as_mut()
            .unwrap()
            .drain_events()
            .unwrap()
            .into_iter()
            .any(|e| matches!(e, Event::Connected))
        {
            return;
        }
        std::thread::sleep(Duration::from_millis(5));
    }
    panic!("meeting-stone fixture connection timed out");
}

fn await_offer(account: &mut Account, server: &mut MeetingStoneServerFixture) -> SummonRequest {
    let deadline = Instant::now() + Duration::from_secs(10);
    while Instant::now() < deadline {
        server.step();
        for event in account.poll().unwrap() {
            if let AccountEvent::Summon(offer) = event {
                return offer;
            }
        }
        std::thread::sleep(Duration::from_millis(5));
    }
    panic!("summon offer did not reach Account");
}

fn await_request(server: &mut MeetingStoneServerFixture) -> Vec<MeetingStoneRequest> {
    let deadline = Instant::now() + Duration::from_secs(10);
    while Instant::now() < deadline {
        server.step();
        let requests = server.take_requests();
        if !requests.is_empty() {
            return requests;
        }
        std::thread::sleep(Duration::from_millis(5));
    }
    panic!("meeting-stone request did not reach server");
}

fn popup_registry(stack: &PopupStack) -> FrameRegistry {
    let mut registry = FrameRegistry::new(1920.0, 1080.0);
    let mut shared = SharedContext::new();
    shared.insert(StaticPopupState {
        popups: stack.visible(),
    });
    Screen::new(static_popup_screen).sync(&shared, &mut registry);
    registry
}

fn click_popup(stack: &mut PopupStack, button: &str, expected: &str) {
    let registry = popup_registry(stack);
    let frame = registry.get(registry.get_by_name(button).unwrap()).unwrap();
    assert!(
        matches!(frame.widget_data.as_ref(), Some(ui_toolkit::frame::WidgetData::Button(data)) if data.text == expected)
    );
    let (id, outcome) = parse_popup_action(frame.onclick.as_deref().unwrap()).unwrap();
    assert!(stack.resolve(id, outcome));
}

#[test]
fn meetingstones_udp_registry_replacement_combat_answers_and_stone_use_both_skins() {
    for (index, skin) in [ActiveSkin::Modern, ActiveSkin::Forever]
        .into_iter()
        .enumerate()
    {
        set_thread_skin(skin);
        let mut server = MeetingStoneServerFixture::start();
        let mut account = Account::new(PathBuf::from("/unused-meetingstone-data"));
        account.bridge =
            Some(NetworkBridge::connect(server.address(), 49301 + index as u64).unwrap());
        await_connected(&mut account, &mut server);
        for kind in [GAMEOBJECT_TYPE_MEETINGSTONE, GAMEOBJECT_TYPE_RITUAL] {
            let info = GameObjectInfo {
                entry: 178834,
                go_type: kind,
                display_id: 5492,
                name: "Meeting Stone".into(),
                scale: 1.0,
            };
            let request = meeting_stone_request(517, &info).unwrap();
            account.send_use_game_object(request.object).unwrap();
            assert_eq!(
                await_request(&mut server),
                vec![MeetingStoneRequest::Use(UseGameObject { object: 517 })]
            );
        }
        let mut flow = SummonPopup::default();
        let mut stack = PopupStack::default();
        for (name, combat) in [("Stonecaller", false), ("Ritebearer", true)] {
            server.send(SummonRequest {
                summoner: name.into(),
                zone_id: 0,
                time_left_ms: 120_000,
            });
            flow.receive(await_offer(&mut account, &mut server), &mut stack, combat);
        }
        assert_eq!(stack.visible().len(), 1);
        assert!(
            stack.visible()[0]
                .spec
                .text
                .starts_with("Ritebearer wants to summon you")
        );
        let registry = popup_registry(&stack);
        let button = registry
            .get(registry.get_by_name("StaticPopup1Button1").unwrap())
            .unwrap();
        assert_eq!(button.onclick.as_deref(), Some(""));
        stack.accept_top();
        assert!(flow.popup_results(&stack.drain_results()).is_none());
        flow.update(Duration::from_secs(61), &mut stack, false);
        assert!(stack.visible()[0].spec.text.ends_with("59 Seconds."));
        click_popup(&mut stack, "StaticPopup1Button1", "Accept");
        account
            .send_summon_response(flow.popup_results(&stack.drain_results()).unwrap())
            .unwrap();
        assert_eq!(
            await_request(&mut server),
            vec![MeetingStoneRequest::Answer(SummonResponse { accept: true })]
        );
        for expired in [false, true] {
            server.send(SummonRequest {
                summoner: "Stonecaller".into(),
                zone_id: 0,
                time_left_ms: 1_000,
            });
            flow.receive(await_offer(&mut account, &mut server), &mut stack, false);
            if expired {
                flow.update(Duration::from_secs(1), &mut stack, false);
            } else {
                click_popup(&mut stack, "StaticPopup1Button2", "Cancel");
            }
            account
                .send_summon_response(flow.popup_results(&stack.drain_results()).unwrap())
                .unwrap();
            assert_eq!(
                await_request(&mut server),
                vec![MeetingStoneRequest::Answer(SummonResponse {
                    accept: false
                })]
            );
        }
        account.stop().unwrap();
    }
}

#[test]
fn meetingstones_cast_refusals_reach_native_account() {
    use shared::spell_data::CastFailReason;
    let mut server = MeetingStoneServerFixture::start();
    let mut account = Account::new(PathBuf::from("/unused-meetingstone-data"));
    account.bridge = Some(NetworkBridge::connect(server.address(), 49310).unwrap());
    await_connected(&mut account, &mut server);
    for (reason, text) in [
        (CastFailReason::InvalidTarget, "Invalid target"),
        (
            CastFailReason::LevelRequirement,
            "You are not high enough level",
        ),
        (CastFailReason::TargetTooLowLevel, "Target is too low level"),
        (CastFailReason::SummonPending, "A summon is already pending"),
    ] {
        server.send_refusal(CastFailed {
            spell_id: 59782,
            reason,
            detail: None,
        });
        let deadline = Instant::now() + Duration::from_secs(10);
        let refusal = loop {
            assert!(Instant::now() < deadline, "cast refusal timed out");
            server.step();
            if let Some(refusal) = account.poll().unwrap().into_iter().find_map(|e| {
                if let AccountEvent::CastFailed(f) = e {
                    Some(f)
                } else {
                    None
                }
            }) {
                break refusal;
            }
            std::thread::sleep(Duration::from_millis(5));
        };
        assert_eq!(
            game_engine_ui_model::cast_failed_text::cast_failed_text(
                refusal.reason,
                refusal.detail.as_deref(),
                None
            ),
            text
        );
    }
    account.stop().unwrap();
}

#[test]
fn meetingstones_stale_replacement_click_cannot_answer_new_request() {
    let mut flow = SummonPopup::default();
    let mut stack = PopupStack::default();
    let request = SummonRequest {
        summoner: "Stonecaller".into(),
        zone_id: 0,
        time_left_ms: 120_000,
    };
    flow.receive(request.clone(), &mut stack, false);
    let old = stack.visible()[0].id;
    stack.resolve(old, PopupOutcome::Accepted);
    flow.receive(request, &mut stack, true);
    assert!(flow.popup_results(&stack.drain_results()).is_none());
    assert!(!stack.resolve(old, PopupOutcome::Accepted));
    assert!(stack.is_open());
}
