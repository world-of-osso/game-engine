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

fn local_catalog() -> game_engine_core::spell_catalog::SpellCatalogData {
    let root = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../data");
    game_engine_ui_model::paths::set_data_root(root.clone()).unwrap();
    game_engine_ui_model::item_catalog::wait_for_item_catalog();
    game_engine_core::spell_catalog::load_spell_catalog(
        &game_engine_core::spell_catalog::SpellCatalogPaths::for_data_dir(&root),
    )
    .unwrap()
}

#[test]
fn trainer_recipe_shows_created_item_quality_reagents_and_both_record_ids() {
    let catalog = local_catalog();
    for skin in [ActiveSkin::Modern, ActiveSkin::Forever] {
        set_thread_skin(skin);
        let tooltip = game_engine_ui_model::game_tooltip::trainer::trainer_spell_tooltip(
            catalog.get(2963).unwrap(),
            &Default::default(),
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
        assert_eq!(
            main.title_color,
            game_engine_ui_model::merchant_data::quality_color(1)
        );
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
        let tooltip = game_engine_ui_model::game_tooltip::trainer::trainer_spell_tooltip(
            spell,
            &input,
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
        assert!(main.lines.iter().any(|l| l.right_text.contains("yd range")));
        assert!(main.lines.iter().any(|l| l.left_text.contains("sec cast")));
        assert!(main.lines.iter().any(|l| l.left_text == input.description));
        assert_eq!(main.lines.last().unwrap().left_text, "Spell ID: 116");
    }
}
