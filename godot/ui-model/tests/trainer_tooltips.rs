use game_engine_core::spell_catalog::CatalogSpell;
use game_engine_ui_model::{
    game_tooltip::{
        GameTooltipView, TooltipAnchor, TooltipScreen, game_tooltip_screen, place,
        spell::{SpellTooltipInput, spell_tooltip},
    },
    trainer::TrainerBook,
    trainer_frame::{TrainerView, trainer_screen},
};
use shared::protocol::{TrainerList, TrainerService, TrainerServiceState};
use ui_toolkit::{
    atlas::{ActiveSkin, set_thread_skin},
    frame::WidgetData,
    registry::FrameRegistry,
    screen::{Screen, SharedContext},
};

fn setup(skin: ActiveSkin) -> (TrainerBook, FrameRegistry) {
    game_engine_ui_model::paths::set_data_root(
        std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../data"),
    )
    .unwrap();
    set_thread_skin(skin);
    let mut book = TrainerBook::default();
    book.receive_list(TrainerList {
        npc: 7,
        trainer_id: 1,
        greeting: String::new(),
        services: [
            TrainerServiceState::Available,
            TrainerServiceState::Unavailable,
            TrainerServiceState::Known,
        ]
        .into_iter()
        .enumerate()
        .map(|(index, state)| TrainerService {
            spell_id: 2963 + index as u32,
            state,
            cost: 12550,
            req_level: 5,
            req_skill_line: 0,
            req_skill_rank: 0,
            req_abilities: vec![],
            profession: false,
        })
        .collect(),
    });
    let mut ctx = SharedContext::new();
    ctx.insert(skin);
    ctx.insert(TrainerView {
        book: book.clone(),
        ..Default::default()
    });
    let mut registry = FrameRegistry::new(1920.0, 1080.0);
    Screen::new(trainer_screen).sync(&ctx, &mut registry);
    (book, registry)
}

#[test]
fn trainer_hover_resolves_row_descendants_and_all_service_states_not_selection() {
    for skin in [ActiveSkin::Modern, ActiveSkin::Forever] {
        let (mut book, registry) = setup(skin);
        for spell in [2963, 2964, 2965] {
            let row = registry
                .get_by_name(&format!("ClassTrainerService{spell}"))
                .unwrap();
            let requirement = if spell == 2965 {
                "Requirements"
            } else {
                "Requirements0"
            };
            for suffix in ["", "Icon", "Name", requirement] {
                let hit = registry
                    .get_by_name(&format!("ClassTrainerService{spell}{suffix}"))
                    .unwrap();
                assert_eq!(book.hovered_service(&registry, hit), Some((row, spell)));
            }
        }
        let frame = registry.get_by_name("ClassTrainerFrame").unwrap();
        assert_eq!(book.hovered_service(&registry, frame), None);
        let unavailable = registry.get_by_name("ClassTrainerService2964Icon").unwrap();
        book.toggle_filter(TrainerServiceState::Unavailable);
        assert_eq!(book.hovered_service(&registry, unavailable), None);
        book.close();
        assert_eq!(
            book.hovered_service(
                &registry,
                registry.get_by_name("ClassTrainerService2963").unwrap()
            ),
            None
        );
    }
}

#[test]
fn trainer_spell_tooltip_uses_shared_content_right_offset_and_id_in_both_skins() {
    for skin in [ActiveSkin::Modern, ActiveSkin::Forever] {
        let (book, _) = setup(skin);
        let spell = CatalogSpell {
            id: 2963,
            name: "Bolt of Linen Cloth".into(),
            cast_time_ms: 3000,
            ..Default::default()
        };
        let tooltip = spell_tooltip(
            &spell,
            &SpellTooltipInput {
                description: "Creates a Bolt of Linen Cloth.".into(),
                ..Default::default()
            },
        );
        let tooltip = book
            .service_tooltip(2963, tooltip.clone(), [34.0, 207.0, 298.0, 47.0])
            .unwrap();
        assert_eq!(
            tooltip.anchor,
            TooltipAnchor::Owner {
                rect: [69.0, 207.0, 298.0, 47.0],
                side: game_engine_ui_model::game_tooltip::OwnerSide::Right
            }
        );
        let main = place(
            tooltip.for_skin(skin),
            TooltipScreen {
                size: [1920.0, 1080.0],
                cursor: [100.0, 220.0],
            },
        );
        assert_eq!(main.title, "Bolt of Linen Cloth");
        let lines: Vec<_> = main
            .lines
            .iter()
            .map(|line| line.left_text.as_str())
            .collect();
        assert_eq!(
            lines,
            [
                "3 sec cast",
                "Creates a Bolt of Linen Cloth.",
                "Spell ID: 2963"
            ]
        );
        assert_eq!(main.x, 367.0);
        let mut ctx = SharedContext::new();
        ctx.insert(skin);
        ctx.insert(GameTooltipView {
            main,
            ..Default::default()
        });
        let mut registry = FrameRegistry::new(1920.0, 1080.0);
        Screen::new(game_tooltip_screen).sync(&ctx, &mut registry);
        let title = registry
            .get(registry.get_by_name("TooltipTitle").unwrap())
            .unwrap();
        assert!(
            matches!(&title.widget_data, Some(WidgetData::FontString(font)) if font.text == "Bolt of Linen Cloth")
        );
        assert!(
            book.service_tooltip(9999, spell_tooltip(&spell, &Default::default()), [0.0; 4])
                .is_none()
        );
    }
}

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
    let data =
        std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../data/db2/12.1.0.69933");
    let recipes = game_engine_ui_model::professions_catalog::RecipeCatalog::load(&data).unwrap();
    for skin in [ActiveSkin::Modern, ActiveSkin::Forever] {
        set_thread_skin(skin);
        let tooltip = game_engine_ui_model::game_tooltip::trainer::trainer_service_content(
            spell_tooltip(catalog.get(2963).unwrap(), &Default::default()),
            recipes
                .recipes
                .iter()
                .find(|recipe| recipe.spell_id == 2963),
            Some(10),
        )
        .unwrap();
        let main = place(
            tooltip.for_skin(skin),
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
fn trainer_class_spell_retains_full_shared_tooltip() {
    let catalog = local_catalog();
    let spell = catalog.get(116).unwrap();
    let input = SpellTooltipInput {
        description: catalog
            .render_description(116, &Default::default())
            .unwrap(),
        ..Default::default()
    };
    assert!(!input.description.is_empty());
    for skin in [ActiveSkin::Modern, ActiveSkin::Forever] {
        let tooltip = game_engine_ui_model::game_tooltip::trainer::trainer_service_content(
            spell_tooltip(spell, &input),
            None,
            Some(10),
        )
        .unwrap();
        assert_eq!(tooltip, spell_tooltip(spell, &input));
        let main = place(
            tooltip.for_skin(skin),
            TooltipScreen {
                size: [1920.0, 1080.0],
                cursor: [0.0; 2],
            },
        );
        assert_eq!(main.title, "Frostbolt");
        assert!(
            main.lines
                .iter()
                .any(|line| line.left_text == "2% of base mana")
        );
        assert!(main.lines.iter().any(|l| l.right_text.contains("yd range")));
        assert!(main.lines.iter().any(|l| l.left_text.contains("sec cast")));
        assert!(main.lines.iter().any(|line| {
            line.left_color == game_engine_ui_model::tooltip_presentation::TOOLTIP_DESCRIPTION_COLOR
                && !line.left_text.is_empty()
        }));
        assert_eq!(main.lines.last().unwrap().left_text, "Spell ID: 116");
    }
}

#[test]
fn trainer_tooltip_requirements_refresh_with_the_current_service() {
    let (book, _) = setup(ActiveSkin::Modern);
    let mut view = TrainerView {
        book,
        player_level: 4,
        ..Default::default()
    };
    view.display.skills.insert(2540, "Classic Tailoring".into());
    view.book.list.as_mut().unwrap().services[0].req_skill_line = 2540;
    view.book.list.as_mut().unwrap().services[0].req_skill_rank = 75;
    view.book.list.as_mut().unwrap().services[0].req_abilities = vec![3908];
    view.display.names.insert(3908, "Tailoring".into());
    let content = spell_tooltip(
        &CatalogSpell {
            id: 2963,
            name: "Bolt of Linen Cloth".into(),
            ..Default::default()
        },
        &Default::default(),
    );
    let tooltip = view
        .service_tooltip(2963, content.clone(), [0.0; 4])
        .unwrap();
    assert!(
        tooltip
            .content
            .lines
            .iter()
            .any(|line| line.left_text == "Requires Level 5")
    );
    assert!(
        tooltip
            .content
            .lines
            .iter()
            .any(|line| line.left_text == "Requires Classic Tailoring (75)")
    );
    let red = game_engine_ui_model::item_tooltip::RED_FONT_COLOR;
    assert!(
        tooltip
            .content
            .lines
            .iter()
            .any(|line| line.left_text == "Requires Level 5" && line.left_color == red)
    );
    assert!(
        tooltip
            .content
            .lines
            .iter()
            .any(|line| line.left_text == "Requires Tailoring" && line.left_color == red)
    );
    view.player_level = 5;
    view.ranks.push(shared::profession::ProfessionSkillLine {
        skill_line: 2540,
        rank: 75,
        max_rank: 300,
        step: 1,
    });
    view.known_spells.insert(3908);
    let met = view
        .service_tooltip(2963, content.clone(), [0.0; 4])
        .unwrap();
    for expected in [
        "Requires Level 5",
        "Requires Classic Tailoring (75)",
        "Requires Tailoring",
    ] {
        assert!(
            met.content
                .lines
                .iter()
                .any(|line| line.left_text == expected && line.left_color == [1.0; 4])
        );
    }
    view.book.list.as_mut().unwrap().services[0].req_skill_rank = 150;
    let refreshed = view
        .service_tooltip(2963, content.clone(), [0.0; 4])
        .unwrap();
    assert!(
        refreshed
            .content
            .lines
            .iter()
            .any(|line| line.left_text == "Requires Classic Tailoring (150)")
    );
    assert!(
        !refreshed
            .content
            .lines
            .iter()
            .any(|line| line.left_text == "Requires Classic Tailoring (75)")
    );
    view.book.close();
    assert!(view.service_tooltip(2963, content, [0.0; 4]).is_none());
}

#[test]
fn trainer_recipe_uses_crafted_gear_quality_and_item_requirements() {
    let spells = local_catalog();
    let dir =
        std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../data/db2/12.1.0.69933");
    let recipes = game_engine_ui_model::professions_catalog::RecipeCatalog::load(&dir).unwrap();
    let recipe = recipes
        .recipes
        .iter()
        .find(|recipe| recipe.spell_id == 12069)
        .unwrap();
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
    let dir =
        std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../data/db2/12.1.0.69933");
    let recipes = game_engine_ui_model::professions_catalog::RecipeCatalog::load(&dir).unwrap();
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
