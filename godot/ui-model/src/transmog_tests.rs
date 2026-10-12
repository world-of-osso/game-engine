use super::*;
use shared::item_data::ItemDefinitionSource;

fn state() -> TransmogState {
    TransmogState {
        npc: 54442,
        slots: vec![TransmogSlot {
            slot: EquipmentSlot::MainHand,
            item_guid: 91001,
            item_id: 2850,
            definition_source: ItemDefinitionSource::Retail,
            inventory_type: 13,
            applied: TransmogChoice::Original,
            cost: Some(1500),
            can_hide: false,
            sources: vec![TransmogSource {
                source_id: 821,
                source_type: 3,
                appearance_id: 154,
                item_id: 2488,
                display_id: 1542,
                icon_fdid: 135274,
                name: "Gladius".into(),
                inventory_type: 13,
            }],
        }],
        ..Default::default()
    }
}

#[test]
fn transmog_wardrobe_pages_have_twenty_models_and_clamp_after_collection_changes() {
    let mut collection = state();
    let source = collection.slots[0].sources[0].clone();
    collection.slots[0].sources = (0..41)
        .map(|index| TransmogSource {
            source_id: 821 + index,
            name: format!("Gladius {index}"),
            ..source.clone()
        })
        .collect();
    let mut book = TransmogBook::default();
    book.receive(collection);
    assert_eq!(book.pages(), 3);
    assert_eq!(book.page_sources().len(), 20);
    book.page = 1;
    assert_eq!(book.page_sources()[0].source_id, 841);
    book.page = 2;
    assert_eq!(book.page_sources().len(), 1);
    book.receive(state());
    assert_eq!(book.page, 0);
    assert_eq!(book.page_sources()[0].source_id, 821);
}

#[test]
fn transmog_collection_preview_uses_the_source_display_without_staging_or_world_mutation() {
    let mut book = TransmogBook::default();
    let mut applied = state();
    applied.slots[0].applied = TransmogChoice::Source(821);
    book.receive(applied);
    let before = book.clone();
    let preview = book.collection_equipment(821).expect("collected source");
    assert_eq!(preview.entries.len(), 1);
    let weapon = &preview.entries[0];
    assert_eq!(
        weapon.slot,
        shared::components::EquipmentVisualSlot::MainHand
    );
    assert_eq!(weapon.display_info_id, Some(1542));
    assert_eq!(weapon.item_id, Some(2850));
    assert_eq!(weapon.definition_source, Some(ItemDefinitionSource::Retail));
    assert_eq!(weapon.inventory_type, 13);
    assert!(!weapon.hidden);
    assert_eq!(book, before);
    assert!(book.collection_equipment(730).is_none());
}

#[test]
fn transmog_search_filters_names_without_losing_hidden_pending_choices() {
    let mut collection = state();
    collection.slots[0].sources.push(TransmogSource {
        source_id: 1181,
        source_type: 4,
        appearance_id: 389,
        item_id: 3295,
        display_id: 1546,
        icon_fdid: 135274,
        name: "Deadman Blade".into(),
        inventory_type: 13,
    });
    let mut book = TransmogBook::default();
    book.receive(collection);
    book.money = 5000;
    assert!(book.stage(TransmogChoice::Source(1181)));
    book.set_search(" gLaDiUs ".into());
    assert_eq!(
        book.page_sources()
            .iter()
            .map(|source| source.source_id)
            .collect::<Vec<_>>(),
        vec![821]
    );
    assert_eq!(book.pending_cost(), Some(1500));
    book.set_search("not collected".into());
    assert!(book.page_sources().is_empty());
    assert_eq!(book.pages(), 1);
    assert!(book.can_apply());
    assert!(book.collection_equipment(1181).is_some());
    book.set_search(String::new());
    assert_eq!(book.page_sources().len(), 2);
    assert_eq!(
        book.apply().unwrap().changes[0].choice,
        TransmogChoice::Source(1181)
    );
}

#[test]
fn transmog_pending_choice_does_not_change_applied_state_and_apply_quotes_exact_cost() {
    let mut book = TransmogBook::default();
    book.receive(state());
    book.money = 1500;
    assert!(book.stage(TransmogChoice::Source(821)));
    assert_eq!(
        book.state.as_ref().unwrap().slots[0].applied,
        TransmogChoice::Original
    );
    assert_eq!(book.pending_cost(), Some(1500));
    let request = book.apply().expect("valid paid pending appearance");
    assert_eq!(request.npc, 54442);
    assert_eq!(
        request.changes,
        vec![TransmogChange {
            slot: EquipmentSlot::MainHand,
            item_guid: 91001,
            choice: TransmogChoice::Source(821)
        }]
    );
    assert!(book.apply().is_none(), "double apply while awaiting server");
    let mut applied = state();
    applied.slots[0].applied = TransmogChoice::Source(821);
    book.receive(applied);
    assert_eq!(book.pending_cost(), Some(0));
    assert!(book.pending.is_empty());
}

#[test]
fn transmog_insufficient_or_unknown_cost_disables_apply_but_not_free_restore() {
    let mut book = TransmogBook::default();
    book.receive(state());
    book.money = 1499;
    assert!(book.stage(TransmogChoice::Source(821)));
    assert!(!book.can_apply());
    let mut unknown = state();
    unknown.slots[0].cost = None;
    book.receive(unknown);
    assert_eq!(book.pending_cost(), None);
    assert!(book.apply().is_none());
    let mut applied = state();
    applied.slots[0].applied = TransmogChoice::Source(821);
    applied.slots[0].cost = None;
    book.receive(applied);
    assert!(book.stage(TransmogChoice::Original));
    book.money = 0;
    assert_eq!(book.pending_cost(), Some(0));
    assert!(book.can_apply());
}

#[test]
fn transmog_unoffered_sources_and_weapon_hide_are_not_staged() {
    let mut book = TransmogBook::default();
    book.receive(state());
    assert!(!book.stage(TransmogChoice::Source(730)));
    assert!(!book.stage(TransmogChoice::Hidden));
    assert!(book.pending.is_empty());
}

#[test]
fn transmog_replacement_guid_cancels_pending_and_server_refusal_unblocks_editor() {
    let mut book = TransmogBook::default();
    book.receive(state());
    book.money = 5000;
    book.stage(TransmogChoice::Source(821));
    book.apply().unwrap();
    book.receive_failure(TransmogFailed {
        npc: 54442,
        error: TransmogError::NotEnoughMoney,
    });
    assert!(!book.applying);
    assert!(!book.error.is_empty());
    let mut replacement = state();
    replacement.slots[0].item_guid = 91002;
    book.receive(replacement);
    assert!(book.pending.is_empty());
    assert!(!book.error.is_empty());
    book.close();
    assert!(book.state.is_none());
}
