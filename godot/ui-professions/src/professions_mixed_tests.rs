use game_engine_ui_model::bag_data::{InventorySlot, InventoryState};
use game_engine_ui_model::professions::{ProfessionBook, Recipe};
use shared::profession::ProfessionSkillLine;
use shared::protocol::ProfessionSnapshot;

fn book() -> ProfessionBook {
    ProfessionBook {
        snapshot: ProfessionSnapshot {
            lines: vec![ProfessionSkillLine {
                skill_line: 2540,
                step: 1,
                rank: 1,
                max_rank: 75,
            }],
            spells: vec![3908, 3275, 3276],
            favorites: vec![],
            recipe_sources: vec![],
        },
        recipes: vec![
            Recipe {
                spell_id: 3275,
                skill_line: 2540,
                profession: 197,
                category: "Bandages".into(),
                category_order: 0,
                name: "Linen Bandage".into(),
                min_rank: 1,
                trivial_low: 30,
                trivial_high: 60,
                output: Some((1251, 1)),
                reagents: vec![(2589, 1)],
            },
            Recipe {
                spell_id: 3276,
                skill_line: 2540,
                profession: 197,
                category: "Bandages".into(),
                category_order: 0,
                name: "Heavy Linen Bandage".into(),
                min_rank: 40,
                trivial_low: 50,
                trivial_high: 100,
                output: Some((2581, 1)),
                reagents: vec![(2589, 2)],
            },
            Recipe {
                spell_id: 2963,
                skill_line: 197,
                profession: 197,
                category: "Cloth".into(),
                category_order: 0,
                name: "Bolt of Linen Cloth".into(),
                min_rank: 1,
                trivial_low: 25,
                trivial_high: 50,
                output: Some((2996, 1)),
                reagents: vec![(2589, 2)],
            },
        ],
        selected: Some(3275),
        quantity: 2,
        visible: true,
        ..Default::default()
    }
}
fn bags() -> InventoryState {
    InventoryState {
        slots: vec![
            vec![InventorySlot {
                item_guid: 11,
                item_id: 2589,
                count: 3,
                ..Default::default()
            }],
            vec![InventorySlot {
                item_guid: 12,
                item_id: 2589,
                count: 2,
                ..Default::default()
            }],
        ],
        ..Default::default()
    }
}
#[test]
fn professions_both_skins_show_counts_and_disable_missing_reagent_create() {
    use crate::professions_frame::{ItemDisplay, ProfessionView, professions_screen};
    use ui_toolkit::{
        atlas::{ActiveSkin, set_thread_skin},
        frame::WidgetData,
        registry::FrameRegistry,
        screen::{Screen, SharedContext},
        widgets::button::ButtonState,
    };
    game_engine_ui_model::paths::set_data_root(
        std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../data"),
    )
    .unwrap();
    for skin in [ActiveSkin::Modern, ActiveSkin::Forever] {
        set_thread_skin(skin);
        let book = book();
        let inventory = bags();
        let mut view = ProfessionView {
            reagents: book.reagent_counts(&inventory),
            can_create: book.craft_request(&inventory, false).is_some(),
            can_all: book.craft_request(&inventory, true).is_some(),
            craftable: book.craftable_count(&inventory),
            book,
            skill_names: [(2540, "Classic Tailoring".into())].into(),
            items: [
                (
                    1251,
                    ItemDisplay {
                        name: "Linen Bandage".into(),
                        icon_fdid: 133681,
                        quality: 1,
                    },
                ),
                (
                    2589,
                    ItemDisplay {
                        name: "Linen Cloth".into(),
                        icon_fdid: 132889,
                        quality: 1,
                    },
                ),
            ]
            .into(),
            ..Default::default()
        };
        let mut shared = SharedContext::new();
        shared.insert(skin);
        shared.insert(view.clone());
        let mut registry = FrameRegistry::new(1920.0, 1080.0);
        let mut screen = Screen::new(professions_screen);
        screen.sync(&shared, &mut registry);
        let reagent = registry
            .get(registry.get_by_name("ProfessionReagent0").unwrap())
            .unwrap();
        assert!(
            matches!(reagent.widget_data.as_ref(), Some(WidgetData::FontString(text)) if text.text == "5/1 Linen Cloth")
        );
        let create = registry
            .get(registry.get_by_name("ProfessionsCreate").unwrap())
            .unwrap();
        assert!(
            matches!(create.widget_data.as_ref(), Some(WidgetData::Button(button)) if button.state != ButtonState::Disabled)
        );
        let inventory = InventoryState::default();
        view.reagents = view.book.reagent_counts(&inventory);
        view.can_create = view.book.craft_request(&inventory, false).is_some();
        view.can_all = view.book.craft_request(&inventory, true).is_some();
        shared.insert(view);
        screen.sync(&shared, &mut registry);
        let create = registry
            .get(registry.get_by_name("ProfessionsCreate").unwrap())
            .unwrap();
        assert!(
            matches!(create.widget_data.as_ref(), Some(WidgetData::Button(button)) if button.state == ButtonState::Disabled)
        );
        let reagent = registry
            .get(registry.get_by_name("ProfessionReagent0").unwrap())
            .unwrap();
        assert!(
            matches!(reagent.widget_data.as_ref(), Some(WidgetData::FontString(text)) if text.text == "0/1 Linen Cloth")
        );
    }
    set_thread_skin(ActiveSkin::Modern);
}

#[test]
fn professions_recipe_cooldown_is_the_longer_of_spell_and_category_clocks() {
    use crate::professions_cooldown::remaining_secs;
    assert_eq!(remaining_secs(None, None), None);
    assert_eq!(remaining_secs(Some(10.2), None), Some(11));
    // An uncast recipe sharing the category of a cast one is on cooldown too.
    assert_eq!(remaining_secs(None, Some(172_799.5)), Some(172_800));
    assert_eq!(remaining_secs(Some(5.0), Some(60.0)), Some(60));
    assert_eq!(remaining_secs(Some(0.0), None), None);
}

#[test]
fn professions_recipe_cooldown_text_uses_two_retail_units() {
    game_engine_ui_model::paths::set_data_root(
        std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../data"),
    )
    .unwrap();
    use crate::professions_cooldown::remaining_text;
    assert_eq!(remaining_text(172_800), "Cooldown remaining: 2 Days");
    assert_eq!(
        remaining_text(172_799),
        "Cooldown remaining: 1 Day 23 Hours"
    );
    assert_eq!(
        remaining_text(3_605),
        "Cooldown remaining: 1 Hour 5 Seconds"
    );
    assert_eq!(remaining_text(61), "Cooldown remaining: 1 Minute 1 Second");
    assert_eq!(remaining_text(59), "Cooldown remaining: 59 Seconds");
}

#[test]
fn professions_both_skins_cooldown_disables_create_and_shows_remaining() {
    use crate::professions_frame::{ProfessionView, professions_screen};
    use ui_toolkit::{
        atlas::{ActiveSkin, set_thread_skin},
        frame::WidgetData,
        registry::FrameRegistry,
        screen::{Screen, SharedContext},
        widgets::button::ButtonState,
    };
    game_engine_ui_model::paths::set_data_root(
        std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../data"),
    )
    .unwrap();
    let button_state = |registry: &FrameRegistry, name: &str| match registry
        .get(registry.get_by_name(name).unwrap())
        .unwrap()
        .widget_data
        .as_ref()
    {
        Some(WidgetData::Button(button)) => button.state,
        other => panic!("{name}: {other:?}"),
    };
    for skin in [ActiveSkin::Modern, ActiveSkin::Forever] {
        set_thread_skin(skin);
        let book = book();
        let inventory = bags();
        let mut view = ProfessionView {
            reagents: book.reagent_counts(&inventory),
            can_create: book.craft_request(&inventory, false).is_some(),
            can_all: book.craft_request(&inventory, true).is_some(),
            craftable: book.craftable_count(&inventory),
            recipe_cooldown_secs: Some(172_799),
            book,
            ..Default::default()
        };
        assert!(
            view.can_create && view.can_all,
            "reagents alone allow crafting"
        );
        let mut shared = SharedContext::new();
        shared.insert(skin);
        shared.insert(view.clone());
        let mut registry = FrameRegistry::new(1920.0, 1080.0);
        let mut screen = Screen::new(professions_screen);
        screen.sync(&shared, &mut registry);
        let cooldown = registry
            .get(
                registry
                    .get_by_name("ProfessionsRecipeCooldown")
                    .expect("cooldown line"),
            )
            .unwrap();
        let Some(WidgetData::FontString(text)) = cooldown.widget_data.as_ref() else {
            panic!("{:?}", cooldown.widget_data)
        };
        assert_eq!(text.text, "Cooldown remaining: 1 Day 23 Hours");
        assert_eq!(
            text.color,
            game_engine_ui_model::item_tooltip::RED_FONT_COLOR
        );
        assert_eq!(
            button_state(&registry, "ProfessionsCreate"),
            ButtonState::Disabled
        );
        assert_eq!(
            button_state(&registry, "ProfessionsCreateAll"),
            ButtonState::Disabled
        );

        view.recipe_cooldown_secs = None;
        shared.insert(view);
        screen.sync(&shared, &mut registry);
        assert!(registry.get_by_name("ProfessionsRecipeCooldown").is_none());
        assert_ne!(
            button_state(&registry, "ProfessionsCreate"),
            ButtonState::Disabled
        );
        assert_ne!(
            button_state(&registry, "ProfessionsCreateAll"),
            ButtonState::Disabled
        );
    }
    set_thread_skin(ActiveSkin::Modern);
}
