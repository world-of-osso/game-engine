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

fn trainer_render(
    book: TrainerBook,
    skin: ui_toolkit::atlas::ActiveSkin,
) -> ui_toolkit::registry::FrameRegistry {
    trainer_render_rank(book, skin, (50, 300))
}

fn trainer_render_rank(
    book: TrainerBook,
    skin: ui_toolkit::atlas::ActiveSkin,
    (rank, max_rank): (u16, u16),
) -> ui_toolkit::registry::FrameRegistry {
    use game_engine_ui_model::trainer_frame::{TrainerView, trainer_screen};
    use ui_toolkit::screen::{Screen, SharedContext};
    game_engine_ui_model::paths::set_data_root(
        std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../data"),
    )
    .unwrap();
    ui_toolkit::atlas::set_thread_skin(skin);
    let mut shared = SharedContext::new();
    shared.insert(skin);
    shared.insert(TrainerView {
        book,
        title: "Herbalist Pomeroy".into(),
        display: TrainerDisplay {
            names: [(100, "Herbalism".into())].into(),
            ..Default::default()
        },
        ranks: vec![shared::profession::ProfessionSkillLine {
            skill_line: 164,
            step: 1,
            rank,
            max_rank,
        }],
    });
    let mut registry = ui_toolkit::registry::FrameRegistry::new(1920.0, 1080.0);
    Screen::new(trainer_screen).sync(&shared, &mut registry);
    registry
}
fn trainer_label(registry: &ui_toolkit::registry::FrameRegistry, name: &str) -> String {
    let frame = registry
        .get(registry.get_by_name(name).expect(name))
        .unwrap();
    match frame.widget_data.as_ref().unwrap() {
        ui_toolkit::frame::WidgetData::FontString(font) => font.text.clone(),
        other => panic!("Expected trainer label, got {other:?}"),
    }
}
#[test]
fn trainer_native_screen_projects_money_ranks_filters_failure_and_confirmation_in_both_skins() {
    use ui_toolkit::atlas::ActiveSkin;
    for skin in [ActiveSkin::Modern, ActiveSkin::Forever] {
        let mut state = book();
        state.filter_menu = true;
        state.error = "You don't have enough money.".into();
        let rendered = trainer_render(state.clone(), skin);
        assert_eq!(
            trainer_label(&rendered, "ClassTrainerMoneyFrame"),
            "Money: 0g 50s 0c"
        );
        assert_eq!(
            trainer_label(&rendered, "ClassTrainerStatusBarRankText"),
            "50/300"
        );
        assert_eq!(
            trainer_label(&rendered, "ClassTrainerError"),
            "You don't have enough money."
        );
        assert!(rendered.get_by_name("ClassTrainerFilter0").is_some());
        state.toggle_filter(TrainerServiceState::Available);
        let filtered = trainer_render(state.clone(), skin);
        assert!(filtered.get_by_name("ClassTrainerService100").is_none());
        assert!(filtered.get_by_name("ClassTrainerService200").is_some());
        state.toggle_filter(TrainerServiceState::Available);
        state.select(100);
        state.list.as_mut().unwrap().services[0].profession = true;
        state.train();
        let confirmed = trainer_render(state, skin);
        assert_eq!(
            trainer_label(&confirmed, "TrainerConfirmationText"),
            "You may only know two professions at any one time. Would you like to learn Herbalism as your first one?"
        );
    }
    ui_toolkit::atlas::set_thread_skin(ActiveSkin::Modern);
}

#[test]
fn trainer_root_profession_flag_has_no_progress_fraction() {
    use ui_toolkit::atlas::ActiveSkin;
    for skin in [ActiveSkin::Modern, ActiveSkin::Forever] {
        let mut state = book();
        state.list.as_mut().unwrap().services[0].req_skill_rank = 1;
        let registry = trainer_render_rank(state, skin, (1, 0));
        assert!(
            registry
                .get_by_name("ClassTrainerStatusBarRankText")
                .is_none()
        );
    }
    ui_toolkit::atlas::set_thread_skin(ActiveSkin::Modern);
}
