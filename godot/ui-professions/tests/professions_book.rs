//! ProfessionsBookFrame empty and learned states through the production screen.
use game_engine_ui_professions::professions_catalog::RecipeCatalog;
use game_engine_ui_professions::professions_frame::{ProfessionView, professions_screen};
use shared::{profession::ProfessionSkillLine, protocol::ProfessionSnapshot};
use ui_toolkit::{
    atlas::{ActiveSkin, set_thread_skin},
    frame::{Frame, WidgetData},
    registry::FrameRegistry,
    screen::{Screen, SharedContext},
};

const PRIMARY_HINT: &str = "Visit a profession trainer in a major city to learn a new profession. You may have two professions. You may have any combination of gathering and production professions.";
const COOKING_HINT: &str = "Visit a trainer to learn cooking. Cooking lets you learn recipes to create food that heals you out of combat and grants you temporary buffs.";
const FISHING_HINT: &str = "Visit a trainer to learn fishing.  Fishing lets you catch fish and other strange things from water.  Fish can be cooked into delicious meals with the Cooking skill.";
const ARCHAEOLOGY_HINT: &str = "Visit a trainer to learn archaeology.  Archaeology allows you to discover valuable treasures and learn ancient lore of the world.";

fn render(view: ProfessionView, skin: ActiveSkin) -> FrameRegistry {
    game_engine_ui_model::paths::set_data_root(
        std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../data"),
    )
    .unwrap();
    set_thread_skin(skin);
    let mut shared = SharedContext::new();
    shared.insert(skin);
    shared.insert(view);
    let mut registry = FrameRegistry::new(1920.0, 1080.0);
    Screen::new(professions_screen).sync(&shared, &mut registry);
    registry
}

fn frame<'a>(registry: &'a FrameRegistry, name: &str) -> &'a Frame {
    registry
        .get(registry.get_by_name(name).expect(name))
        .unwrap()
}

fn text(registry: &FrameRegistry, name: &str) -> String {
    match frame(registry, name).widget_data.as_ref().unwrap() {
        WidgetData::FontString(font) => font.text.clone(),
        other => panic!("Expected label {name}, got {other:?}"),
    }
}

#[test]
fn professions_book_empty_character_opens_the_retail_empty_book() {
    for skin in [ActiveSkin::Modern, ActiveSkin::Forever] {
        let registry = render(
            ProfessionView {
                profession_book_visible: true,
                ..Default::default()
            },
            skin,
        );
        let root = frame(&registry, "ProfessionsBookFrame");
        let close = frame(&registry, "ProfessionsBookFrameCloseButton");
        assert_eq!(close.onclick.as_deref(), Some("profession:close"));
        assert!(registry.get_by_name("ProfessionsFrame").is_none());
        for (name, expected) in [
            ("ProfessionsBookPrimary0MissingHeader", "First Profession"),
            ("ProfessionsBookPrimary0MissingText", PRIMARY_HINT),
            ("ProfessionsBookPrimary1MissingHeader", "Second Profession"),
            ("ProfessionsBookPrimary1MissingText", PRIMARY_HINT),
            ("ProfessionsBookSecondary0MissingHeader", "Cooking"),
            ("ProfessionsBookSecondary0MissingText", COOKING_HINT),
            ("ProfessionsBookSecondary1MissingHeader", "Fishing"),
            ("ProfessionsBookSecondary1MissingText", FISHING_HINT),
            ("ProfessionsBookSecondary2MissingHeader", "Archaeology"),
            ("ProfessionsBookSecondary2MissingText", ARCHAEOLOGY_HINT),
        ] {
            assert_eq!(text(&registry, name), expected, "{skin:?} {name}");
        }
        assert_eq!(
            text(&registry, "ProfessionsBookFrameTitleText"),
            "Professions"
        );
        assert_eq!(
            root.lifecycle.as_ref().unwrap().close_hook.as_deref(),
            Some("professions")
        );
    }
}

#[test]
fn professions_book_learned_skill_lines_fill_their_retail_slots() {
    let db2 = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../data/db2/active");
    let catalog = RecipeCatalog::load(&db2).unwrap();
    let snapshot = ProfessionSnapshot {
        lines: vec![
            ProfessionSkillLine {
                skill_line: 197,
                step: 1,
                rank: 1,
                max_rank: 300,
            },
            ProfessionSkillLine {
                skill_line: 2540,
                step: 1,
                rank: 35,
                max_rank: 300,
            },
            ProfessionSkillLine {
                skill_line: 185,
                step: 1,
                rank: 20,
                max_rank: 300,
            },
        ],
        spells: vec![3908, 2550],
        ..Default::default()
    };
    let summary = catalog.book_summary(&snapshot);
    assert_eq!(summary.primary.len(), 1);
    assert_eq!(summary.primary[0].name, "Tailoring");
    assert_eq!(summary.primary[0].rank_name, "Classic Tailoring");
    assert_eq!(summary.primary[0].rank, 35);
    assert_eq!(summary.primary[0].max_rank, 300);
    assert_eq!(summary.primary[0].opener_spell, Some(3908));
    assert_eq!(summary.secondary[0].as_ref().unwrap().name, "Cooking");

    let registry = render(
        ProfessionView {
            profession_book_visible: true,
            profession_book: summary,
            ..Default::default()
        },
        ActiveSkin::Forever,
    );
    assert_eq!(
        text(&registry, "ProfessionsBookPrimary0ProfessionName"),
        "Tailoring"
    );
    assert_eq!(
        text(&registry, "ProfessionsBookPrimary0RankName"),
        "Classic Tailoring"
    );
    assert_eq!(
        text(&registry, "ProfessionsBookPrimary0StatusBarRank"),
        "35/300"
    );
    assert_eq!(
        text(&registry, "ProfessionsBookSecondary0ProfessionName"),
        "Cooking"
    );
    assert_eq!(
        text(&registry, "ProfessionsBookSecondary0StatusBarRank"),
        "20/300"
    );
    assert_eq!(
        frame(&registry, "ProfessionsBookPrimary0SpellButton")
            .onclick
            .as_deref(),
        Some("profession:open:3908")
    );
    assert!(
        registry
            .get_by_name("ProfessionsBookPrimary0MissingHeader")
            .is_none()
    );
}
