//! What a replicated creature holds and how it stands, from the build-pinned DB2
//! exports: its `UnitPose` (TrinityCore `UnitStandStateType`, `SheathState`,
//! `UnitData::EmoteState`), its virtual items (`EquipmentAppearance`) and the armor its
//! display authors (`NPCModelItemSlotDisplayInfo` of its `CreatureDisplayInfoExtra`).

use std::collections::HashMap;
use std::path::Path;
use std::sync::OnceLock;

use shared::components::{
    EquipmentAppearance, EquipmentVisualSlot, EquippedAppearanceEntry, SheathState, StandState,
    UnitPose,
};

use crate::spell_catalog::SPELL_DB2_BUILD;
use crate::spell_catalog::csv_records::CsvTable;

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

/// `Emotes.AnimID` of `emote`; the table loads on first use.
pub fn emote_anim_id(emote: u32) -> Option<u16> {
    static EMOTES: OnceLock<HashMap<u32, u16>> = OnceLock::new();
    EMOTES
        .get_or_init(|| {
            let path = db2_path("Emotes.csv");
            load_emote_anims(&path).unwrap_or_else(|error| {
                bevy::log::error!("{error}");
                HashMap::new()
            })
        })
        .get(&emote)
        .copied()
}

/// Emotes with `AnimID` -1 play no animation and are left out.
fn load_emote_anims(path: &Path) -> Result<HashMap<u32, u16>, String> {
    let table = CsvTable::read(path)?;
    let (id, anim) = (table.column("ID")?, table.column("AnimID")?);
    let mut anims = HashMap::new();
    for record in table.records() {
        let anim_id: i32 = number(&record, anim, path)?;
        if let Ok(anim_id) = u16::try_from(anim_id) {
            anims.insert(number(&record, id, path)?, anim_id);
        }
    }
    Ok(anims)
}

// WMVx `AttachmentPosition` / M2 attachment IDs.
const ATTACH_LEFT_WRIST: u32 = 0;
const ATTACH_RIGHT_PALM: u32 = 1;
const ATTACH_LEFT_PALM: u32 = 2;
const ATTACH_LEFT_BACK_SHEATH: u32 = 27;
const ATTACH_MIDDLE_BACK_SHEATH: u32 = 28;
const ATTACH_LEFT_BACK: u32 = 30;
const ATTACH_LEFT_HIP_SHEATH: u32 = 32;
const ATTACH_RIGHT_HIP_SHEATH: u32 = 33;

// `Item.InventoryType`.
const INVTYPE_SHIELD: u8 = 14;
const INVTYPE_RANGED: u8 = 15;

/// The attachment a creature virtual item renders on, or `None` when it is not shown.
/// Main and off hand are drawn with `SheathState::Melee`, the ranged item with
/// `SheathState::Ranged`; a shield is drawn on the left wrist, a bow in the left hand.
/// Otherwise the item sits at its `Item.SheatheType` position (WMVx
/// `Mapping::sheathTypeAttachmentPosition`: 1 back, 2 large weapon back, 3 hip, 4 shield
/// back); a hand item without one stays in the hand, a ranged item without one is not
/// shown.
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
    let sheathed = match sheathe_type {
        1 => Some(ATTACH_LEFT_BACK_SHEATH),
        2 => Some(ATTACH_LEFT_BACK),
        3 if slot == EquipmentVisualSlot::OffHand => Some(ATTACH_RIGHT_HIP_SHEATH),
        3 => Some(ATTACH_LEFT_HIP_SHEATH),
        4 => Some(ATTACH_MIDDLE_BACK_SHEATH),
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

/// Creature display → `CreatureDisplayInfoExtra` → its authored armor.
#[derive(Debug, Default)]
pub struct NpcItemSlots {
    extra_by_display: HashMap<u32, u32>,
    items_by_extra: HashMap<u32, Vec<(u8, u32)>>,
}

impl NpcItemSlots {
    pub(crate) fn from_tables(
        display_info: &CsvTable,
        item_slots: &CsvTable,
    ) -> Result<Self, String> {
        let mut slots = Self::default();
        let path = display_info.path();
        let (id, extra) = (
            display_info.column("ID")?,
            display_info.column("ExtendedDisplayInfoID")?,
        );
        for record in display_info.records() {
            let extra_id: u32 = number(&record, extra, path)?;
            if extra_id != 0 {
                slots
                    .extra_by_display
                    .insert(number(&record, id, path)?, extra_id);
            }
        }
        let path = item_slots.path();
        let (model, display, slot) = (
            item_slots.column("NpcModelID")?,
            item_slots.column("ItemDisplayInfoID")?,
            item_slots.column("ItemSlot")?,
        );
        for record in item_slots.records() {
            slots
                .items_by_extra
                .entry(number(&record, model, path)?)
                .or_default()
                .push((
                    number(&record, slot, path)?,
                    number(&record, display, path)?,
                ));
        }
        Ok(slots)
    }

    /// The display's armor as ItemDisplayInfo entries; empty without an Extra.
    pub fn appearance(&self, display_id: u32) -> Result<EquipmentAppearance, String> {
        let Some(extra) = self.extra_by_display.get(&display_id) else {
            return Ok(EquipmentAppearance::default());
        };
        let rows = self
            .items_by_extra
            .get(extra)
            .map_or(&[][..], Vec::as_slice);
        let entries = rows
            .iter()
            .map(|&(item_slot, display_info_id)| {
                let slot = npc_item_slot_visual_slot(item_slot).ok_or_else(|| {
                    format!("display {display_id} Extra {extra}: unknown ItemSlot {item_slot}")
                })?;
                Ok(EquippedAppearanceEntry {
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
}

/// The pinned exports' item slots; they load on first use.
pub fn npc_item_slots() -> &'static NpcItemSlots {
    static SLOTS: OnceLock<NpcItemSlots> = OnceLock::new();
    SLOTS.get_or_init(|| {
        let tables = CsvTable::read(&db2_path("CreatureDisplayInfo.csv")).and_then(|display| {
            let slots = CsvTable::read(&db2_path("NPCModelItemSlotDisplayInfo.csv"))?;
            NpcItemSlots::from_tables(&display, &slots)
        });
        tables.unwrap_or_else(|error| {
            bevy::log::error!("NPC item slots unavailable: {error}");
            NpcItemSlots::default()
        })
    })
}

fn db2_path(file: &str) -> std::path::PathBuf {
    crate::paths::resolve_data_path(Path::new("db2").join(SPELL_DB2_BUILD).join(file))
}

fn number<T: std::str::FromStr>(
    record: &[std::borrow::Cow<'_, str>],
    index: usize,
    path: &Path,
) -> Result<T, String>
where
    T::Err: std::fmt::Display,
{
    let value = record
        .get(index)
        .ok_or_else(|| format!("{}: short row {record:?}", path.display()))?;
    value
        .parse()
        .map_err(|error| format!("{}: bad value {value:?}: {error}", path.display()))
}

#[cfg(test)]
#[path = "npc_gear_data_tests.rs"]
mod tests;
