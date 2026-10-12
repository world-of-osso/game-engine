//! Shared-skin Wardrobe contract: Retail collection model cells and paperdoll slots.
use game_engine_ui_model::{
    transmog::TransmogBook,
    transmog_frame::{TransmogView, transmog_screen},
};
use shared::{item_data::ItemDefinitionSource, protocol::*};
use ui_toolkit::{
    atlas::{ActiveSkin, set_thread_skin},
    frame::WidgetData,
    registry::FrameRegistry,
    screen::{Screen, SharedContext},
};

fn book() -> TransmogBook {
    let mut book = TransmogBook::default();
    book.receive(TransmogState {
        npc: 54442,
        slots: vec![TransmogSlot {
            slot: EquipmentSlot::MainHand,
            item_guid: 91001,
            item_id: 2850,
            definition_source: ItemDefinitionSource::Retail,
            inventory_type: 13,
            applied: TransmogChoice::Source(821),
            cost: Some(1500),
            can_hide: false,
            sources: vec![
                TransmogSource {
                    source_id: 821,
                    appearance_id: 154,
                    item_id: 2488,
                    display_id: 1542,
                    icon_fdid: 135274,
                    name: "Gladius".into(),
                    inventory_type: 13,
                },
                TransmogSource {
                    source_id: 1181,
                    appearance_id: 389,
                    item_id: 3295,
                    display_id: 1546,
                    icon_fdid: 135343,
                    name: "Deadman Blade".into(),
                    inventory_type: 13,
                },
            ],
        }],
        ..Default::default()
    });
    book.money = 5000;
    book
}

fn build(skin: ActiveSkin, book: TransmogBook) -> FrameRegistry {
    game_engine_ui_model::paths::set_data_root(
        std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../data"),
    )
    .unwrap();
    set_thread_skin(skin);
    let mut shared = SharedContext::new();
    shared.insert(skin);
    let view = TransmogView {
        book,
        viewport_width: 1920.0,
        race_id: 1,
    };
    shared.insert(view.clone());
    let mut registry = FrameRegistry::new(1920.0, 1080.0);
    Screen::new(transmog_screen).sync(&shared, &mut registry);
    game_engine_ui_model::transmog_frame::apply_transmog_art(&view, &mut registry);
    registry
}

fn text(registry: &FrameRegistry, name: &str) -> String {
    let frame = registry
        .get(registry.get_by_name(name).expect(name))
        .unwrap();
    let Some(WidgetData::FontString(text)) = &frame.widget_data else {
        panic!("{name}")
    };
    text.text.clone()
}

#[test]
fn transmog_retail_frame_and_cells_use_121_natural_dimensions() {
    for skin in [ActiveSkin::Modern, ActiveSkin::Forever] {
        let registry = build(skin, book());
        let frame = registry
            .get(registry.get_by_name("TransmogFrame").unwrap())
            .unwrap();
        assert_eq!(
            (frame.resolved_width(), frame.resolved_height()),
            (1618.0, 883.0)
        );
        let model = registry
            .get(registry.get_by_name("TransmogSource821ModelScene").unwrap())
            .unwrap();
        assert_eq!(
            (model.resolved_width(), model.resolved_height()),
            (100.0, 132.0)
        );
    }
}

#[test]
fn transmog_slot_icon_tracks_authoritative_applied_and_pending_source() {
    for skin in [ActiveSkin::Modern, ActiveSkin::Forever] {
        let mut state = book();
        for (choice, expected_icon, expected_cost) in [
            (TransmogChoice::Source(821), 135274, 0),
            (TransmogChoice::Source(1181), 135343, 1500),
        ] {
            state.stage(choice);
            assert_eq!(state.pending_cost(), Some(expected_cost));
            let registry = build(skin, state.clone());
            let frame = registry
                .get(
                    registry
                        .get_by_name("TransmogSlot16Icon")
                        .expect("current appearance icon"),
                )
                .unwrap();
            let Some(WidgetData::Texture(texture)) = &frame.widget_data else {
                panic!("slot icon")
            };
            assert_eq!(
                texture.source,
                ui_toolkit::widgets::texture::TextureSource::FileDataId(expected_icon)
            );
        }
    }
}

#[test]
fn transmog_retail_grid_pages_twenty_models_without_losing_pending_cost() {
    let mut state = book();
    let slot = &mut state.state.as_mut().unwrap().slots[0];
    let source = slot.sources[0].clone();
    slot.sources = (0..41)
        .map(|index| TransmogSource {
            source_id: 821 + index,
            name: format!("Blade {index}"),
            ..source.clone()
        })
        .collect();
    assert_eq!(state.pages(), 3);
    assert_eq!(state.page_sources().len(), 20);
    state.page = 1;
    assert_eq!(state.page_sources()[0].source_id, 841);
    assert!(state.stage(TransmogChoice::Source(841)));
    state.page = 2;
    assert_eq!(state.page_sources().len(), 1);
    assert_eq!(state.pending_cost(), Some(1500));
    assert!(state.can_apply());
}

fn assert_retail_alpha(registry: &FrameRegistry, name: &str) {
    let frame = registry
        .get(registry.get_by_name(name).expect(name))
        .unwrap();
    let Some(WidgetData::Texture(texture)) = &frame.widget_data else {
        panic!("{name}")
    };
    assert_eq!(
        texture.blend_mode,
        ui_toolkit::widgets::texture::BlendMode::AlphaKey,
        "{name}: Retail12.1 authored StateTexture uses ordinary alpha"
    );
}

#[test]
fn transmog_wardrobe_exposes_model_cells_fixed_slots_and_outfit_gap_in_both_skins() {
    for skin in [ActiveSkin::Modern, ActiveSkin::Forever] {
        let registry = build(skin, book());
        for name in [
            "TransmogSlot1",
            "TransmogSlot16",
            "TransmogSlot17",
            "TransmogSource821ModelScene",
            "TransmogSource1181ModelScene",
            "TransmogOutfitDropdown",
            "TransmogSaveOutfitButton",
            "TransmogRotateLeft",
            "TransmogRotateRight",
            "TransmogSheatheButton",
            "TransmogRaceBackground",
            "TransmogSearchBox",
            "TransmogItemsTab",
            "TransmogSetsTab",
            "TransmogCustomSetsTab",
            "TransmogIllusion16",
            "TransmogIllusion17",
        ] {
            assert!(
                registry.get_by_name(name).is_some(),
                "{skin:?}: missing {name}"
            );
        }
        assert!(registry.get_by_name("TransmogSource821Icon").is_none());
        assert_eq!(text(&registry, "TransmogSource821Status"), "Applied");
        assert_retail_alpha(&registry, "TransmogSource821Applied");
        assert_eq!(text(&registry, "TransmogSource1181Status"), "Collected");
        let mut pending = book();
        pending.stage(TransmogChoice::Source(1181));
        let registry = build(skin, pending);
        assert_eq!(text(&registry, "TransmogSource1181Status"), "Pending");
        assert_retail_alpha(&registry, "TransmogSource1181Pending");
        assert_eq!(
            text(&registry, "TransmogPendingChanges"),
            "1 pending changes"
        );
    }
}
