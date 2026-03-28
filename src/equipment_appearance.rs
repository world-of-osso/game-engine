use std::collections::HashSet;
use std::path::{Path, PathBuf};

use shared::components::{EquipmentAppearance as NetEquipmentAppearance, EquipmentVisualSlot};

use crate::asset::casc_resolver;
use crate::equipment::{Equipment, EquipmentSlot};
use game_engine::outfit_data::{OutfitData, OutfitResult};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RuntimeModelAppearance {
    pub slot: EquipmentSlot,
    pub path: PathBuf,
    pub skin_fdids: [u32; 3],
}

#[derive(Debug, Clone, Default)]
pub struct ResolvedEquipmentAppearance {
    pub outfit: OutfitResult,
    pub runtime_models: Vec<RuntimeModelAppearance>,
    pub explicit_slots: HashSet<EquipmentVisualSlot>,
    pub hidden_character_geoset_groups: HashSet<u16>,
}

pub fn resolve_equipment_appearance(
    appearance: &NetEquipmentAppearance,
    outfit_data: &OutfitData,
    race: u8,
    sex: u8,
) -> ResolvedEquipmentAppearance {
    let mut resolved = ResolvedEquipmentAppearance::default();
    for entry in &appearance.entries {
        apply_equipment_entry(&mut resolved, entry, outfit_data, race, sex);
    }
    resolved
}

fn apply_equipment_entry(
    resolved: &mut ResolvedEquipmentAppearance,
    entry: &shared::components::EquippedAppearanceEntry,
    outfit_data: &OutfitData,
    race: u8,
    sex: u8,
) {
    resolved.explicit_slots.insert(entry.slot);
    if entry.hidden {
        return;
    }
    let Some(display_info_id) = entry.display_info_id else {
        return;
    };
    if entry.slot == EquipmentVisualSlot::Head {
        resolved
            .hidden_character_geoset_groups
            .extend(outfit_data.helmet_hide_geoset_groups(display_info_id, race));
    }
    let mut display = outfit_data.resolve_display_info(display_info_id);
    apply_slot_geoset_overrides(entry.slot, display_info_id, outfit_data, &mut display);
    resolved.outfit =
        crate::character_customization::merge_overlay_texture_sets(&resolved.outfit, &display);
    if let Some(slot) = visual_slot_to_runtime_slot(entry.slot) {
        maybe_push_runtime_model(
            resolved,
            slot,
            display_info_id,
            &display,
            outfit_data,
            race,
            sex,
        );
    }
}

fn apply_slot_geoset_overrides(
    slot: EquipmentVisualSlot,
    display_info_id: u32,
    outfit_data: &OutfitData,
    display: &mut OutfitResult,
) {
    if let Some(variant) = outfit_data.hand_geoset_variant(display_info_id) {
        match slot {
            EquipmentVisualSlot::Hands => {
                display.geoset_overrides.retain(|(group, _)| *group != 4);
                display.geoset_overrides.push((4, variant));
            }
            EquipmentVisualSlot::Feet => {
                display.geoset_overrides.retain(|(group, _)| *group != 5);
                display.geoset_overrides.push((5, variant));
            }
            _ => {}
        }
    }
}

fn maybe_push_runtime_model(
    resolved: &mut ResolvedEquipmentAppearance,
    slot: EquipmentSlot,
    display_info_id: u32,
    display: &OutfitResult,
    outfit_data: &OutfitData,
    race: u8,
    sex: u8,
) {
    if let Some((model_path, skin_fdids)) = runtime_model_for_slot(
        slot,
        display_info_id,
        display,
        outfit_data,
        race,
        sex,
    ) {
        resolved.runtime_models.push(RuntimeModelAppearance {
            slot,
            path: model_path,
            skin_fdids,
        });
    }
}

pub fn apply_runtime_equipment(equipment: &mut Equipment, resolved: &ResolvedEquipmentAppearance) {
    equipment.slots.retain(|slot, _| {
        matches!(
            slot,
            EquipmentSlot::Head | EquipmentSlot::MainHand | EquipmentSlot::OffHand
        )
    });
    equipment.slot_skin_fdids.retain(|slot, _| {
        matches!(
            slot,
            EquipmentSlot::Head | EquipmentSlot::MainHand | EquipmentSlot::OffHand
        )
    });
    for runtime_model in &resolved.runtime_models {
        equipment
            .slots
            .insert(runtime_model.slot, runtime_model.path.clone());
        equipment
            .slot_skin_fdids
            .insert(runtime_model.slot, runtime_model.skin_fdids);
    }
}

fn visual_slot_to_runtime_slot(slot: EquipmentVisualSlot) -> Option<EquipmentSlot> {
    match slot {
        EquipmentVisualSlot::Head => Some(EquipmentSlot::Head),
        EquipmentVisualSlot::MainHand => Some(EquipmentSlot::MainHand),
        EquipmentVisualSlot::OffHand => Some(EquipmentSlot::OffHand),
        _ => None,
    }
}

fn runtime_model_for_slot(
    slot: EquipmentSlot,
    display_info_id: u32,
    display: &OutfitResult,
    outfit_data: &OutfitData,
    race: u8,
    sex: u8,
) -> Option<(PathBuf, [u32; 3])> {
    match slot {
        EquipmentSlot::Head => {
            let (fdid, skin_fdids) = outfit_data.resolve_runtime_model(display_info_id, race, sex)?;
            let path = resolve_model_path(fdid)?;
            Some((path, skin_fdids))
        }
        _ => first_model_path(display).map(|path| (path, [0, 0, 0])),
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

        let resolved = resolve_equipment_appearance(&appearance, &data, 1, 0);

        assert!(resolved.runtime_models.is_empty());
    }

    #[test]
    fn head_slot_maps_to_runtime_slot() {
        assert_eq!(
            visual_slot_to_runtime_slot(EquipmentVisualSlot::Head),
            Some(EquipmentSlot::Head)
        );
    }

    #[test]
    fn apply_runtime_equipment_preserves_head_slot() {
        let mut equipment = Equipment::default();
        equipment
            .slots
            .insert(EquipmentSlot::Head, PathBuf::from("old"));
        equipment
            .slots
            .insert(EquipmentSlot::MainHand, PathBuf::from("mh"));
        equipment
            .slots
            .insert(EquipmentSlot::OffHand, PathBuf::from("oh"));
        let resolved = ResolvedEquipmentAppearance {
            runtime_models: vec![RuntimeModelAppearance {
                slot: EquipmentSlot::Head,
                path: PathBuf::from("new-head"),
                skin_fdids: [123, 0, 0],
            }],
            ..Default::default()
        };

        apply_runtime_equipment(&mut equipment, &resolved);

        assert_eq!(
            equipment.slots.get(&EquipmentSlot::Head),
            Some(&PathBuf::from("new-head"))
        );
        assert_eq!(
            equipment.slot_skin_fdids.get(&EquipmentSlot::Head),
            Some(&[123, 0, 0])
        );
    }

    #[test]
    fn first_model_path_prefers_first_available_model() {
        let display = OutfitResult {
            model_fdids: vec![],
            ..Default::default()
        };
        assert_eq!(first_model_path(&display), None);
    }

    #[test]
    fn human_male_helm_runtime_model_uses_human_variant_and_texture() {
        let data = OutfitData::load(Path::new("data"));
        let appearance = NetEquipmentAppearance {
            entries: vec![shared::components::EquippedAppearanceEntry {
                slot: EquipmentVisualSlot::Head,
                item_id: Some(1),
                display_info_id: Some(1128),
                inventory_type: 1,
                hidden: false,
            }],
        };

        let resolved = resolve_equipment_appearance(&appearance, &data, 1, 0);
        let runtime = resolved
            .runtime_models
            .iter()
            .find(|model| model.slot == EquipmentSlot::Head)
            .expect("expected head runtime model");

        assert!(runtime.path.ends_with("helm_plate_d_02_hum.m2"));
        assert_eq!(runtime.skin_fdids[0], 140455);
    }

    #[test]
    fn head_display_resolves_helmet_geoset_hide_groups() {
        let data = OutfitData::load(Path::new("data"));
        let appearance = NetEquipmentAppearance {
            entries: vec![shared::components::EquippedAppearanceEntry {
                slot: EquipmentVisualSlot::Head,
                item_id: Some(1),
                display_info_id: Some(173086),
                inventory_type: 1,
                hidden: false,
            }],
        };

        let resolved = resolve_equipment_appearance(&appearance, &data, 1, 0);

        assert!(
            resolved.hidden_character_geoset_groups.contains(&1),
            "expected vis 247 to reset human head geoset group 1: {:?}",
            resolved.hidden_character_geoset_groups
        );
        assert!(
            resolved.hidden_character_geoset_groups.contains(&2),
            "expected vis 247 to reset human head geoset group 2: {:?}",
            resolved.hidden_character_geoset_groups
        );
        assert!(
            resolved.hidden_character_geoset_groups.contains(&3),
            "expected vis 247 to reset human head geoset group 3: {:?}",
            resolved.hidden_character_geoset_groups
        );
    }

    #[test]
    fn live_helm_display_resolves_to_runtime_model_path() {
        let data = OutfitData::load(Path::new("data"));
        let display = data.resolve_display_info(1128);

        assert!(
            !display.model_fdids.is_empty(),
            "expected helm display 1128 to have model fdids"
        );

        let path = first_model_path(&display);
        assert!(
            path.is_some(),
            "expected helm display 1128 to resolve to a model path, model_fdids={:?}",
            display.model_fdids
        );
    }
}
