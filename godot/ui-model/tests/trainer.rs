use game_engine_ui_model::trainer::{TrainerBook, TrainerDisplay};
use shared::protocol::{
    TrainerBuyFailed, TrainerBuySpell, TrainerFailReason, TrainerList, TrainerService,
    TrainerServiceState,
};
use std::collections::BTreeMap;

fn list() -> TrainerList {
    TrainerList {
        npc: 4201,
        trainer_id: 7,
        greeting: "Welcome, apprentice.".into(),
        services: vec![
            TrainerService {
                spell_id: 100,
                cost: 250,
                state: TrainerServiceState::Available,
                req_level: 5,
                req_skill_line: 164,
                req_skill_rank: 50,
                req_abilities: vec![99],
                profession: false,
            },
            TrainerService {
                spell_id: 200,
                cost: 1000,
                state: TrainerServiceState::Unavailable,
                req_level: 20,
                req_skill_line: 0,
                req_skill_rank: 0,
                req_abilities: vec![],
                profession: false,
            },
            TrainerService {
                spell_id: 300,
                cost: 10,
                state: TrainerServiceState::Known,
                req_level: 1,
                req_skill_line: 0,
                req_skill_rank: 0,
                req_abilities: vec![],
                profession: false,
            },
        ],
    }
}
fn book() -> TrainerBook {
    let mut book = TrainerBook::default();
    book.receive_list(list());
    book.money = 5000;
    book
}
#[test]
fn trainer_projects_concrete_list_cost_rank_and_ability() {
    let book = book();
    let display = TrainerDisplay {
        names: BTreeMap::from([
            (100, "Forging".into()),
            (99, "Apprentice Blacksmith".into()),
        ]),
        skills: BTreeMap::from([(164, "Blacksmithing".into())]),
        ..Default::default()
    };
    let rows = book.rows(&display);
    assert_eq!(book.list.as_ref().unwrap().greeting, "Welcome, apprentice.");
    assert_eq!(book.selected, Some(100));
    assert_eq!(rows[0].name, "Forging");
    assert_eq!(rows[0].cost, 250);
    assert_eq!(
        rows[0].requirements,
        "Requires Level 5, Blacksmithing (50), Apprentice Blacksmith"
    );
    assert_eq!(rows[2].requirements, "Already known");
}
#[test]
fn trainer_filter_preserves_server_states_and_reselects_visible_service() {
    let mut book = book();
    book.toggle_filter(TrainerServiceState::Available);
    assert_eq!(
        book.visible_services()
            .map(|s| s.spell_id)
            .collect::<Vec<_>>(),
        vec![200, 300]
    );
    assert_eq!(book.selected, Some(200));
    book.toggle_filter(TrainerServiceState::Unavailable);
    assert_eq!(
        book.visible_services()
            .map(|s| s.spell_id)
            .collect::<Vec<_>>(),
        vec![300]
    );
    book.toggle_filter(TrainerServiceState::Known);
    assert_eq!(book.selected, None);
}
#[test]
fn trainer_train_sends_exact_request_once_and_waits_for_authority() {
    let mut book = book();
    assert_eq!(
        book.train(),
        Some(TrainerBuySpell {
            npc: 4201,
            spell_id: 100
        })
    );
    assert_eq!(book.train(), None);
    assert_eq!(book.money, 5000);
    let mut updated = list();
    updated.services[0].state = TrainerServiceState::Known;
    book.receive_list(updated);
    book.money = 4750;
    assert_eq!(
        book.list.as_ref().unwrap().services[0].state,
        TrainerServiceState::Known
    );
    assert_eq!(book.money, 4750);
    assert!(!book.pending);
}
#[test]
fn trainer_failures_show_reason_without_spending_money() {
    let mut book = book();
    book.train();
    book.receive_failure(TrainerBuyFailed {
        npc: 4201,
        spell_id: 100,
        reason: TrainerFailReason::NotEnoughMoney,
    });
    assert_eq!(book.error, "You don't have enough money.");
    assert_eq!(book.money, 5000);
    book.train();
    book.receive_failure(TrainerBuyFailed {
        npc: 4201,
        spell_id: 100,
        reason: TrainerFailReason::Unavailable,
    });
    assert_eq!(book.error, "This service is unavailable.");
    assert!(!book.pending);
}
#[test]
fn trainer_primary_profession_requires_explicit_confirmation() {
    let mut book = book();
    book.list.as_mut().unwrap().services[0].profession = true;
    assert_eq!(book.train(), None);
    assert_eq!(book.confirmation, Some(100));
    assert_eq!(book.confirm(false), None);
    assert_eq!(book.confirmation, None);
    book.train();
    assert_eq!(
        book.confirm(true),
        Some(TrainerBuySpell {
            npc: 4201,
            spell_id: 100
        })
    );
    assert_eq!(book.confirm(true), None);
}
#[test]
fn trainer_unavailable_known_unaffordable_and_full_professions_cannot_train() {
    let mut book = book();
    for id in [200, 300] {
        book.select(id);
        assert_eq!(book.train(), None);
    }
    book.select(100);
    book.money = 249;
    assert_eq!(book.train(), None);
    book.money = 5000;
    book.list.as_mut().unwrap().services[0].profession = true;
    book.primary_professions = 2;
    assert_eq!(book.train(), None);
    assert_eq!(book.confirmation, None);
}
#[test]
fn trainer_confirmation_cannot_buy_a_different_selection_or_closed_npc() {
    let mut book = book();
    book.list.as_mut().unwrap().services[0].profession = true;
    book.train();
    book.select(200);
    assert_eq!(book.confirm(true), None);
    book.select(100);
    book.train();
    book.close();
    assert_eq!(book.confirm(true), None);
    book.receive_failure(TrainerBuyFailed {
        npc: 4201,
        spell_id: 100,
        reason: TrainerFailReason::NotEnoughMoney,
    });
    assert!(book.error.is_empty());
}
