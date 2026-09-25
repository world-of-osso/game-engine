use std::collections::HashMap;

use game_engine::professions_data::SkillLineInfo;
use game_engine::ui::popup::PopupId;
use shared::profession::ProfessionSkillLine;
use shared::protocol::TrainerList;

use super::*;

fn catalog() -> ProfessionCatalog {
    let line = |name: &str, parent, category| SkillLineInfo {
        name: name.into(),
        category,
        parent,
        parent_tier_index: if parent == 0 { 0 } else { 4 },
        icon_fdid: 0,
        spell_book_spell: 0,
    };
    ProfessionCatalog {
        lines: HashMap::from([
            (197, line("Tailoring", 0, 11)),
            (2540, line("Classic Tailoring", 197, 11)),
            (164, line("Blacksmithing", 0, 11)),
            (186, line("Mining", 0, 11)),
        ]),
        ..Default::default()
    }
}

fn service(spell_id: u32, state: TrainerServiceState) -> TrainerService {
    TrainerService {
        spell_id,
        cost: 10,
        state,
        req_level: 0,
        req_skill_line: 0,
        req_skill_rank: 0,
        req_abilities: vec![],
        profession: false,
    }
}

/// Georgio Bolero (trainer 163): Tailoring at level 5, White Linen Shirt at
/// Classic Tailoring 1.
fn georgio() -> TrainerState {
    let mut state = TrainerState::default();
    state.apply_list(
        TrainerList {
            npc: 7,
            trainer_id: 163,
            greeting: String::new(),
            services: vec![
                TrainerService {
                    req_level: 5,
                    profession: true,
                    ..service(264617, TrainerServiceState::Available)
                },
                TrainerService {
                    req_skill_line: 2540,
                    req_skill_rank: 1,
                    ..service(2393, TrainerServiceState::Unavailable)
                },
            ],
        },
        "Georgio Bolero".into(),
    );
    state
}

fn names(spell: u32) -> Option<(String, u32)> {
    match spell {
        264617 => Some(("Tailoring".into(), 4_620_681)),
        2393 => Some(("White Linen Shirt".into(), 132_149)),
        _ => None,
    }
}

fn line(skill_line: u32, rank: u16) -> ProfessionSkillLine {
    ProfessionSkillLine {
        skill_line,
        step: 1,
        rank,
        max_rank: 300,
    }
}

#[test]
fn georgio_rows_show_requirements_costs_and_the_selected_profession() {
    let professions = ProfessionStatusSnapshot::default();
    let player = Player {
        level: 10,
        money: 5,
        professions: &professions,
    };
    let state = build_state(&georgio(), true, &player, &catalog(), &names);

    assert_eq!(state.title, "Georgio Bolero");
    assert_eq!(state.rows.len(), 2);
    let tailoring = &state.rows[0];
    assert_eq!(
        (tailoring.name.as_str(), tailoring.sub_text.as_str()),
        ("Tailoring", "Requires: Level 5")
    );
    assert!(tailoring.selected && !tailoring.sub_text_red && tailoring.cost_red);
    let shirt = &state.rows[1];
    assert_eq!(shirt.sub_text, "Requires: Classic Tailoring (1)");
    assert!(shirt.sub_text_red && shirt.unavailable);
    // 5 copper cannot pay 10.
    assert!(!state.train_enabled);
    assert_eq!(state.rank, None, "no Classic Tailoring yet");
}

#[test]
fn affordable_tailoring_enables_train_until_two_primaries_are_known() {
    let none = ProfessionStatusSnapshot::default();
    let rich = Player {
        level: 10,
        money: 100,
        professions: &none,
    };
    assert!(build_state(&georgio(), true, &rich, &catalog(), &names).train_enabled);

    let two = ProfessionStatusSnapshot {
        lines: vec![line(164, 1), line(186, 1)],
        ..Default::default()
    };
    let full = Player {
        professions: &two,
        ..rich
    };
    assert!(!build_state(&georgio(), true, &full, &catalog(), &names).train_enabled);
}

#[test]
fn known_services_hide_the_cost_and_the_rank_bar_shows_classic_tailoring() {
    let mut trainer = georgio();
    trainer.services[0].state = TrainerServiceState::Known;
    let professions = ProfessionStatusSnapshot {
        lines: vec![line(197, 1), line(2540, 12)],
        spells: vec![3908],
        received: true,
    };
    let player = Player {
        level: 10,
        money: 100,
        professions: &professions,
    };
    let state = build_state(&trainer, true, &player, &catalog(), &names);
    assert_eq!(state.rows[0].sub_text, "Already known");
    assert_eq!(state.rows[0].cost, None);
    assert!(!state.rows[1].sub_text_red);
    assert_eq!(state.rank, Some(("12/300".into(), 0.04)));
}

#[test]
fn accepting_the_profession_confirmation_buys_the_selected_service() {
    let mut app = App::new();
    app.add_message::<PopupResult>()
        .add_message::<TrainerRequest>()
        .insert_resource(georgio())
        .add_systems(Update, confirm_profession);
    app.world_mut().write_message(PopupResult {
        id: PopupId(1),
        key: CONFIRM_PROFESSION.into(),
        outcome: PopupOutcome::Accepted,
    });
    app.world_mut().write_message(PopupResult {
        id: PopupId(2),
        key: CONFIRM_PROFESSION.into(),
        outcome: PopupOutcome::Cancelled,
    });
    app.update();
    let sent: Vec<TrainerRequest> = app
        .world_mut()
        .resource_mut::<Messages<TrainerRequest>>()
        .drain()
        .collect();
    assert_eq!(sent, vec![TrainerRequest { spell_id: 264617 }]);
}
