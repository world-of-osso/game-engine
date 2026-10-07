//! Native aura input classification, mounted frame action, and emitted protocol request.
use game_engine_ui_model::aura_display_data::{AuraInstance, DebuffType};
use game_engine_ui_model::buff_frame_component::{
    BuffFrameState, buff_frame_screen, player_buff_cancel,
};
use godot::global::MouseButton;
use shared::protocol::CancelAura;
use ui_toolkit::atlas::ActiveSkin;
use ui_toolkit::registry::FrameRegistry;
use ui_toolkit::screen::{Screen, SharedContext};

use super::input_queue::PendingInputs;
use super::projection::{UiInput, buff_cancel_input};

fn aura(debuff: bool) -> AuraInstance {
    AuraInstance {
        instance_id: 18,
        spell_id: 1459,
        name: "Arcane Intellect".into(),
        description: "Increases Intellect by 5%.".into(),
        icon_fdid: 135932,
        source: "Buffcancel".into(),
        from_local_player: true,
        from_player: true,
        duration: 3600.0,
        remaining: 3542.0,
        stacks: 1,
        is_debuff: debuff,
        debuff_type: DebuffType::None,
    }
}

fn click_requests(debuff: bool, button: MouseButton) -> Vec<CancelAura> {
    let auras = [aura(debuff)];
    let mut shared = SharedContext::new();
    shared.insert(ActiveSkin::Modern);
    shared.insert(BuffFrameState::from_auras(&auras, false));
    let mut registry = FrameRegistry::new(1920.0, 1080.0);
    Screen::new(buff_frame_screen).sync(&shared, &mut registry);
    let name = if debuff {
        "DebuffButton0"
    } else {
        "BuffButton0"
    };
    let id = registry.get_by_name(name).unwrap();
    let pending = PendingInputs::default();
    if let Some(input) = buff_cancel_input(id, button, true, false) {
        pending.push(input);
    }
    assert!(pending.drain().is_empty(), "press must not emit a cancel");
    if let Some(input) = buff_cancel_input(id, button, false, false) {
        pending.push(input);
    }
    let requests = pending
        .drain()
        .into_iter()
        .filter_map(|(_, input)| {
            let UiInput::AltClick {
                id, right: true, ..
            } = input
            else {
                return None;
            };
            let action = registry.click_frame(id)?;
            player_buff_cancel(&action, &auras)
        })
        .collect();
    assert!(pending.drain().is_empty(), "input consumed exactly once");
    requests
}

#[test]
fn buffcancel_native_right_release_emits_exactly_one_correct_cancel() {
    assert_eq!(
        click_requests(false, MouseButton::RIGHT),
        [CancelAura { spell_id: 1459 }]
    );
}

#[test]
fn buffcancel_native_debuff_right_click_emits_nothing() {
    assert!(click_requests(true, MouseButton::RIGHT).is_empty());
}

#[test]
fn buffcancel_native_left_click_emits_nothing() {
    assert!(click_requests(false, MouseButton::LEFT).is_empty());
}
