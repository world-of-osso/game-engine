use std::collections::HashMap;

use shared::profession::ProfessionSkillLine;

use super::*;
use crate::professions_data::{RecipeInfo, SkillLineInfo};

fn catalog() -> ProfessionCatalog {
    let line = |name: &str, parent| SkillLineInfo {
        name: name.into(),
        category: 11,
        parent,
        parent_tier_index: if parent == 0 { 0 } else { 4 },
        icon_fdid: 0,
        spell_book_spell: 0,
    };
    ProfessionCatalog {
        lines: HashMap::from([
            (197, line("Tailoring", 0)),
            (2540, line("Classic Tailoring", 197)),
        ]),
        recipes: HashMap::from([(
            2963,
            RecipeInfo {
                spell_id: 2963,
                name: "Bolt of Linen Cloth".into(),
                ..Default::default()
            },
        )]),
        ..Default::default()
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

fn snapshot(lines: Vec<ProfessionSkillLine>, spells: Vec<u32>) -> ProfessionSnapshot {
    ProfessionSnapshot { lines, spells }
}

#[test]
fn first_snapshot_after_entering_the_world_is_silent() {
    let mut status = ProfessionStatusSnapshot::default();
    let messages = apply_snapshot(
        &mut status,
        snapshot(vec![line(2540, 12)], vec![2963]),
        &catalog(),
    );
    assert!(messages.is_empty());
    assert_eq!(status.lines, vec![line(2540, 12)]);
    assert_eq!(status.spells, vec![2963]);
}

#[test]
fn learning_tailoring_reports_the_skills_and_the_bolt_recipe() {
    let mut status = ProfessionStatusSnapshot::default();
    apply_snapshot(&mut status, snapshot(vec![], vec![]), &catalog());
    let messages = apply_snapshot(
        &mut status,
        snapshot(vec![line(197, 1), line(2540, 1)], vec![3908, 2963]),
        &catalog(),
    );
    assert_eq!(
        messages,
        vec![
            "You have gained the Tailoring skill.",
            "You have gained the Classic Tailoring skill.",
            "You have learned how to create a new item: Bolt of Linen Cloth.",
        ]
    );
}

#[test]
fn a_craft_skill_up_reports_the_new_rank() {
    let mut status = ProfessionStatusSnapshot::default();
    apply_snapshot(
        &mut status,
        snapshot(vec![line(2540, 1)], vec![2963]),
        &catalog(),
    );
    let messages = apply_snapshot(
        &mut status,
        snapshot(vec![line(2540, 2)], vec![2963]),
        &catalog(),
    );
    assert_eq!(
        messages,
        vec!["Your skill in Classic Tailoring has increased to 2."]
    );
}

#[test]
fn status_names_lines_and_counts_known_recipes() {
    let status = ProfessionStatusSnapshot {
        lines: vec![line(2540, 3)],
        spells: vec![3908, 2963],
        received: true,
    };
    assert_eq!(
        format_status(&status, &catalog()),
        "professions: Classic Tailoring 3/300\nrecipes: 1"
    );
    assert_eq!(
        format_recipes(&status, &catalog(), "linen"),
        "recipes text=linen: 1\n2963 Bolt of Linen Cloth"
    );
}
