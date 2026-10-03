use super::*;
use std::path::PathBuf;
use std::sync::OnceLock;

/// The build-pinned exports under the repository's `data/` (root or Godot crate).
fn data() -> &'static NpcGearData {
    static DATA: OnceLock<NpcGearData> = OnceLock::new();
    DATA.get_or_init(|| {
        let manifest = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        let dir = [manifest.join("data"), manifest.join("../../data")]
            .into_iter()
            .find(|dir| dir.is_dir())
            .expect("repository data directory")
            .join("db2/12.1.0.69933");
        NpcGearData::load(&dir).unwrap()
    })
}

fn emote_anim_id(emote: u32) -> Option<u16> {
    data().emote_anim_id(emote)
}

fn pose(stand_state: StandState, sheath_state: SheathState, emote_state: u32) -> UnitPose {
    UnitPose {
        stand_state,
        sheath_state,
        emote_state,
    }
}

/// Map 34 (The Stockade) TDB rows: Stockade Guard 375782 (creature_addon emote 333),
/// Petty Criminal template 46382 (StandState 3) and spawn 375707 (StandState 1).
#[test]
fn stockade_poses_play_retail_animations() {
    let guard = pose(StandState::Stand, SheathState::Melee, 333);
    assert_eq!(unit_pose_anim_id(&guard, emote_anim_id), Ok(Some(26)));
    let sleeping = pose(StandState::Sleep, SheathState::Melee, 0);
    assert_eq!(unit_pose_anim_id(&sleeping, emote_anim_id), Ok(Some(100)));
    let sitting = pose(StandState::Sit, SheathState::Melee, 0);
    assert_eq!(unit_pose_anim_id(&sitting, emote_anim_id), Ok(Some(97)));
    // Stockade Rifleman 46406: emote 214 EMOTE_STATE_READYRIFLE.
    let rifleman = pose(StandState::Stand, SheathState::Unarmed, 214);
    assert_eq!(unit_pose_anim_id(&rifleman, emote_anim_id), Ok(Some(48)));
    let standing = pose(StandState::Stand, SheathState::Melee, 0);
    assert_eq!(unit_pose_anim_id(&standing, emote_anim_id), Ok(None));
    let kneeling = pose(StandState::Kneel, SheathState::Unarmed, 0);
    assert_eq!(unit_pose_anim_id(&kneeling, emote_anim_id), Ok(Some(115)));
}

#[test]
fn unknown_emote_and_generic_chair_are_errors() {
    let unknown = pose(StandState::Stand, SheathState::Melee, 999_999);
    let error = unit_pose_anim_id(&unknown, emote_anim_id).unwrap_err();
    assert!(error.contains("999999"), "{error}");
    let chair = pose(StandState::SitChair, SheathState::Melee, 0);
    assert!(unit_pose_anim_id(&chair, emote_anim_id).is_err());
}

/// Stockade Guard 46405 equipment 1: sword 5305 (InventoryType 13, SheatheType 3) and
/// shield 1984 (InventoryType 14, SheatheType 4).
#[test]
fn guard_sword_and_shield_follow_sheath_state() {
    use EquipmentVisualSlot::{MainHand, OffHand};
    assert_eq!(
        virtual_item_attachment(MainHand, 13, 3, SheathState::Melee),
        Some(1)
    );
    assert_eq!(
        virtual_item_attachment(OffHand, 14, 4, SheathState::Melee),
        Some(0)
    );
    assert_eq!(
        virtual_item_attachment(MainHand, 13, 3, SheathState::Unarmed),
        Some(32)
    );
    assert_eq!(
        virtual_item_attachment(OffHand, 13, 3, SheathState::Unarmed),
        Some(33)
    );
    assert_eq!(
        virtual_item_attachment(OffHand, 14, 4, SheathState::Unarmed),
        Some(28)
    );
    assert_eq!(
        virtual_item_attachment(MainHand, 13, 3, SheathState::Ranged),
        Some(32)
    );
}

/// Stockade Rifleman 46406 equipment 1: rifle 12523 (InventoryType 26, SheatheType 0)
/// as ItemID1 and ItemID3, SheathState 0.
#[test]
fn rifle_without_sheath_position_stays_in_hand_and_ranged_copy_hides() {
    use EquipmentVisualSlot::{MainHand, Ranged};
    assert_eq!(
        virtual_item_attachment(MainHand, 26, 0, SheathState::Unarmed),
        Some(1)
    );
    assert_eq!(
        virtual_item_attachment(Ranged, 26, 0, SheathState::Unarmed),
        None
    );
    assert_eq!(
        virtual_item_attachment(Ranged, 26, 0, SheathState::Ranged),
        Some(1)
    );
    // A bow (InventoryType 15) is drawn in the left hand.
    assert_eq!(
        virtual_item_attachment(Ranged, 15, 1, SheathState::Ranged),
        Some(2)
    );
    assert_eq!(
        virtual_item_attachment(Ranged, 15, 1, SheathState::Melee),
        Some(27)
    );
}

/// Native `GetSheatheLink` (solarityclient `sheath_point`): a back (1) or large back (2)
/// sheath has a main-hand and an off-hand link, so two sheathed weapons do not overlap;
/// a gun or thrown weapon (InventoryType 26/25) uses the main-hand side.
#[test]
fn sheathed_main_and_off_hand_use_their_own_sides() {
    use EquipmentVisualSlot::{MainHand, OffHand, Ranged};
    let sheathed = |slot, inventory_type, sheathe_type| {
        virtual_item_attachment(slot, inventory_type, sheathe_type, SheathState::Unarmed)
    };
    assert_eq!(sheathed(MainHand, 13, 1), Some(26));
    assert_eq!(sheathed(OffHand, 13, 1), Some(27));
    assert_eq!(sheathed(MainHand, 17, 2), Some(30));
    assert_eq!(sheathed(OffHand, 17, 2), Some(31));
    assert_eq!(sheathed(Ranged, 26, 1), Some(26));
    assert_eq!(sheathed(Ranged, 15, 1), Some(27));
}

/// Stockade Guard display 2989 → CreatureDisplayInfoExtra 1274, build 12.1.0.69933.
#[test]
fn stockade_guard_display_authors_its_armor() {
    let slots = data();
    let armor = slots.display_armor(2989).unwrap();
    let entries: Vec<_> = armor
        .entries
        .iter()
        .map(|entry| (entry.slot, entry.display_info_id))
        .collect();
    use EquipmentVisualSlot::*;
    assert_eq!(
        entries,
        vec![
            (Shirt, Some(9447)),
            (Waist, Some(9448)),
            (Legs, Some(697)),
            (Feet, Some(6229)),
            (Hands, Some(9449)),
            (Tabard, Some(6255)),
        ]
    );
    // Display 36656's Extra 150806 has an ItemSlot 11 row (ItemDisplayInfo 185704), which
    // dresses nothing.
    let with_slot_11 = slots.display_armor(36656).unwrap();
    assert!(
        with_slot_11
            .entries
            .iter()
            .all(|entry| entry.display_info_id != Some(185704)),
        "{with_slot_11:?}"
    );
    // A display without an Extra authors no armor.
    assert!(slots.display_armor(u32::MAX).unwrap().entries.is_empty());
}

/// Stockade Guard 46405's sword 5305 (SheatheType 3) and shield 1984 (SheatheType 4) from
/// Item.db2: drawn in hand and on the wrist, sheathed at the left hip and on the back.
#[test]
fn guard_virtual_items_resolve_their_sheath_position_from_item_db2() {
    use EquipmentVisualSlot::{MainHand, OffHand, Ranged};
    assert_eq!(data().sheathe_type(5305), Some(3));
    assert_eq!(data().sheathe_type(1984), Some(4));
    let item = |slot, item_id, inventory_type| EquippedAppearanceEntry {
        slot,
        item_id: Some(item_id),
        display_info_id: None,
        inventory_type,
        hidden: false,
    };
    let guard = EquipmentAppearance {
        entries: vec![item(MainHand, 5305, 13), item(OffHand, 1984, 14)],
    };
    let attachments = |sheath| -> Vec<_> {
        data()
            .virtual_item_attachments(&guard, sheath)
            .into_iter()
            .map(|(entry, attachment)| (entry.slot, attachment))
            .collect()
    };
    assert_eq!(
        attachments(SheathState::Melee),
        [(MainHand, 1), (OffHand, 0)]
    );
    assert_eq!(
        attachments(SheathState::Unarmed),
        [(MainHand, 32), (OffHand, 28)]
    );
    // Stockade Rifleman 46406: rifle 12523 as main hand and ranged, sheath state 0.
    let rifleman = EquipmentAppearance {
        entries: vec![item(MainHand, 12523, 26), item(Ranged, 12523, 26)],
    };
    let shown: Vec<_> = data()
        .virtual_item_attachments(&rifleman, SheathState::Unarmed)
        .into_iter()
        .map(|(entry, attachment)| (entry.slot, attachment))
        .collect();
    assert_eq!(shown, [(MainHand, 1)]);
    assert_eq!(
        data().pose_anim_id(&pose(StandState::Stand, SheathState::Unarmed, 214)),
        Ok(Some(48))
    );
}
