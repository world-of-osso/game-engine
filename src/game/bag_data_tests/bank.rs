use super::*;

#[test]
fn default_bank_has_28_main_slots() {
    let bank = BankState::default();
    assert_eq!(bank.slots.len(), 28);
    assert!(bank.main_slot(0).is_some());
    assert!(bank.main_slot(27).is_some());
    assert!(bank.main_slot(28).is_none());
}

#[test]
fn default_bank_bag_slots_are_locked() {
    let bank = BankState::default();
    assert_eq!(bank.bag_slots.len(), 7);
    assert_eq!(bank.purchased_bag_count(), 0);
    assert!(!bank.is_bag_slot_purchased(0));
}

#[test]
fn purchased_bag_slot_tracking() {
    let mut bank = BankState::default();
    bank.bag_slots[0].purchased = true;
    bank.bag_slots[0].bag_name = "Runecloth Bag".into();
    bank.bag_slots[0].bag_size = 14;
    assert!(bank.is_bag_slot_purchased(0));
    assert!(!bank.is_bag_slot_purchased(1));
    assert_eq!(bank.purchased_bag_count(), 1);
}

#[test]
fn reagent_slot_locked_state() {
    let mut bank = BankState::default();
    assert_eq!(bank.reagent_slots.len(), 98);
    assert!(bank.is_reagent_slot_locked(0));
    bank.reagent_unlocked = 49;
    assert!(!bank.is_reagent_slot_locked(0));
    assert!(!bank.is_reagent_slot_locked(48));
    assert!(bank.is_reagent_slot_locked(49));
}

#[test]
fn main_and_reagent_slots_are_independent() {
    let mut bank = BankState::default();
    bank.slots[0] = InventorySlot {
        icon_fdid: 100,
        name: "Ore".into(),
        count: 20,
        quality: ItemQuality::Common,
    };
    bank.reagent_slots[0] = InventorySlot {
        icon_fdid: 200,
        name: "Herb".into(),
        count: 5,
        quality: ItemQuality::Uncommon,
    };
    assert_eq!(bank.main_slot(0).unwrap().name, "Ore");
    assert_eq!(bank.reagent_slot(0).unwrap().name, "Herb");
    assert_eq!(bank.main_slot(0).unwrap().icon_fdid, 100);
    assert_eq!(bank.reagent_slot(0).unwrap().icon_fdid, 200);
}

#[test]
fn main_slot_out_of_bounds() {
    let bank = BankState::default();
    assert!(bank.main_slot(27).is_some());
    assert!(bank.main_slot(28).is_none());
    assert!(bank.main_slot(100).is_none());
}

#[test]
fn reagent_slot_out_of_bounds() {
    let bank = BankState::default();
    assert!(bank.reagent_slot(97).is_some());
    assert!(bank.reagent_slot(98).is_none());
}

#[test]
fn reagent_slot_content_when_unlocked() {
    let mut bank = BankState {
        reagent_unlocked: 10,
        ..Default::default()
    };
    bank.reagent_slots[5] = InventorySlot {
        icon_fdid: 300,
        name: "Flask".into(),
        count: 3,
        quality: ItemQuality::Rare,
    };
    let slot = bank.reagent_slot(5).unwrap();
    assert!(!slot.is_empty());
    assert_eq!(slot.name, "Flask");
    assert!(!bank.is_reagent_slot_locked(5));
    assert!(bank.is_reagent_slot_locked(10));
}

#[test]
fn multiple_bag_purchases() {
    let mut bank = BankState::default();
    assert_eq!(bank.purchased_bag_count(), 0);

    bank.bag_slots[0].purchased = true;
    bank.bag_slots[0].bag_name = "Frostweave Bag".into();
    bank.bag_slots[0].bag_size = 20;
    assert_eq!(bank.purchased_bag_count(), 1);

    bank.bag_slots[1].purchased = true;
    bank.bag_slots[1].bag_name = "Netherweave Bag".into();
    bank.bag_slots[1].bag_size = 16;
    assert_eq!(bank.purchased_bag_count(), 2);

    bank.bag_slots[6].purchased = true;
    bank.bag_slots[6].bag_name = "Embersilk Bag".into();
    bank.bag_slots[6].bag_size = 22;
    assert_eq!(bank.purchased_bag_count(), 3);

    for i in 2..6 {
        assert!(!bank.is_bag_slot_purchased(i));
    }
}

#[test]
fn bag_slot_purchased_out_of_bounds() {
    let bank = BankState::default();
    assert!(!bank.is_bag_slot_purchased(7));
    assert!(!bank.is_bag_slot_purchased(100));
}

#[test]
fn reagent_unlock_boundary_at_full() {
    let bank = BankState {
        reagent_unlocked: 98,
        ..Default::default()
    };
    for i in 0..98 {
        assert!(
            !bank.is_reagent_slot_locked(i),
            "slot {i} should be unlocked"
        );
    }
    assert!(bank.reagent_slot(98).is_none());
}
