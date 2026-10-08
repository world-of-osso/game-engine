//! What a replicated creature holds and how it stands, from the build-pinned DB2
//! exports: its `UnitPose` (TrinityCore `UnitStandStateType`, `SheathState`,
//! `UnitData::EmoteState`), its virtual items (`EquipmentAppearance`) and the armor its
//! display authors (`NPCModelItemSlotDisplayInfo` of its `CreatureDisplayInfoExtra`).
//! Engine-free: the Bevy and Godot clients both read it.

use std::collections::HashMap;
use std::path::Path;

use shared::components::{
    EquipmentAppearance, EquipmentVisualSlot, EquippedAppearanceEntry, SheathState, StandState,
    UnitPose,
};

// AnimationData IDs (wowdev.wiki M2/AnimationList).
const ANIM_DEAD: u16 = 6;
const ANIM_SIT_GROUND: u16 = 97;
const ANIM_SLEEP: u16 = 100;
const ANIM_SIT_CHAIR_LOW: u16 = 102;
const ANIM_SIT_CHAIR_MED: u16 = 103;
const ANIM_SIT_CHAIR_HIGH: u16 = 104;
const ANIM_KNEEL_LOOP: u16 = 115;
const ANIM_SUBMERGED: u16 = 202;

/// The looping animation a unit holds while it does not move: its stand state's pose,
/// else the `Emotes.AnimID` of its emote state, else none (it stands).
pub fn unit_pose_anim_id(
    pose: &UnitPose,
    emote_anim_id: impl Fn(u32) -> Option<u16>,
) -> Result<Option<u16>, String> {
    let anim = match pose.stand_state {
        StandState::Stand if pose.emote_state == 0 => return Ok(None),
        StandState::Stand => emote_anim_id(pose.emote_state)
            .ok_or_else(|| format!("Emotes.db2 has no emote {}", pose.emote_state))?,
        StandState::Sit => ANIM_SIT_GROUND,
        StandState::SitChair => {
            return Err("stand state SitChair (2) has no established Retail animation".into());
        }
        StandState::Sleep => ANIM_SLEEP,
        StandState::SitLowChair => ANIM_SIT_CHAIR_LOW,
        StandState::SitMediumChair => ANIM_SIT_CHAIR_MED,
        StandState::SitHighChair => ANIM_SIT_CHAIR_HIGH,
        StandState::Dead => ANIM_DEAD,
        StandState::Kneel => ANIM_KNEEL_LOOP,
        StandState::Submerged => ANIM_SUBMERGED,
    };
    Ok(Some(anim))
}

// WMVx `AttachmentPosition` / M2 attachment IDs.
const ATTACH_LEFT_WRIST: u32 = 0;
const ATTACH_RIGHT_PALM: u32 = 1;
const ATTACH_LEFT_PALM: u32 = 2;
const ATTACH_RIGHT_BACK_SHEATH: u32 = 26;
const ATTACH_LEFT_BACK_SHEATH: u32 = 27;
const ATTACH_MIDDLE_BACK_SHEATH: u32 = 28;
const ATTACH_LEFT_BACK: u32 = 30;
const ATTACH_RIGHT_BACK: u32 = 31;
const ATTACH_LEFT_HIP_SHEATH: u32 = 32;
const ATTACH_RIGHT_HIP_SHEATH: u32 = 33;

// `Item.InventoryType`.
const INVTYPE_SHIELD: u8 = 14;
const INVTYPE_RANGED: u8 = 15;
const INVTYPE_THROWN: u8 = 25;
const INVTYPE_RANGED_RIGHT: u8 = 26;

/// The attachment a creature virtual item renders on, or `None` when it is not shown.
/// Main and off hand are drawn with `SheathState::Melee`, the ranged item with
/// `SheathState::Ranged`; a shield is drawn on the left wrist, a bow in the left hand.
/// Otherwise the item sits at its `Item.SheatheType` position on the side of its hand
/// (native `GetSheatheLink`, solarityclient `sheath_point`: 1 back, 2 large weapon back,
/// 3 hip, each a main-hand and an off-hand link, 4 shield back); a hand item without one
/// stays in the hand, a ranged item without one is not shown.
pub fn virtual_item_attachment(
    slot: EquipmentVisualSlot,
    inventory_type: u8,
    sheathe_type: u8,
    sheath: SheathState,
) -> Option<u32> {
    let drawn = match slot {
        EquipmentVisualSlot::MainHand | EquipmentVisualSlot::OffHand => {
            sheath == SheathState::Melee
        }
        EquipmentVisualSlot::Ranged => sheath == SheathState::Ranged,
        _ => return None,
    };
    let right_hand = slot == EquipmentVisualSlot::MainHand
        || (slot == EquipmentVisualSlot::Ranged
            && matches!(inventory_type, INVTYPE_THROWN | INVTYPE_RANGED_RIGHT));
    let sheathed = match (sheathe_type, right_hand) {
        (1, true) => Some(ATTACH_RIGHT_BACK_SHEATH),
        (1, false) => Some(ATTACH_LEFT_BACK_SHEATH),
        (2, true) => Some(ATTACH_LEFT_BACK),
        (2, false) => Some(ATTACH_RIGHT_BACK),
        (3, true) => Some(ATTACH_LEFT_HIP_SHEATH),
        (3, false) => Some(ATTACH_RIGHT_HIP_SHEATH),
        (4, _) => Some(ATTACH_MIDDLE_BACK_SHEATH),
        _ => None,
    };
    if !drawn && slot == EquipmentVisualSlot::Ranged {
        return sheathed;
    }
    if !drawn && sheathed.is_some() {
        return sheathed;
    }
    Some(match (slot, inventory_type) {
        (_, INVTYPE_SHIELD) => ATTACH_LEFT_WRIST,
        (EquipmentVisualSlot::OffHand, _) | (_, INVTYPE_RANGED) => ATTACH_LEFT_PALM,
        _ => ATTACH_RIGHT_PALM,
    })
}

/// `NPCModelItemSlotDisplayInfo.ItemSlot` 11: its 1,635 rows (build 12.1.0.69933) all
/// reference ItemDisplayInfo 185704, which has no model, texture or geoset.
const ITEM_SLOT_UNDRESSED: u8 = 11;

/// `NPCModelItemSlotDisplayInfo.ItemSlot` → the visual slot it dresses.
pub fn npc_item_slot_visual_slot(item_slot: u8) -> Option<EquipmentVisualSlot> {
    Some(match item_slot {
        0 => EquipmentVisualSlot::Head,
        1 => EquipmentVisualSlot::Shoulder,
        2 => EquipmentVisualSlot::Shirt,
        3 => EquipmentVisualSlot::Chest,
        4 => EquipmentVisualSlot::Waist,
        5 => EquipmentVisualSlot::Legs,
        6 => EquipmentVisualSlot::Feet,
        7 => EquipmentVisualSlot::Wrist,
        8 => EquipmentVisualSlot::Hands,
        9 => EquipmentVisualSlot::Tabard,
        10 => EquipmentVisualSlot::Back,
        _ => return None,
    })
}

/// The DB2 rows a creature's pose and gear resolve through.
#[derive(Debug, Default)]
pub struct NpcGearData {
    /// `Emotes.ID` → `AnimID`; emotes with `AnimID` -1 play no animation and are absent.
    emote_anims: HashMap<u32, u16>,
    /// `CreatureDisplayInfo.ID` → `ExtendedDisplayInfoID` (non-zero only).
    extra_by_display: HashMap<u32, u32>,
    /// `NPCModelItemSlotDisplayInfo.NpcModelID` → (`ItemSlot`, `ItemDisplayInfoID`).
    items_by_extra: HashMap<u32, Vec<(u8, u32)>>,
    /// Product-scoped Extra and armor joins for display IDs absent from Retail.
    forever_extra_by_display: HashMap<u32, u32>,
    forever_items_by_extra: HashMap<u32, Vec<(u8, u32)>>,
    /// `Item.ID` → `SheatheType`.
    sheathe_types: HashMap<u32, u8>,
    /// `Item.ID` → `SubclassID` of weapons (`ClassID` 2).
    weapon_subclasses: HashMap<u32, u8>,
}

/// `Item.ClassID` of weapons (`ITEM_CLASS_WEAPON`).
const ITEM_CLASS_WEAPON: i64 = 2;

impl NpcGearData {
    /// `Emotes`, `CreatureDisplayInfo`, `NPCModelItemSlotDisplayInfo` and `Item` from one
    /// DB2 export directory.
    pub fn load(db2_dir: &Path) -> Result<Self, String> {
        let mut data = Self::default();
        read_rows(
            &db2_dir.join("Emotes.csv"),
            ["ID", "AnimID"],
            |[id, anim]| {
                if let Ok(anim) = u16::try_from(anim) {
                    data.emote_anims.insert(id as u32, anim);
                }
            },
        )?;
        read_rows(
            &db2_dir.join("CreatureDisplayInfo.csv"),
            ["ID", "ExtendedDisplayInfoID"],
            |[id, extra]| {
                data.extra_by_display.insert(id as u32, extra as u32);
            },
        )?;
        read_rows(
            &db2_dir.join("NPCModelItemSlotDisplayInfo.csv"),
            ["NpcModelID", "ItemSlot", "ItemDisplayInfoID"],
            |[extra, slot, display]| {
                data.items_by_extra
                    .entry(extra as u32)
                    .or_default()
                    .push((slot as u8, display as u32));
            },
        )?;
        read_rows(
            &db2_dir.join("Item.csv"),
            ["ID", "SheatheType", "ClassID", "SubclassID"],
            |[id, sheathe, class, subclass]| {
                data.sheathe_types.insert(id as u32, sheathe as u8);
                if class == ITEM_CLASS_WEAPON {
                    data.weapon_subclasses.insert(id as u32, subclass as u8);
                }
            },
        )?;
        let forever = db2_dir.parent().map(|dir| dir.join("1.60.1.70205"));
        if let Some(dir) = forever.filter(|dir| dir.join("CreatureDisplayInfo.csv").is_file()) {
            data.import_forever_armor(&dir)?;
        }
        Ok(data)
    }

    fn import_forever_armor(&mut self, dir: &Path) -> Result<(), String> {
        read_rows(
            &dir.join("CreatureDisplayInfo.csv"),
            ["ID", "ExtendedDisplayInfoID"],
            |[id, extra]| {
                if extra != 0 && !self.extra_by_display.contains_key(&(id as u32)) {
                    self.forever_extra_by_display
                        .insert(id as u32, extra as u32);
                }
            },
        )?;
        read_rows(
            &dir.join("NPCModelItemSlotDisplayInfo.csv"),
            ["NpcModelID", "ItemSlot", "ItemDisplayInfoID"],
            |[extra, slot, display]| {
                self.forever_items_by_extra
                    .entry(extra as u32)
                    .or_default()
                    .push((slot as u8, display as u32));
            },
        )
    }

    /// `Emotes.AnimID` of `emote`.
    pub fn emote_anim_id(&self, emote: u32) -> Option<u16> {
        self.emote_anims.get(&emote).copied()
    }

    /// The looping animation `pose` holds while the unit stands still (see
    /// [`unit_pose_anim_id`]).
    pub fn pose_anim_id(&self, pose: &UnitPose) -> Result<Option<u16>, String> {
        unit_pose_anim_id(pose, |emote| self.emote_anim_id(emote))
    }

    /// `Item.SubclassID` of weapon `item_id` (`ItemSubclassWeapon`), `None` for others.
    pub fn weapon_subclass(&self, item_id: u32) -> Option<u8> {
        self.weapon_subclasses.get(&item_id).copied()
    }

    /// `Item.SheatheType` of `item_id`, or `None` when Item.db2 has no such item.
    pub fn sheathe_type(&self, item_id: u32) -> Option<u8> {
        self.sheathe_types.get(&item_id).copied()
    }

    /// The display's armor as ItemDisplayInfo entries; empty without an Extra.
    pub fn display_armor(&self, display_id: u32) -> Result<EquipmentAppearance, String> {
        let (extra, rows) = if let Some(extra) = self.forever_extra_by_display.get(&display_id) {
            let rows = self.forever_items_by_extra.get(extra).ok_or_else(|| {
                format!("Forever display {display_id} Extra {extra}: missing NPCModelItemSlotDisplayInfo")
            })?;
            (extra, rows.as_slice())
        } else {
            let Some(extra) = self
                .extra_by_display
                .get(&display_id)
                .filter(|extra| **extra != 0)
            else {
                return Ok(EquipmentAppearance::default());
            };
            (
                extra,
                self.items_by_extra
                    .get(extra)
                    .map_or(&[][..], Vec::as_slice),
            )
        };
        let entries = rows
            .iter()
            .filter(|(item_slot, _)| *item_slot != ITEM_SLOT_UNDRESSED)
            .map(|&(item_slot, display_info_id)| {
                let slot = npc_item_slot_visual_slot(item_slot).ok_or_else(|| {
                    format!("display {display_id} Extra {extra}: unknown ItemSlot {item_slot}")
                })?;
                Ok(EquippedAppearanceEntry {
                    definition_source: None,
                    slot,
                    item_id: None,
                    display_info_id: Some(display_info_id),
                    inventory_type: 0,
                    hidden: false,
                })
            })
            .collect::<Result<_, String>>()?;
        Ok(EquipmentAppearance { entries })
    }

    /// The attachment virtual item `entry` renders on under `sheath`, or `None` when it
    /// is not shown (see [`virtual_item_attachment`]); an item absent from Item.db2 sits
    /// as one without a sheath position.
    pub fn virtual_item_placement(
        &self,
        entry: &EquippedAppearanceEntry,
        sheath: SheathState,
    ) -> Option<u32> {
        let sheathe_type = entry
            .item_id
            .and_then(|item| self.sheathe_type(item))
            .unwrap_or(0);
        virtual_item_attachment(entry.slot, entry.inventory_type, sheathe_type, sheath)
    }

    /// Each shown virtual item with its attachment under `sheath`.
    pub fn virtual_item_attachments(
        &self,
        items: &EquipmentAppearance,
        sheath: SheathState,
    ) -> Vec<(EquippedAppearanceEntry, u32)> {
        items
            .entries
            .iter()
            .filter_map(|entry| {
                self.virtual_item_placement(entry, sheath)
                    .map(|attachment| (entry.clone(), attachment))
            })
            .collect()
    }
}

/// Call `row` with the integer values of `columns` for each record of the numeric CSV
/// export at `path`. A quoted field is an error: these tables hold numbers only.
fn read_rows<const N: usize>(
    path: &Path,
    columns: [&str; N],
    mut row: impl FnMut([i64; N]),
) -> Result<(), String> {
    let text = std::fs::read_to_string(path)
        .map_err(|error| format!("read {}: {error}", path.display()))?;
    let mut lines = text.lines();
    let header: Vec<&str> = lines
        .next()
        .ok_or_else(|| format!("{} has no header", path.display()))?
        .split(',')
        .collect();
    let mut indexes = [0; N];
    for (index, column) in indexes.iter_mut().zip(columns) {
        *index = header
            .iter()
            .position(|name| *name == column)
            .ok_or_else(|| format!("{} has no column {column}", path.display()))?;
    }
    for line in lines.filter(|line| !line.is_empty()) {
        let fields: Vec<&str> = line.split(',').collect();
        let mut values = [0; N];
        for (value, index) in values.iter_mut().zip(indexes) {
            let field = fields
                .get(index)
                .ok_or_else(|| format!("{}: short row {line:?}", path.display()))?;
            *value = field
                .parse()
                .map_err(|error| format!("{}: bad value {field:?}: {error}", path.display()))?;
        }
        row(values);
    }
    Ok(())
}

#[cfg(test)]
#[path = "npc_gear_data_tests.rs"]
mod tests;
