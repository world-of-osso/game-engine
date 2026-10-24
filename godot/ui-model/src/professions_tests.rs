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
