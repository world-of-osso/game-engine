use super::*;
use shared::protocol::{ToyCollectionUpdate, ToyOperation, ToyResult, ToySnapshot};

#[test]
fn toys_wire_dispatch_keeps_catalog_then_result_order() {
    let mut account = Account::new(PathBuf::from("/unused-toys-data"));
    let toy = ToySnapshot {
        item_id: 32782,
        name: "Time-Lost Figurine".into(),
        icon_file_data_id: 134899,
        expansion_id: 1,
        flags: 0,
        source_type: 1,
        source_text: "Terokk".into(),
        spell_id: Some(41301),
        learned: true,
        favourite: true,
        unavailable_reason: None,
    };
    let catalog = ToyCollectionUpdate { toys: vec![toy] };
    let result = ToyResult {
        item_id: 32782,
        operation: ToyOperation::Use,
        spell_id: Some(41301),
        error: Some("Item is not ready yet.".into()),
    };
    let mut events = vec![];
    account
        .dispatch_message(ProtocolMessage::for_tests(catalog.clone()), &mut events)
        .unwrap();
    account
        .dispatch_message(ProtocolMessage::for_tests(result.clone()), &mut events)
        .unwrap();
    assert!(matches!(&events[0], AccountEvent::Toys(update) if update == &catalog));
    assert!(matches!(&events[1], AccountEvent::ToyResult(update) if update == &result));
    assert_eq!(events.len(), 2);
}
