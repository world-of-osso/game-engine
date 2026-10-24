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
fn professions_snapshot_groups_known_recipes_before_unlearned() {
    let book = book();
    let groups = book.groups();
    assert_eq!(groups.len(), 2);
    assert_eq!(groups[0].0, "Bandages");
    assert_eq!(groups[1].0, "Cloth");
    assert_eq!(groups[1].1[0].spell_id, 2963);
    assert_eq!(
        groups[0].1.iter().map(|r| r.spell_id).collect::<Vec<_>>(),
        [3276, 3275]
    );
    assert_eq!(book.snapshot.lines[0].rank, 1);
}
#[test]
fn professions_unlearned_recipe_is_selectable_but_cannot_emit_a_craft() {
    let mut book = book();
    book.snapshot.spells.retain(|spell| *spell != 3276);
    book.selected = Some(3276);
    assert_eq!(book.selected_recipe().unwrap().spell_id, 3276);
    assert_eq!(book.reagent_counts(&bags()), [(2589, 5, 2)]);
    assert_eq!(book.craftable_count(&bags()), 0);
    assert!(book.craft_request(&bags(), false).is_none());
    assert!(book.craft_request(&bags(), true).is_none());
    book.snapshot.spells.push(3276);
    assert_eq!(book.craft_request(&bags(), false).unwrap().spell_id, 3276);
    assert_eq!(book.groups()[0].1.len(), 2);
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
fn professions_create_all_counts_banked_reagents_after_the_bank_frame_closes() {
    use shared::protocol::{BankContents, BankTabView, BankType, ItemStack};
    let banked = |bank, count| BankContents {
        bank,
        tabs: vec![BankTabView {
            name: "Tab 1".into(),
            icon: 0,
            deposit_flags: 0,
            slots: vec![Some(ItemStack {
                item_guid: 20 + count as u64,
                item_id: 2589,
                definition_source: shared::item_data::ItemDefinitionSource::Retail,
                count,
                durability: None,
                soulbound: false,
                crafting_origin: None,
            })],
        }],
        next_tab_cost: None,
        money: None,
    };
    let mut bags = bags();
    bags.apply_bank_contents(&banked(BankType::Character, 4));
    bags.apply_bank_contents(&banked(BankType::Account, 9));
    // Closing the bank frame drops only the cursor projection.
    bags.bank_slots.clear();
    // A newer Warband snapshot replaces the old one.
    bags.apply_bank_contents(&banked(BankType::Account, 3));
    let mut book = book();
    book.selected = Some(3276);
    assert_eq!(book.reagent_counts(&bags), [(2589, 12, 2)]);
    assert_eq!(book.craft_request(&bags, true).unwrap().casts, 6);
}
#[test]
fn professions_create_emits_spell_id_and_cast_quantity() {
    let request = book().craft_request(&bags(), false).expect("can craft");
    assert_eq!(request.spell_id, 3275);
    assert_eq!(request.casts, 2);
    assert_eq!(book().craft_request(&bags(), true).unwrap().casts, 5);
}
#[test]
fn professions_trainer_learned_leather_pants_use_recipe_knowledge_not_auto_learn_rank() {
    let dir = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../data/db2/active");
    let catalog = crate::professions_catalog::RecipeCatalog::load(&dir).unwrap();
    let recipe = catalog.get(2153).unwrap().clone();
    assert_eq!(recipe.min_rank, 30);
    let book = ProfessionBook {
        recipes: vec![recipe],
        snapshot: ProfessionSnapshot {
            lines: vec![ProfessionSkillLine {
                skill_line: 2532,
                step: 1,
                rank: 15,
                max_rank: 300,
            }],
            spells: vec![2108, 2153],
            favorites: vec![],
            recipe_sources: vec![],
        },
        selected: Some(2153),
        quantity: 1,
        ..Default::default()
    };
    let bags = InventoryState {
        slots: vec![vec![
            InventorySlot {
                item_guid: 128,
                item_id: 2318,
                count: 14,
                ..Default::default()
            },
            InventorySlot {
                item_guid: 102,
                item_id: 2320,
                count: 1,
                ..Default::default()
            },
        ]],
        ..Default::default()
    };
    assert_eq!(book.craftable_count(&bags), 1);
    assert_eq!(
        book.craft_request(&bags, false),
        Some(CraftRecipe {
            spell_id: 2153,
            casts: 1
        })
    );
    let mut unlearned = book;
    unlearned.snapshot.spells.retain(|spell| *spell != 2153);
    assert!(unlearned.craft_request(&bags, false).is_none());
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
            "ModifiedCraftingSpellSlot",
            "SpellID,Slot,ModifiedCraftingReagentSlotID,ReagentCount,ReagentReCraftCount\n",
        ),
        (
            "ModifiedCraftingReagentSlot",
            "ID,Name_lang,ReagentType,PlayerConditionID\n",
        ),
        (
            "MCRSlotXMCRCategory",
            "ModifiedCraftingReagentSlotID,ModifiedCraftingCategoryID,Order\n",
        ),
        (
            "CraftingReagentQuality",
            "ModifiedCraftingCategoryID,ItemID,CurrencyTypesID,OrderIndex\n",
        ),
        (
            "CraftingQuality",
            "ID,QualityTier,CraftingQualityAtlasSetID\n14,2,7\n",
        ),
        (
            "CraftingQualityAtlasSet",
            "ID,IconInv,IconMixedInv\n7,32404,32399\n",
        ),
        (
            "UiTextureAtlasElement",
            "ID,Name\n32404,Professions-Icon-Quality-12-Tier2-Inv\n32399,Professions-Icon-Quality-12-Mixed-Inv\n",
        ),
        ("Item", "ID,CraftingQualityID\n244634,14\n2589,0\n"),
        (
            "SkillLine",
            "ID,DisplayName_lang,SpellBookSpellID,CategoryID,ParentSkillLineID,ParentTierIndex\n197,Tailoring,3908,11,0,0\n2540,Classic Tailoring,0,11,197,0\n",
        ),
        ("SkillLineXTraitTree", "ID,SkillLineID,Variant\n"),
        ("SpellCategories", "SpellID,DifficultyID,Category\n"),
        (
            "TradeSkillCategory",
            "ID,Name_lang,OrderIndex\n1089,Bandages,20\n1090,Weaving,10\n",
        ),
        (
            "SpellEffect",
            "SpellID,DifficultyID,Effect,EffectItemType,EffectBasePointsF\n3275,0,24,1251,1\n2963,0,24,2996,1\n3908,0,118,0,1\n",
        ),
        (
            "SpellReagents",
            "SpellID,Reagent_0,Reagent_1,Reagent_2,Reagent_3,Reagent_4,Reagent_5,Reagent_6,Reagent_7,ReagentCount_0,ReagentCount_1,ReagentCount_2,ReagentCount_3,ReagentCount_4,ReagentCount_5,ReagentCount_6,ReagentCount_7\n3275,2589,0,0,0,0,0,0,0,1,0,0,0,0,0,0,0\n44864,34259,0,0,0,0,0,0,0,-1,0,0,0,0,0,0,0\n",
        ),
        (
            "SkillLineAbility",
            "Spell,SkillLine,SkillupSkillLineID,TradeSkillCategoryID,MinSkillLineRank,TrivialSkillLineRankLow,TrivialSkillLineRankHigh\n3275,197,2540,1089,1,30,60\n2963,197,2540,1090,1,25,50\n3908,197,0,0,1,0,0\n",
        ),
    ] {
        std::fs::write(dir.join(format!("{table}.csv")), data).unwrap();
    }
    let catalog = RecipeCatalog::load(&dir).unwrap();
    assert_eq!(catalog.recipes.len(), 2);
    let recipe = catalog.get(3275).unwrap();
    assert_eq!(
        (recipe.spell_id, recipe.skill_line, recipe.output),
        (3275, 2540, Some((1251, 1)))
    );
    assert_eq!(recipe.reagents, [(2589, 1)]);
    assert_eq!(recipe.category, "Bandages");
    assert_eq!((recipe.trivial_low, recipe.trivial_high), (30, 60));
    assert_eq!(catalog.openers[&3908], 197);
    assert_eq!(catalog.skill_names[&2540], "Classic Tailoring");
    assert_eq!(catalog.crafting_quality[&244634].tier, 2);
    assert!(!catalog.crafting_quality.contains_key(&2589));
    let book = ProfessionBook {
        snapshot: ProfessionSnapshot {
            spells: vec![3275, 2963],
            ..Default::default()
        },
        recipes: catalog.recipes,
        ..Default::default()
    };
    let categories: Vec<_> = book.groups().into_iter().map(|(name, _)| name).collect();
    assert_eq!(categories, ["Weaving", "Bandages"]);
    std::fs::remove_dir_all(dir).unwrap();
}

#[test]
fn professions_recipe_cooldown_category_comes_from_spell_categories() {
    let dir = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../data/db2/active");
    let catalog = crate::professions_catalog::RecipeCatalog::load(&dir).unwrap();
    // Prismatic Sphere and Void Sphere share the 48 h Enchanting category.
    assert_eq!(catalog.cooldown_categories.get(&28027), Some(&1174));
    assert_eq!(catalog.cooldown_categories.get(&28028), Some(&1174));
    assert_eq!(catalog.cooldown_categories.get(&2153), None);
    // Berserker Rage is a class spell, not a recipe.
    assert!(!catalog.cooldown_categories.contains_key(&18499));
}
