use std::collections::HashSet;
use std::path::{Path, PathBuf};

use shared::components::{EquipmentAppearance as NetEquipmentAppearance, EquipmentVisualSlot};

use crate::asset::asset_cache;
use crate::equipment::{Equipment, EquipmentSlot};
use game_engine::outfit_data::{self, OutfitData, OutfitResult};

#[path = "equipment_appearance_data.rs"]
pub mod equipment_appearance_data;
use equipment_appearance_data as policy;

#[cfg(test)]
#[path = "../../../tests/unit/equipment_item_tests.rs"]
mod item_tests;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RuntimeModelAppearance {
    pub slot: EquipmentSlot,
    pub path: PathBuf,
    pub skin_fdids: [u32; 3],
}

#[cfg(test)]
#[path = "../../../tests/unit/equipment_chest_tests.rs"]
mod chest_tests;
#[cfg(test)]
#[path = "../../../tests/unit/equipment_cloak_tests.rs"]
mod cloak_tests;
#[cfg(test)]
#[path = "../../../tests/unit/equipment_feet_tests.rs"]
mod feet_tests;
#[cfg(test)]
#[path = "../../../tests/unit/equipment_hands_tests.rs"]
mod hands_tests;
#[cfg(test)]
#[path = "../../../tests/unit/equipment_legs_tests.rs"]
mod legs_tests;
#[cfg(test)]
#[path = "../../../tests/unit/equipment_shoulder_tests.rs"]
mod shoulder_tests;
#[cfg(test)]
#[path = "../../../tests/unit/equipment_waist_tests.rs"]
mod waist_tests;

#[derive(Debug, Clone, Default)]
pub struct ResolvedEquipmentAppearance {
    pub outfit: OutfitResult,
    pub runtime_models: Vec<RuntimeModelAppearance>,
    pub merged_cape_texture_fdid: Option<u32>,
    pub explicit_slots: HashSet<EquipmentVisualSlot>,
    pub hidden_character_geoset_groups: HashSet<u16>,
    pub hidden_character_geoset_ids: HashSet<u16>,
}

pub fn resolve_equipment_appearance(
    appearance: &NetEquipmentAppearance,
    outfit_data: &OutfitData,
    race: u8,
    sex: u8,
) -> ResolvedEquipmentAppearance {
    resolve_equipment_appearance_with_texture_cache(
        appearance,
        outfit_data,
        race,
        sex,
        &mut |fdid| {
            let _ = asset_cache::texture(fdid);
        },
    )
}

fn resolve_equipment_appearance_with_texture_cache(
    appearance: &NetEquipmentAppearance,
    outfit_data: &OutfitData,
    race: u8,
    sex: u8,
    cache_texture: &mut dyn FnMut(u32),
) -> ResolvedEquipmentAppearance {
    let decision = policy::resolve_equipment_appearance_with_errors(
        appearance,
        outfit_data,
        race,
        sex,
        |error| bevy::log::error!("{error}"),
    );
    for &fdid in &decision.texture_fdids {
        cache_texture(fdid);
    }
    let runtime_models = decision
        .runtime_models
        .iter()
        .filter_map(|model| {
            let path = resolve_model_path(model.fdid)?;
            Some(RuntimeModelAppearance {
                slot: model.slot,
                path,
                skin_fdids: model.skin_fdids,
            })
        })
        .collect();
    ResolvedEquipmentAppearance {
        outfit: decision.outfit,
        runtime_models,
        merged_cape_texture_fdid: decision.merged_cape_texture_fdid,
        explicit_slots: decision.explicit_slots,
        hidden_character_geoset_groups: decision.hidden_character_geoset_groups,
        hidden_character_geoset_ids: decision.hidden_character_geoset_ids,
    }
}

pub fn apply_runtime_equipment(equipment: &mut Equipment, resolved: &ResolvedEquipmentAppearance) {
    for slot in resolved
        .explicit_slots
        .iter()
        .copied()
        .flat_map(visual_slot_to_runtime_slots)
    {
        equipment.slots.remove(&slot);
        equipment.slot_skin_fdids.remove(&slot);
    }
    for runtime_model in &resolved.runtime_models {
        equipment
            .slots
            .insert(runtime_model.slot, runtime_model.path.clone());
        equipment
            .slot_skin_fdids
            .insert(runtime_model.slot, runtime_model.skin_fdids);
    }
}

fn visual_slot_to_runtime_slots(slot: EquipmentVisualSlot) -> Vec<EquipmentSlot> {
    policy::visual_slot_to_runtime_slots(slot)
}

fn first_model_path(display: &OutfitResult) -> Option<PathBuf> {
    display
        .model_fdids
        .iter()
        .find_map(|(_, fdid)| resolve_model_path(*fdid))
}

fn resolve_model_path(fdid: u32) -> Option<PathBuf> {
    let wow_path = game_engine::listfile::lookup_fdid(fdid)?;
    let out_path = Path::new("data/item-models").join(wow_path);
    let path = asset_cache::file_at_path(fdid, &out_path)?;
    let _ = crate::asset::m2::ensure_primary_skin_path(&path);
    Some(path)
}

#[cfg(test)]
#[path = "../../../tests/unit/equipment_appearance_tests.rs"]
mod tests;
