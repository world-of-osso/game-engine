use ui_toolkit::layout::LayoutRect;
use ui_toolkit::registry::FrameRegistry;
use ui_toolkit::screen::{Screen, SharedContext};

use super::*;
use crate::ui::screens::menu_character_layout_test_support::compute_layout;
use crate::ui::screens::screen_test_helpers::fontstring_text;

fn build(state: ProfessionsBookState) -> FrameRegistry {
    let mut reg = FrameRegistry::new(1920.0, 1080.0);
    let mut shared = SharedContext::new();
    shared.insert(state);
    Screen::new(professions_book_screen).sync(&shared, &mut reg);
    compute_layout(&mut reg);
    reg
}

fn rect(reg: &FrameRegistry, name: &str) -> LayoutRect {
    reg.get(reg.get_by_name(name).expect(name))
        .and_then(|frame| frame.layout_rect.clone())
        .unwrap_or_else(|| panic!("{name} has no layout rect"))
}

fn offset(reg: &FrameRegistry, name: &str) -> (f32, f32) {
    let (root, child) = (rect(reg, FRAME_NAME), rect(reg, name));
    (child.x - root.x, child.y - root.y)
}

fn tailoring() -> BookEntry {
    BookEntry {
        skill_line: 197,
        name: "Tailoring".into(),
        icon_fdid: 4_620_681,
        rank_title: "Classic Tailoring".into(),
        rank: 12,
        max_rank: 300,
        spell_name: "Tailoring".into(),
        spell_icon: 4_620_681,
    }
}

#[test]
fn a_learned_primary_shows_name_tier_rank_and_opens_its_frame() {
    let reg = build(ProfessionsBookState {
        visible: true,
        primary: [Some(tailoring()), None],
        ..Default::default()
    });
    assert_eq!(
        fontstring_text(&reg, "PrimaryProfession1ProfessionName"),
        "Tailoring"
    );
    assert_eq!(
        fontstring_text(&reg, "PrimaryProfession1Rank"),
        "Classic Tailoring"
    );
    assert_eq!(
        fontstring_text(&reg, "PrimaryProfession1StatusBarRank"),
        "12/300"
    );
    let button = reg
        .get(reg.get_by_name("PrimaryProfession1SpellButton").unwrap())
        .unwrap();
    assert_eq!(button.onclick.as_deref(), Some("professions_book_open:197"));
    // The second slot keeps the Retail prompt.
    assert_eq!(
        fontstring_text(&reg, "PrimaryProfession2Missing"),
        "Second Profession"
    );
}

#[test]
fn entries_follow_the_book_layout() {
    let reg = build(ProfessionsBookState {
        visible: true,
        primary: [Some(tailoring()), None],
        secondary: [
            Some(BookEntry {
                skill_line: 185,
                name: "Cooking".into(),
                rank_title: "Classic Cooking".into(),
                ..tailoring()
            }),
            None,
            None,
        ],
    });
    assert_eq!(offset(&reg, "PrimaryProfession1"), (80.0, 67.0));
    assert_eq!(offset(&reg, "PrimaryProfession2"), (80.0, 160.0));
    assert_eq!(offset(&reg, "SecondaryProfession1"), (80.0, 281.0));
    assert_eq!(
        fontstring_text(&reg, "SecondaryProfession1ProfessionName"),
        "Cooking"
    );
    assert!(reg.get_by_name("SecondaryProfession2").is_none());
}
