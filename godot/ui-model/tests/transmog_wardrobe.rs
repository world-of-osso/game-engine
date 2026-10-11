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
                    icon_fdid: 135274,
                    name: "Deadman Blade".into(),
                    inventory_type: 13,
                },
            ],
        }],
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
    shared.insert(TransmogView {
        book,
        viewport_width: 1920.0,
        race_id: 1,
    });
    let mut registry = FrameRegistry::new(1920.0, 1080.0);
    Screen::new(transmog_screen).sync(&shared, &mut registry);
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
        ] {
            assert!(
                registry.get_by_name(name).is_some(),
                "{skin:?}: missing {name}"
            );
        }
        assert!(registry.get_by_name("TransmogSource821Icon").is_none());
        assert_eq!(text(&registry, "TransmogSource821Status"), "Applied");
        assert_eq!(text(&registry, "TransmogSource1181Status"), "Collected");
        let mut pending = book();
        pending.stage(TransmogChoice::Source(1181));
        let registry = build(skin, pending);
        assert_eq!(text(&registry, "TransmogSource1181Status"), "Pending");
        assert_eq!(
            text(&registry, "TransmogPendingChanges"),
            "1 pending changes"
        );
    }
}
