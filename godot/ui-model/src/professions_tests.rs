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
