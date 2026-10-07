use super::*;
use crate::bag_data::InventorySlot;
use shared::profession::ProfessionSkillLine;

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
        },
        recipes: vec![
            Recipe {
                spell_id: 3275,
                skill_line: 2540,
                profession: 197,
                category: "Bandages".into(),
                name: "Linen Bandage".into(),
                min_rank: 1,
                trivial_low: 30,
                trivial_high: 60,
                output: (1251, 1),
                reagents: vec![(2589, 1)],
            },
            Recipe {
                spell_id: 3276,
                skill_line: 2540,
                profession: 197,
                category: "Bandages".into(),
                name: "Heavy Linen Bandage".into(),
                min_rank: 40,
                trivial_low: 50,
                trivial_high: 100,
                output: (2581, 1),
                reagents: vec![(2589, 2)],
            },
            Recipe {
                spell_id: 2963,
                skill_line: 197,
                profession: 197,
                category: "Cloth".into(),
                name: "Bolt of Linen Cloth".into(),
                min_rank: 1,
                trivial_low: 25,
                trivial_high: 50,
                output: (2996, 1),
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
fn professions_snapshot_groups_only_known_recipes() {
    let book = book();
    let groups = book.groups();
    assert_eq!(groups.len(), 1);
    assert_eq!(groups[0].0, "Bandages");
    assert_eq!(
        groups[0].1.iter().map(|r| r.spell_id).collect::<Vec<_>>(),
        [3276, 3275]
    );
    assert_eq!(book.snapshot.lines[0].rank, 1);
}
#[test]
fn professions_search_filters_case_insensitively() {
    let mut book = book();
    book.search = " hEaVy ".into();
    let groups = book.groups();
    assert_eq!(groups.len(), 1);
    assert_eq!(
        groups[0].1.iter().map(|r| r.spell_id).collect::<Vec<_>>(),
        [3276]
    );
    book.search = "silk".into();
    assert!(book.groups().is_empty());
}
#[test]
fn professions_schematic_sums_concrete_bag_stacks_not_equipment() {
    let mut bags = bags();
    bags.equipment.insert(
        shared::protocol::EquipmentSlot::MainHand,
        InventorySlot {
            item_guid: 13,
            item_id: 2589,
            count: 100,
            ..Default::default()
        },
    );
    assert_eq!(book().reagent_counts(&bags), [(2589, 5, 1)]);
}
#[test]
fn professions_create_emits_spell_id_and_cast_quantity() {
    let request = book().craft_request(&bags(), false).expect("can craft");
    assert_eq!(request.spell_id, 3275);
    assert_eq!(request.casts, 2);
    assert_eq!(book().craft_request(&bags(), true).unwrap().casts, 5);
}
#[test]
fn professions_missing_reagents_disable_create() {
    assert!(
        book()
            .craft_request(&InventoryState::default(), false)
            .is_none()
    );
    let mut book = book();
    book.quantity = 6;
    assert!(book.craft_request(&bags(), false).is_none());
    book.quantity = 0;
    assert!(book.craft_request(&bags(), false).is_none());
}
#[test]
fn professions_snapshot_and_bag_refresh_change_rank_and_remaining_crafts() {
    let mut book = book();
    let mut bags = bags();
    book.snapshot.lines[0].rank = 2;
    bags.slots[0][0].count = 2;
    assert_eq!(book.reagent_counts(&bags), [(2589, 4, 1)]);
    assert_eq!(book.craft_request(&bags, true).unwrap().casts, 4);
    assert_eq!(book.snapshot.lines[0].rank, 2);
    book.snapshot.spells.clear();
    assert!(book.craft_request(&bags, true).is_none());
}

#[test]
fn professions_db2_catalog_joins_classic_tailoring_recipe() {
    use crate::professions_catalog::RecipeCatalog;
    let dir = std::env::temp_dir().join(format!("professions-catalog-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    for (table, data) in [
        (
            "SkillLine",
            "ID,DisplayName_lang,SpellBookSpellID\n197,Tailoring,3908\n2540,Classic Tailoring,0\n",
        ),
        ("TradeSkillCategory", "ID,Name_lang\n1089,Bandages\n"),
        (
            "SpellEffect",
            "SpellID,DifficultyID,Effect,EffectItemType,EffectBasePointsF\n3275,0,24,1251,1\n3908,0,118,0,1\n",
        ),
        (
            "SpellReagents",
            "SpellID,Reagent_0,Reagent_1,Reagent_2,Reagent_3,Reagent_4,Reagent_5,Reagent_6,Reagent_7,ReagentCount_0,ReagentCount_1,ReagentCount_2,ReagentCount_3,ReagentCount_4,ReagentCount_5,ReagentCount_6,ReagentCount_7\n3275,2589,0,0,0,0,0,0,0,1,0,0,0,0,0,0,0\n44864,34259,0,0,0,0,0,0,0,-1,0,0,0,0,0,0,0\n",
        ),
        (
            "SkillLineAbility",
            "Spell,SkillLine,SkillupSkillLineID,TradeSkillCategoryID,MinSkillLineRank,TrivialSkillLineRankLow,TrivialSkillLineRankHigh\n3275,197,2540,1089,1,30,60\n3908,197,0,0,1,0,0\n",
        ),
    ] {
        std::fs::write(dir.join(format!("{table}.csv")), data).unwrap();
    }
    let catalog = RecipeCatalog::load(&dir).unwrap();
    assert_eq!(catalog.recipes.len(), 1);
    let recipe = &catalog.recipes[0];
    assert_eq!(
        (recipe.spell_id, recipe.skill_line, recipe.output),
        (3275, 2540, (1251, 1))
    );
    assert_eq!(recipe.reagents, [(2589, 1)]);
    assert_eq!(recipe.category, "Bandages");
    assert_eq!((recipe.trivial_low, recipe.trivial_high), (30, 60));
    assert_eq!(catalog.openers[&3908], 197);
    assert_eq!(catalog.skill_names[&2540], "Classic Tailoring");
    std::fs::remove_dir_all(dir).unwrap();
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
    crate::paths::set_data_root(
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
            matches!(reagent.widget_data.as_ref(), Some(WidgetData::FontString(text)) if text.text == "Linen Cloth")
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
            matches!(reagent.widget_data.as_ref(), Some(WidgetData::FontString(text)) if text.text == "Linen Cloth")
        );
    }
    set_thread_skin(ActiveSkin::Modern);
}
