use std::collections::HashSet;
use std::path::{Path, PathBuf};

use shared::components::{EquipmentAppearance as NetEquipmentAppearance, EquipmentVisualSlot};

use crate::asset::casc_resolver;
use crate::equipment::{Equipment, EquipmentSlot};
use game_engine::outfit_data::{OutfitData, OutfitResult};

#[derive(Debug, Clone, Default)]
pub struct ResolvedEquipmentAppearance {
    pub outfit: OutfitResult,
    pub runtime_models: Vec<(EquipmentSlot, PathBuf)>,
    pub explicit_slots: HashSet<EquipmentVisualSlot>,
}

pub fn resolve_equipment_appearance(
    appearance: &NetEquipmentAppearance,
    outfit_data: &OutfitData,
) -> ResolvedEquipmentAppearance {
    let mut resolved = ResolvedEquipmentAppearance::default();
    for entry in &appearance.entries {
        resolved.explicit_slots.insert(entry.slot);
        if entry.hidden {
            continue;
        }
        let Some(display_info_id) = entry.display_info_id else {
            continue;
        };
        let mut display = outfit_data.resolve_display_info(display_info_id);
        if entry.slot == EquipmentVisualSlot::Hands
            && let Some(variant) = outfit_data.hand_geoset_variant(display_info_id)
        {
            display.geoset_overrides.retain(|(group, _)| *group != 4);
            display.geoset_overrides.push((4, variant));
        }
        resolved.outfit =
            crate::character_customization::merge_overlay_texture_sets(&resolved.outfit, &display);

        if let Some(slot) = visual_slot_to_runtime_slot(entry.slot) {
            if let Some(model_path) = first_model_path(&display) {
                resolved.runtime_models.push((slot, model_path));
            }
        }
    }
    resolved
}

pub fn apply_runtime_equipment(equipment: &mut Equipment, resolved: &ResolvedEquipmentAppearance) {
    equipment
        .slots
        .retain(|slot, _| matches!(slot, EquipmentSlot::MainHand | EquipmentSlot::OffHand));
    for (slot, path) in &resolved.runtime_models {
        equipment.slots.insert(*slot, path.clone());
    }
}

fn visual_slot_to_runtime_slot(slot: EquipmentVisualSlot) -> Option<EquipmentSlot> {
    match slot {
        EquipmentVisualSlot::MainHand => Some(EquipmentSlot::MainHand),
        EquipmentVisualSlot::OffHand => Some(EquipmentSlot::OffHand),
        _ => None,
    }
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
    casc_resolver::ensure_file_at_path(fdid, &out_path)
}

#[cfg(test)]
mod tests {
    use super::*;
    use game_engine::outfit_data::OutfitResult;

    #[test]
    fn ignores_non_attachment_slots_for_runtime_models() {
        let appearance = NetEquipmentAppearance {
            entries: vec![shared::components::EquippedAppearanceEntry {
                slot: EquipmentVisualSlot::Chest,
                item_id: Some(1),
                display_info_id: Some(1),
                inventory_type: 5,
                hidden: false,
            }],
        };
        let data = OutfitData::default();

        let resolved = resolve_equipment_appearance(&appearance, &data);

        assert!(resolved.runtime_models.is_empty());
    }

    #[test]
    fn first_model_path_prefers_first_available_model() {
        let display = OutfitResult {
            model_fdids: vec![],
            ..Default::default()
        };
        assert_eq!(first_model_path(&display), None);
    }
}
