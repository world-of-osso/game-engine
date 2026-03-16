use std::path::Path;

use serde::{Deserialize, Serialize};
use shared::components::CharacterAppearance;

use crate::status::{CharacterStatsSnapshot, EquippedGearEntry, EquippedGearStatusSnapshot};

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
}

pub fn build_export_character_payload(
    stats: &CharacterStatsSnapshot,
    equipped_gear: &EquippedGearStatusSnapshot,
) -> Result<ExportCharacterPayload, String> {
    let character_id = stats
        .character_id
        .ok_or_else(|| "no selected character available to export".to_string())?;
    let name = required_field(stats.name.clone(), "name")?;
    let level = required_field(stats.level, "level")?;
    let race = required_field(stats.race, "race")?;
    let class = required_field(stats.class, "class")?;
    let appearance = required_field(stats.appearance, "appearance")?;

    Ok(ExportCharacterPayload {
        character_id,
        name,
        level,
        race,
        class,
        appearance,
        zone_id: stats.zone_id,
        health_current: stats.health_current,
        health_max: stats.health_max,
        mana_current: stats.mana_current,
        mana_max: stats.mana_max,
        movement_speed: stats.movement_speed,
        equipped_gear: equipped_gear.entries.clone(),
    })
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
