//! GDScript-facing player requests: a named character built through the same
//! loader the world, character select and creation use, from plain dictionaries.

use std::{
    path::{Path, PathBuf},
    sync::OnceLock,
};

use godot::{classes::ProjectSettings, prelude::*};
use shared::components::{
    CharacterAppearance, EquipmentAppearance, EquipmentVisualSlot, EquippedAppearanceEntry, Player,
    SheathState,
};

use crate::{npc_gear_data::NpcGearData, world_models};
use game_engine_core::npc_appearance_assets::load_customization_db;

pub(super) fn data_root() -> PathBuf {
    PathBuf::from(
        ProjectSettings::singleton()
            .globalize_path("res://../data")
            .to_string(),
    )
}

/// The player `race`/`sex`/`class` with the default appearance of that sex.
pub(super) fn player(race: i64, sex: i64, class: i64) -> Result<Player, String> {
    let byte = |name: &str, value: i64| {
        u8::try_from(value).map_err(|_| format!("Invalid player {name} {value}"))
    };
    Ok(Player {
        name: "Fixture".into(),
        race: byte("race", race)?,
        class: byte("class", class)?,
        appearance: CharacterAppearance {
            sex: byte("sex", sex)?,
            ..CharacterAppearance::default()
        },
    })
}

/// `player` with each option of `choices` (option ID to choice ID) selected.
pub(super) fn customized(mut player: Player, choices: &VarDictionary) -> Result<Player, String> {
    let db = load_customization_db(&data_root())?;
    for (option, choice) in choices.iter_shared() {
        let id = |value: &Variant| {
            value
                .try_to::<i64>()
                .ok()
                .and_then(|id| u32::try_from(id).ok())
                .ok_or_else(|| format!("Customization {option}: {choice} is not an ID"))
        };
        crate::appearance_options::set_choice(
            &db,
            player.race,
            player.appearance.sex,
            player.class,
            &mut player.appearance,
            id(&option)?,
            id(&choice)?,
        )?;
    }
    Ok(player)
}

/// Equipment from dictionaries `{slot, item_id?, display_id?, inventory_type?, hidden?}`;
/// `slot` is an `EquipmentVisualSlot` name such as `"MainHand"`.
pub(super) fn equipment(items: &VarArray) -> Result<EquipmentAppearance, String> {
    let entries = items
        .iter_shared()
        .map(|item| {
            let item = item
                .try_to::<VarDictionary>()
                .map_err(|_| format!("Equipment entry {item} is not a dictionary"))?;
            equipment_entry(&item)
        })
        .collect::<Result<_, String>>()?;
    Ok(EquipmentAppearance { entries })
}

fn equipment_entry(item: &VarDictionary) -> Result<EquippedAppearanceEntry, String> {
    let slot_name = item
        .get("slot")
        .and_then(|slot| slot.try_to::<GString>().ok())
        .ok_or_else(|| format!("Equipment entry {item} has no slot name"))?
        .to_string();
    let id = |key: &str| -> Result<Option<u32>, String> {
        item.get(key)
            .map(|value| {
                value
                    .try_to::<i64>()
                    .ok()
                    .and_then(|value| u32::try_from(value).ok())
                    .ok_or_else(|| format!("Equipment {slot_name} {key} {value} is not an ID"))
            })
            .transpose()
    };
    let inventory_type = id("inventory_type")?.unwrap_or(0);
    let item_id = id("item_id")?;
    let definition_source = item_id.map(|_| shared::item_data::ItemDefinitionSource::Retail);
    Ok(EquippedAppearanceEntry {
        definition_source,
        slot: visual_slot(&slot_name)?,
        item_id,
        display_info_id: id("display_id")?,
        inventory_type: u8::try_from(inventory_type)
            .map_err(|_| format!("Equipment {slot_name} inventory type {inventory_type}"))?,
        hidden: item
            .get("hidden")
            .is_some_and(|hidden| hidden.try_to::<bool>().unwrap_or(false)),
    })
}

fn visual_slot(name: &str) -> Result<EquipmentVisualSlot, String> {
    use EquipmentVisualSlot::*;
    Ok(match name {
        "Head" => Head,
        "Shoulder" => Shoulder,
        "Back" => Back,
        "Chest" => Chest,
        "Shirt" => Shirt,
        "Tabard" => Tabard,
        "Wrist" => Wrist,
        "Hands" => Hands,
        "Waist" => Waist,
        "Legs" => Legs,
        "Feet" => Feet,
        "MainHand" => MainHand,
        "OffHand" => OffHand,
        "Ranged" => Ranged,
        _ => return Err(format!("Unknown equipment slot {name}")),
    })
}

pub(super) fn sheath_state(value: i64) -> Result<SheathState, String> {
    Ok(match value {
        0 => SheathState::Unarmed,
        1 => SheathState::Melee,
        2 => SheathState::Ranged,
        _ => return Err(format!("Unknown sheath state {value}")),
    })
}

/// Move `model`'s weapons to where `sheath` places them, as a world player's are.
pub(super) fn place_player_weapons(
    model: &Gd<godot::classes::Node3D>,
    equipment: &EquipmentAppearance,
    sheath: SheathState,
) -> Result<(), String> {
    let placements = world_models::player_weapon_placements(gear(&data_root())?, equipment, sheath);
    world_models::place_items(model, &placements)
}

/// The build-pinned DB2 pose and gear rows, read once per process.
fn gear(data_root: &Path) -> Result<&'static NpcGearData, String> {
    static GEAR: OnceLock<Result<NpcGearData, String>> = OnceLock::new();
    GEAR.get_or_init(|| NpcGearData::load(&data_root.join("db2/12.1.0.69933")))
        .as_ref()
        .map_err(Clone::clone)
}
