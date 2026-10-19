//! Owner profession snapshots use the same native Account dispatch as world entry.
use super::*;
use shared::profession::ProfessionSkillLine;
use shared::protocol::ProfessionSnapshot;

#[test]
fn collection_journal_reaches_native_account_dispatch_during_loading() {
    let mut account = Account::new(PathBuf::new());
    account.session.screen = SessionScreen::Loading;
    let mut journal = shared::pet_battle::PetJournal::default();
    journal
        .add_with_breed(39, 7, shared::pet_battle::PetQuality::Rare, 4)
        .unwrap();
    let update = shared::protocol::CollectionStateUpdate {
        snapshot: None,
        message: None,
        error: None,
        pet_journal: Some(journal.clone()),
        summoned_pet_id: Some(1),
    };
    let mut events = Vec::new();
    account
        .dispatch_message(ProtocolMessage::for_tests(update.clone()), &mut events)
        .unwrap();
    assert_eq!(events.len(), 1);
    assert!(matches!(&events[0], AccountEvent::Collections(received) if received == &update));
}

#[test]
fn pet_combat_details_reach_native_account_during_loading() {
    let mut account = Account::new(PathBuf::new());
    account.session.screen = SessionScreen::Loading;
    let update = shared::protocol::PetJournalDetailsUpdate {
        pets: vec![shared::protocol::OwnedPetBattleSnapshot {
            pet_id: u64::MAX - 8,
            health: 73,
            max_health: 146,
            power: 27,
            speed: 31,
            next_level_xp: 100,
            selected_abilities: [384, 0, 0],
            choices: vec![],
        }],
        locked: true,
        error: None,
    };
    let mut events = Vec::new();
    account
        .dispatch_message(ProtocolMessage::for_tests(update.clone()), &mut events)
        .unwrap();
    assert_eq!(events.len(), 1);
    assert!(matches!(&events[0], AccountEvent::PetJournalDetails(received) if received == &update));
}

#[test]
fn professions_account_dispatches_loading_snapshot_and_skill_refresh() {
    let mut account = Account::new(PathBuf::new());
    account.session.screen = SessionScreen::Loading;
    let mut snapshot = ProfessionSnapshot {
        lines: vec![ProfessionSkillLine {
            skill_line: 2540,
            step: 1,
            rank: 1,
            max_rank: 300,
        }],
        spells: vec![3908, 264616, 3275],
        recipe_sources: vec![],
    };
    for rank in [1, 2] {
        snapshot.lines[0].rank = rank;
        let mut events = Vec::new();
        account
            .dispatch_message(ProtocolMessage::for_tests(snapshot.clone()), &mut events)
            .unwrap();
        assert_eq!(events.len(), 1);
        assert!(matches!(&events[0], AccountEvent::Professions(received) if received == &snapshot));
    }
}

#[test]
fn professions_workflow_account_dispatches_authoritative_catalog_and_refusals() {
    use shared::protocol::*;
    let mut account = Account::new(PathBuf::new());
    account.session.screen = SessionScreen::InWorld;
    let specialization = ProfessionSpecializationStateUpdate {
        snapshot: Some(ProfessionSpecializationsSnapshot {
            skill_line: 2830,
            knowledge_currency_id: 2025,
            knowledge_available: 1,
            currencies: vec![ProfessionSpecCurrencySnapshot {
                trait_currency_id: 9,
                currency_id: 2025,
                available: 1,
            }],
            trees: vec![],
        }),
        message: None,
        error: None,
    };
    let orders = CraftingOrderStateUpdate {
        snapshot: Some(CraftingOrdersSnapshot {
            skill_line: 0,
            kind: CraftingOrderKind::Public,
            mine: true,
            recipes: vec![],
            orders: vec![],
        }),
        message: None,
        error: None,
    };
    let refusal = CraftingOrderStateUpdate {
        snapshot: None,
        message: None,
        error: Some("Insufficient reagents".into()),
    };
    let mut events = Vec::new();
    account
        .dispatch_message(
            ProtocolMessage::for_tests(specialization.clone()),
            &mut events,
        )
        .unwrap();
    account
        .dispatch_message(ProtocolMessage::for_tests(orders.clone()), &mut events)
        .unwrap();
    account
        .dispatch_message(ProtocolMessage::for_tests(refusal.clone()), &mut events)
        .unwrap();
    assert_eq!(events.len(), 3);
    assert!(
        matches!(&events[0], AccountEvent::ProfessionSpecializations(update) if update == &specialization)
    );
    assert!(matches!(&events[1], AccountEvent::CraftingOrders(update) if update == &orders));
    assert!(matches!(&events[2], AccountEvent::CraftingOrders(update) if update == &refusal));
}
