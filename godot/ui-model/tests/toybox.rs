use game_engine_ui_model::toybox::{PAGE_SIZE, ToyAction, ToyBox, ToyFilters};
use shared::protocol::{ActionRef, SpellCooldownUpdate, ToySnapshot};

fn toy(id: u32, name: &str, learned: bool) -> ToySnapshot {
    ToySnapshot {
        item_id: id,
        name: name.into(),
        icon_file_data_id: 134400,
        expansion_id: (id % 3) as i32,
        flags: 0,
        source_type: (id % 2) as i32,
        source_text: "Quest reward".into(),
        spell_id: Some(id + 1000),
        learned,
        favourite: false,
        unavailable_reason: None,
    }
}

#[test]
fn paging_has_eighteen_and_a_real_second_page() {
    let mut model = ToyBox::default();
    model.receive(
        (1..=40)
            .map(|id| toy(id, &format!("Toy {id:02}"), true))
            .collect(),
    );
    assert_eq!(PAGE_SIZE, 18);
    assert_eq!(model.page_count(), 3);
    assert_eq!(model.page_items().len(), 18);
    model.turn_page(1);
    assert_eq!(model.page_items()[0].item_id, 19);
    model.turn_page(1);
    assert_eq!(model.page_items().len(), 4);
    model.turn_page(1);
    assert_eq!(model.page, 2);
}

#[test]
fn filters_search_and_favourite_sort_are_catalog_backed() {
    let mut model = ToyBox::default();
    let mut favourite = toy(3, "Zebra", true);
    favourite.favourite = true;
    let mut blocked = toy(4, "Azure blocked", true);
    blocked.unavailable_reason = Some("Unsupported effect".into());
    model.receive(vec![
        toy(1, "Azure", true),
        toy(2, "Bear", false),
        favourite,
        blocked,
    ]);
    assert_eq!(model.filtered()[0].item_id, 3);
    model.filters = ToyFilters {
        collected: false,
        ..Default::default()
    };
    assert_eq!(
        model
            .filtered()
            .iter()
            .map(|t| t.item_id)
            .collect::<Vec<_>>(),
        vec![2]
    );
    model.filters = ToyFilters {
        usable_only: true,
        search: "AZURE".into(),
        ..Default::default()
    };
    assert_eq!(
        model
            .filtered()
            .iter()
            .map(|t| t.item_id)
            .collect::<Vec<_>>(),
        vec![1]
    );
    model.filters.sources = Some([0].into());
    assert!(model.filtered().is_empty());
    model.filters = ToyFilters {
        expansions: Some([2].into()),
        ..Default::default()
    };
    assert_eq!(
        model
            .filtered()
            .iter()
            .map(|t| t.item_id)
            .collect::<Vec<_>>(),
        vec![2]
    );
}

#[test]
fn initial_snapshot_is_not_new_but_learning_pages_and_glows() {
    let mut model = ToyBox::default();
    let catalog: Vec<_> = (1..=40)
        .map(|id| toy(id, &format!("Toy {id:02}"), id != 39))
        .collect();
    model.receive(catalog.clone());
    assert!(model.new_toys.is_empty());
    let mut learned = catalog;
    learned[38].learned = true;
    model.receive(learned);
    assert_eq!(model.page, 2);
    assert!(model.new_toys.contains(&39));
    model.acknowledge(39);
    assert!(!model.new_toys.contains(&39));
}

#[test]
fn authoritative_cooldown_ticks_clears_and_ignores_gcd() {
    let mut model = ToyBox::default();
    model.receive(vec![toy(1, "Orb", true)]);
    let mut update = SpellCooldownUpdate {
        spell_id: 1001,
        category: 0,
        duration_ms: 10000,
        remaining_ms: 5000,
        is_gcd: false,
    };
    model.cooldowns.apply(&update);
    assert_eq!(model.cooldowns.fraction(1001), 0.5);
    model.cooldowns.tick(2.0);
    assert!((model.cooldowns.fraction(1001) - 0.3).abs() < 0.0001);
    update.is_gcd = true;
    model.cooldowns.apply(&update);
    assert!((model.cooldowns.fraction(1001) - 0.3).abs() < 0.0001);
    update.is_gcd = false;
    update.remaining_ms = 0;
    model.cooldowns.apply(&update);
    assert_eq!(model.cooldowns.fraction(1001), 0.0);
}

#[test]
fn toy_slot_round_trip_preserves_identity_and_rejects_unlearned() {
    let mut model = ToyBox::default();
    model.receive(vec![
        toy(32782, "Time-Lost Figurine", true),
        toy(1973, "Orb", false),
    ]);
    let action = ToyAction::from_slot(ActionRef::Item(32782), &model).unwrap();
    let request = game_engine_ui_model::main_action_bar_component::ActionBar::BottomLeft
        .assignment(5, action.to_slot(), 0)
        .unwrap();
    assert_eq!(request.slot, 65);
    assert_eq!(
        ToyAction::from_slot(request.action.unwrap(), &model),
        Some(action)
    );
    assert_eq!(action.use_request().item_id, 32782);
    assert!(model.pickup(1973).is_none());
    assert!(ToyAction::from_slot(ActionRef::Item(6948), &model).is_none());
}
