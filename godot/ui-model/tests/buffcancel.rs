use game_engine_ui_model::aura_display_data::{AuraInstance, DebuffType};
use game_engine_ui_model::buff_frame_component::{
    BuffFrameState, buff_button_at, buff_frame_screen, player_buff_cancel,
};
use game_engine_ui_model::game_tooltip::spell::aura_tooltip;
use ui_toolkit::registry::FrameRegistry;
use ui_toolkit::screen::{Screen, SharedContext};

fn intellect(debuff: bool) -> AuraInstance {
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

fn mounted(auras: &[AuraInstance]) -> FrameRegistry {
    let mut shared = SharedContext::new();
    shared.insert(ui_toolkit::atlas::ActiveSkin::Modern);
    shared.insert(BuffFrameState::from_auras(auras, false));
    let mut registry = FrameRegistry::new(1920.0, 1080.0);
    Screen::new(buff_frame_screen).sync(&shared, &mut registry);
    registry
}

#[test]
fn buffcancel_hover_concrete_aura_shows_retail_content() {
    game_engine_ui_model::paths::set_data_root(
        std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../data"),
    )
    .unwrap();
    for debuff in [false, true] {
        let aura = intellect(debuff);
        let registry = mounted(std::slice::from_ref(&aura));
        let name = if debuff {
            "DebuffButton0Icon"
        } else {
            "BuffButton0Icon"
        };
        let hit = registry.get_by_name(name).unwrap();
        let (harmful, index) = buff_button_at(&registry, hit).unwrap();
        let shown = [aura];
        let selected = shown
            .iter()
            .filter(|a| a.is_debuff == harmful)
            .nth(index)
            .unwrap();
        let tooltip = aura_tooltip(selected);
        assert_eq!(tooltip.content.title, "Arcane Intellect");
        let lines: Vec<_> = tooltip
            .content
            .lines
            .iter()
            .map(|line| line.left_text.as_str())
            .collect();
        assert_eq!(
            lines,
            ["Increases Intellect by 5%.", "60 minutes remaining"]
        );
    }
}

#[test]
fn buffcancel_buff_click_resolves_one_aura_action() {
    let mut registry = mounted(&[intellect(false)]);
    let id = registry.get_by_name("BuffButton0").unwrap();
    let action = registry.click_frame(id).unwrap();
    let requests: Vec<_> = player_buff_cancel(&action, &[intellect(true), intellect(false)])
        .into_iter()
        .collect();
    assert_eq!(requests, [shared::protocol::CancelAura { spell_id: 1459 }]);
}

#[test]
fn buffcancel_debuff_click_has_no_cancel_action() {
    let mut registry = mounted(&[intellect(true)]);
    let id = registry.get_by_name("DebuffButton0").unwrap();
    let action = registry.click_frame(id).unwrap_or_default();
    let requests: Vec<_> = player_buff_cancel(&action, &[intellect(true)])
        .into_iter()
        .collect();
    assert!(requests.is_empty());
    assert!(player_buff_cancel("DebuffButton0", &[intellect(true)]).is_none());
}
