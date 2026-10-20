use game_engine_ui_model::bag_data::InventoryState;
use shared::protocol::InventorySnapshot;

#[test]
#[ignore = "RED: engine owner inventory discards server crafting origin"]
fn crafting_origin_owner_snapshot_changes_only_instance_provenance() {
    let item = serde_json::json!({
        "item_guid": 9001,
        "item_id": 2303,
        "definition_source": "Retail",
        "count": 1,
        "durability": null,
        "soulbound": false,
        "crafting_origin": null
    });
    let mut wire =
        serde_json::json!({"bags": [{"bag": 0, "size": 16, "items": [{"slot": 0, "item": item}]}]});
    let unknown: InventorySnapshot = serde_json::from_value(wire.clone()).unwrap();
    let mut state = InventoryState::default();
    state.apply_snapshot(&unknown);
    let before = state.slots[0][0].clone();
    wire["bags"][0]["items"][0]["item"]["crafting_origin"] = serde_json::json!({
        "skill_line_ability_id": 1355,
        "recipe_spell_id": 2153,
        "crafting_quality_id": null
    });
    let crafted: InventorySnapshot = serde_json::from_value(wire).unwrap();
    state.apply_snapshot(&crafted);
    let after = &state.slots[0][0];
    assert_ne!(
        after, &before,
        "engine must mirror changed authoritative origin on the same GUID"
    );
    assert_eq!(after.item_guid, before.item_guid);
    assert_eq!(after.item_id, before.item_id);
    assert_eq!(after.count, before.count);
    assert_eq!(after.quality, before.quality);
    assert_eq!(after.durability, before.durability);
    assert_eq!(after.soulbound, before.soulbound);
    state.apply_snapshot(&unknown);
    assert_eq!(
        state.slots[0][0], before,
        "unknown lineage must not be inferred or retained from another snapshot"
    );
}
