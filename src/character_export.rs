use std::path::Path;

use serde::{Deserialize, Serialize};
use shared::components::{CharacterAppearance, EquipmentAppearance};
use shared::protocol::CharacterListEntry;

use crate::status::{
    CharacterStatsSnapshot, EquipmentAppearanceStatusSnapshot, EquippedGearEntry,
    EquippedGearStatusSnapshot,
};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ExportCharacterPayload {
    pub character_id: u64,
    pub name: String,
    pub level: u16,
    pub race: u8,
    pub class: u8,
    pub appearance: CharacterAppearance,
    pub zone_id: u32,
    pub health_current: Option<f32>,
    pub health_max: Option<f32>,
    pub mana_current: Option<f32>,
    pub mana_max: Option<f32>,
    pub movement_speed: Option<f32>,
    pub equipped_gear: Vec<EquippedGearEntry>,
    pub equipment_appearance: EquipmentAppearance,
}

pub fn build_export_character_payload(
    stats: &CharacterStatsSnapshot,
    equipped_gear: &EquippedGearStatusSnapshot,
    equipment_appearance: &EquipmentAppearanceStatusSnapshot,
    character_list: &[CharacterListEntry],
    requested_name: Option<&str>,
    requested_character_id: Option<u64>,
) -> Result<ExportCharacterPayload, String> {
    let identity = resolve_export_identity(
        stats,
        character_list,
        requested_name,
        requested_character_id,
    )?;

    Ok(ExportCharacterPayload {
        character_id: identity.character_id,
        name: identity.name,
        level: identity.level,
        race: identity.race,
        class: identity.class,
        appearance: identity.appearance,
        zone_id: stats.zone_id,
        health_current: stats.health_current,
        health_max: stats.health_max,
        mana_current: stats.mana_current,
        mana_max: stats.mana_max,
        movement_speed: stats.movement_speed,
        equipped_gear: equipped_gear.entries.clone(),
        equipment_appearance: identity
            .equipment_appearance
            .unwrap_or_else(|| equipment_appearance.appearance.clone()),
    })
}

fn resolve_export_identity(
    stats: &CharacterStatsSnapshot,
    character_list: &[CharacterListEntry],
    requested_name: Option<&str>,
    requested_character_id: Option<u64>,
) -> Result<ExportIdentity, String> {
    if let Some(character_id) = requested_character_id {
        let entry = character_list
            .iter()
            .find(|entry| entry.character_id == character_id)
            .ok_or_else(|| format!("character id {character_id} not found"))?;
        return Ok(ExportIdentity::from_entry(entry));
    }

    if let Some(name) = requested_name {
        let entry = character_list
            .iter()
            .find(|entry| entry.name.eq_ignore_ascii_case(name))
            .ok_or_else(|| format!("character '{name}' not found"))?;
        return Ok(ExportIdentity::from_entry(entry));
    }

    Ok(ExportIdentity {
        character_id: stats
            .character_id
            .ok_or_else(|| "no selected character available to export".to_string())?,
        name: required_field(stats.name.clone(), "name")?,
        level: required_field(stats.level, "level")?,
        race: required_field(stats.race, "race")?,
        class: required_field(stats.class, "class")?,
        appearance: required_field(stats.appearance, "appearance")?,
        equipment_appearance: None,
    })
}

struct ExportIdentity {
    character_id: u64,
    name: String,
    level: u16,
    race: u8,
    class: u8,
    appearance: CharacterAppearance,
    equipment_appearance: Option<EquipmentAppearance>,
}

impl ExportIdentity {
    fn from_entry(entry: &CharacterListEntry) -> Self {
        Self {
            character_id: entry.character_id,
            name: entry.name.clone(),
            level: entry.level,
            race: entry.race,
            class: entry.class,
            appearance: entry.appearance,
            equipment_appearance: Some(entry.equipment_appearance.clone()),
        }
    }
}

pub fn write_export_character_file(
    output_path: &Path,
    payload: &ExportCharacterPayload,
) -> Result<(), String> {
    if let Some(parent) = output_path.parent() {
        std::fs::create_dir_all(parent)
            .map_err(|e| format!("failed to create {}: {e}", parent.display()))?;
    }
    let serialized = serde_json::to_string_pretty(payload)
        .map_err(|e| format!("failed to encode export payload: {e}"))?;
    std::fs::write(output_path, serialized)
        .map_err(|e| format!("failed to write {}: {e}", output_path.display()))?;
    Ok(())
}

fn required_field<T>(value: Option<T>, field: &str) -> Result<T, String> {
    value.ok_or_else(|| format!("selected character is missing {field}"))
}
