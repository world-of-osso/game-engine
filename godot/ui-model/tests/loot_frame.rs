use game_engine_ui_model::loot_data::LootState;
use game_engine_ui_model::loot_frame_component::{FRAME_NAME, loot_frame_screen};
use game_engine_ui_model::loot_frame_data::{LootFrameAction, build_state, request_for_action};
use shared::protocol::{LootContent, LootResponse, LootSlot};
use ui_toolkit::frame::WidgetData;
use ui_toolkit::registry::FrameRegistry;
use ui_toolkit::screen::{Screen, SharedContext};

fn text(registry: &FrameRegistry, name: &str) -> String {
    let frame = registry.get(registry.get_by_name(name).unwrap()).unwrap();
    match frame.widget_data.as_ref().unwrap() {
        WidgetData::FontString(font) => font.text.clone(),
        _ => panic!("{name} must be a font string"),
    }
}

#[test]
fn server_removal_rebinds_the_remaining_card_and_closure_hides_the_frame() {
    let mut loot = LootState::default();
    loot.open(LootResponse {
        corpse: 42,
        auto: false,
        slots: vec![
            LootSlot {
                slot: 5,
                content: LootContent::Money { copper: 3 },
            },
            LootSlot {
                slot: 9,
                content: LootContent::Money { copper: 120 },
            },
        ],
    });
    let mut registry = FrameRegistry::new(1920.0, 1080.0);
    let mut shared = SharedContext::new();
    let mut screen = Screen::new(loot_frame_screen);
    shared.insert(build_state(&loot, [400.0, 300.0]));
    screen.sync(&shared, &mut registry);
    assert_eq!(text(&registry, "LootFrameElement1Text"), "3 Copper");
    assert_eq!(
        text(&registry, "LootFrameElement2Text"),
        "1 Silver\n20 Copper"
    );
    let close = registry
        .get(registry.get_by_name("LootFrameCloseButton").unwrap())
        .unwrap();
    assert_eq!(
        request_for_action(close.onclick.as_deref().unwrap()),
        Some(LootFrameAction::Release)
    );

    assert_eq!(loot.remove(42, 5), Some(LootContent::Money { copper: 3 }));
    shared.insert(build_state(&loot, [400.0, 300.0]));
    screen.sync(&shared, &mut registry);
    assert_eq!(
        text(&registry, "LootFrameElement1Text"),
        "1 Silver\n20 Copper"
    );
    assert!(registry.get_by_name("LootFrameElement2").is_none());
    let remaining = registry
        .get(registry.get_by_name("LootFrameElement1").unwrap())
        .unwrap();
    assert_eq!(
        request_for_action(remaining.onclick.as_deref().unwrap()),
        Some(LootFrameAction::Take { slot: 9 })
    );
    assert!(
        !registry
            .get(registry.get_by_name(FRAME_NAME).unwrap())
            .unwrap()
            .hidden
    );

    loot.close(42);
    shared.insert(build_state(&loot, [400.0, 300.0]));
    screen.sync(&shared, &mut registry);
    assert!(
        registry
            .get(registry.get_by_name(FRAME_NAME).unwrap())
            .unwrap()
            .hidden
    );
    assert!(registry.get_by_name("LootFrameElement1").is_none());
}
