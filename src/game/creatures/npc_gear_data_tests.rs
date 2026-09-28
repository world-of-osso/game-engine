use super::*;

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

/// Stockade Guard display 2989 → CreatureDisplayInfoExtra 1274, build 12.1.0.69933.
#[test]
fn stockade_guard_display_authors_its_armor() {
    let slots = npc_item_slots();
    let armor = slots.appearance(2989).unwrap();
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
    // A display without an Extra authors no armor.
    assert!(slots.appearance(u32::MAX).unwrap().entries.is_empty());
}
