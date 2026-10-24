use game_engine_ui_model::game_tooltip::{
    TooltipScreen, place,
    spell::{SpellTooltipInput, spell_tooltip},
};
use ui_toolkit::atlas::{ActiveSkin, set_thread_skin};

fn local_catalog() -> &'static game_engine_core::spell_catalog::SpellCatalogData {
    static CATALOG: std::sync::OnceLock<game_engine_core::spell_catalog::SpellCatalogData> =
        std::sync::OnceLock::new();
    CATALOG.get_or_init(|| {
        let root = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../data");
        game_engine_ui_model::paths::set_data_root(root.clone()).unwrap();
        game_engine_ui_model::item_catalog::wait_for_item_catalog();
        game_engine_core::spell_catalog::load_spell_catalog(
            &game_engine_core::spell_catalog::SpellCatalogPaths::for_data_dir(&root),
        )
        .unwrap()
    })
}

#[test]
fn trainer_recipe_shows_created_item_quality_reagents_and_both_record_ids() {
    let catalog = local_catalog();
    let data = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../data/db2/active");
    let recipes =
        game_engine_ui_professions::professions_catalog::RecipeCatalog::load(&data).unwrap();
    for skin in [ActiveSkin::Modern, ActiveSkin::Forever] {
        set_thread_skin(skin);
        let tooltip = game_engine_ui_model::game_tooltip::trainer::trainer_service_content(
            spell_tooltip(catalog.get(2963).unwrap(), &Default::default()),
            recipes.get(2963),
            Some(10),
        )
        .unwrap();
        let main = place(
            tooltip,
            TooltipScreen {
                size: [1920.0, 1080.0],
                cursor: [0.0; 2],
            },
        );
        assert_eq!(main.title, "Bolt of Linen Cloth");
        assert_eq!(main.title_color, [1.0, 1.0, 1.0, 1.0]);
        let lines: Vec<_> = main.lines.iter().map(|l| l.left_text.as_str()).collect();
        assert!(lines.contains(&"Reagents:"), "{lines:?}");
        assert!(lines.contains(&"Linen Cloth (2)"), "{lines:?}");
        assert!(lines.contains(&"Item ID: 2996"), "{lines:?}");
        assert_eq!(lines.last(), Some(&"Spell ID: 2963"));
    }
}

#[test]
fn trainer_recipe_uses_crafted_gear_quality_and_item_requirements() {
    let spells = local_catalog();
    let dir = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../data/db2/active");
    let recipes =
        game_engine_ui_professions::professions_catalog::RecipeCatalog::load(&dir).unwrap();
    let recipe = recipes.get(12069).unwrap();
    let tooltip = game_engine_ui_model::game_tooltip::trainer::trainer_service_content(
        spell_tooltip(spells.get(12069).unwrap(), &Default::default()),
        Some(recipe),
        Some(10),
    )
    .unwrap();
    assert_eq!(tooltip.content.title, "Cindercloth Robe");
    assert_eq!(
        tooltip.content.title_color,
        game_engine_ui_model::tooltip_presentation::parse_rgba(
            game_engine_ui_model::merchant_data::quality_color(2)
        )
    );
    assert!(
        tooltip
            .content
            .lines
            .iter()
            .any(|line| line.left_text == "Requires Level 17")
    );
    assert!(
        tooltip
            .content
            .lines
            .iter()
            .any(|line| line.left_text == "Item ID: 10042")
    );
}

#[test]
fn trainer_class_spell_that_creates_an_item_keeps_its_spell_tooltip() {
    let spells = local_catalog();
    let dir = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../data/db2/active");
    let recipes =
        game_engine_ui_professions::professions_catalog::RecipeCatalog::load(&dir).unwrap();
    // Rogue SkillLine/category7: creating an item alone does not make this a profession recipe.
    let spell = spells.get(212205).unwrap();
    let input = SpellTooltipInput {
        description: spells
            .render_description(212205, &Default::default())
            .unwrap(),
        ..Default::default()
    };
    let expected = spell_tooltip(spell, &input);
    let actual = game_engine_ui_model::game_tooltip::trainer::trainer_service_content(
        expected.clone(),
        recipes.get(212205),
        Some(10),
    );
    assert_eq!(actual, Ok(expected));
}

#[test]
fn trainer_secondary_profession_recipe_keeps_item_and_reagents() {
    let spells = local_catalog();
    let dir = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../data/db2/active");
    let recipes =
        game_engine_ui_professions::professions_catalog::RecipeCatalog::load(&dir).unwrap();
    let tooltip = game_engine_ui_model::game_tooltip::trainer::trainer_service_content(
        spell_tooltip(spells.get(7751).unwrap(), &Default::default()),
        recipes.get(7751),
        Some(10),
    )
    .unwrap();
    assert_eq!(tooltip.content.title, "Brilliant Smallfish");
    assert!(
        tooltip
            .content
            .lines
            .iter()
            .any(|line| line.left_text == "Item ID: 6290")
    );
    assert!(
        tooltip
            .content
            .lines
            .iter()
            .any(|line| line.left_text == "Raw Brilliant Smallfish (1)")
    );
}
