use game_engine_ui_model::loot_data::LootState;
use game_engine_ui_model::loot_frame_component::{
    FRAME_NAME, LootFrameRow, LootFrameState, loot_frame_screen,
};
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
fn loot_item_hit_area_preserves_source_geometry_and_slot_actions_both_skins() {
    use ui_toolkit::atlas::{ActiveSkin, set_thread_skin};
    use ui_toolkit::layout_values::Val;

    game_engine_ui_model::paths::set_data_root(
        std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../data"),
    )
    .unwrap();
    for skin in [ActiveSkin::Modern, ActiveSkin::Forever] {
        set_thread_skin(skin);
        let mut shared = SharedContext::new();
        shared.insert(LootFrameState {
            visible: true,
            rows: vec![
                LootFrameRow {
                    slot: 5,
                    icon_fdid: 133788,
                    name: "3 Copper".into(),
                    color: "1,1,1,1",
                    count: 1,
                    quality_text: None,
                },
                LootFrameRow {
                    slot: 9,
                    icon_fdid: 132350,
                    name: "Sharp Shortsword".into(),
                    color: "0.12,1,0,1",
                    count: 2,
                    quality_text: Some("Uncommon"),
                },
            ],
            left: 400.,
            top: 300.,
        });
        let mut registry = FrameRegistry::new(1920., 1080.);
        Screen::new(loot_frame_screen).sync(&shared, &mut registry);
        for (index, slot) in [(1, 5), (2, 9)] {
            let row_name = format!("LootFrameElement{index}");
            let row = registry
                .get(registry.get_by_name(&row_name).unwrap())
                .unwrap();
            assert_eq!(row.onclick, None, "{skin:?}: row gutters cannot loot");
            assert!(!row.mouse_enabled);
            let item_name = format!("{row_name}Item");
            let item = registry
                .get(registry.get_by_name(&item_name).unwrap())
                .unwrap();
            assert_eq!((item.width.value(), item.height.value()), (37., 37.));
            assert_eq!(
                (item.position.left, item.position.top),
                (Val::Px(5.), Val::Px(4.))
            );
            assert_eq!(item.hit_rect_insets, [0., -157., 0., 0.]);
            assert!(item.mouse_enabled);
            assert_eq!(
                request_for_action(item.onclick.as_deref().unwrap()),
                Some(LootFrameAction::Take { slot })
            );
            let Some(WidgetData::Button(button)) = &item.widget_data else {
                panic!("{item_name} must dispatch on release, not row press");
            };
            assert!(
                !button.use_default_skin,
                "Input must not add replacement paint"
            );
            assert!(button.normal_texture.is_none() && button.pushed_texture.is_none());
            assert!(button.highlight_texture.is_none() && button.disabled_texture.is_none());
        }
    }
    set_thread_skin(ActiveSkin::Modern);
}

#[test]
fn server_removal_rebinds_the_remaining_card_and_closure_hides_the_frame() {
    // The close button resolves `RedButton-Exit` through the atlas tables.
    game_engine_ui_model::paths::set_data_root(
        std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../data"),
    )
    .unwrap();
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
        .get(registry.get_by_name("LootFrameElement1Item").unwrap())
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
