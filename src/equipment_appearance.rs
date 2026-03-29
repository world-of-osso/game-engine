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

#[derive(Debug, Clone, Default, PartialEq, Eq)]
struct HeadAppearanceEffects {
    hidden_geoset_groups: Vec<u16>,
    geoset_overrides: Vec<(u16, u16)>,
    runtime_model: Option<(PathBuf, [u32; 3])>,
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
    match entry.slot {
        EquipmentVisualSlot::Head => {
            apply_head_equipment_entry(resolved, display_info_id, outfit_data, race, sex)
        }
        _ => apply_non_head_equipment_entry(resolved, entry.slot, display_info_id, outfit_data, race, sex),
    }
}

fn apply_head_equipment_entry(
    resolved: &mut ResolvedEquipmentAppearance,
    display_info_id: u32,
    outfit_data: &OutfitData,
    race: u8,
    sex: u8,
) {
    let mut display = outfit_data.resolve_display_info(display_info_id);
    let head = resolve_head_appearance_effects(display_info_id, outfit_data, race, sex);
    resolved
        .hidden_character_geoset_groups
        .extend(head.hidden_geoset_groups);
    apply_geoset_overrides(&mut display, head.geoset_overrides);
    resolved.outfit =
        crate::character_customization::merge_overlay_texture_sets(&resolved.outfit, &display);
    if let Some((path, skin_fdids)) = head.runtime_model {
        resolved.runtime_models.push(RuntimeModelAppearance {
            slot: EquipmentSlot::Head,
            path,
            skin_fdids,
        });
    }
}

fn apply_non_head_equipment_entry(
    resolved: &mut ResolvedEquipmentAppearance,
    slot: EquipmentVisualSlot,
    display_info_id: u32,
    outfit_data: &OutfitData,
    race: u8,
    sex: u8,
) {
    let mut display = outfit_data.resolve_display_info(display_info_id);
    apply_slot_geoset_overrides(slot, display_info_id, outfit_data, &mut display);
    resolved.outfit =
        crate::character_customization::merge_overlay_texture_sets(&resolved.outfit, &display);
    if let Some(runtime_slot) = visual_slot_to_runtime_slot(slot) {
        maybe_push_runtime_model(
            resolved,
            runtime_slot,
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
                apply_geoset_overrides(display, vec![(4, variant)]);
            }
            EquipmentVisualSlot::Feet => {
                apply_geoset_overrides(display, vec![(5, variant)]);
            }
            _ => {}
        }
    }
}

fn apply_geoset_overrides(display: &mut OutfitResult, overrides: Vec<(u16, u16)>) {
    for (group, value) in overrides {
        display
            .geoset_overrides
            .retain(|(existing_group, _)| *existing_group != group);
        display.geoset_overrides.push((group, value));
    }
}

fn resolve_head_appearance_effects(
    display_info_id: u32,
    outfit_data: &OutfitData,
    race: u8,
    sex: u8,
) -> HeadAppearanceEffects {
    let hidden_geoset_groups = outfit_data.helmet_hide_geoset_groups(display_info_id, race);
    let geoset_overrides = outfit_data.head_geoset_overrides(display_info_id);
    let runtime_model = outfit_data
        .resolve_runtime_model(display_info_id, race, sex)
        .and_then(|(fdid, skin_fdids)| {
            let path = resolve_model_path(fdid)?;
            (!is_collection_head_model(&path)).then_some((path, skin_fdids))
        });
    HeadAppearanceEffects {
        hidden_geoset_groups,
        geoset_overrides,
        runtime_model,
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
            if is_collection_head_model(&path) {
                return None;
            }
            Some((path, skin_fdids))
        }
        _ => first_model_path(display).map(|path| (path, [0, 0, 0])),
    }
}

fn is_collection_head_model(path: &Path) -> bool {
    let lower = path.to_string_lossy().to_ascii_lowercase();
    lower.contains("item/objectcomponents/collections/")
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
    fn real_mask_display_hides_scalp_and_enables_head_geosets() {
        let data = OutfitData::load(Path::new("data"));
        let appearance = NetEquipmentAppearance {
            entries: vec![shared::components::EquippedAppearanceEntry {
                slot: EquipmentVisualSlot::Head,
                item_id: Some(249913),
                display_info_id: Some(720086),
                inventory_type: 1,
                hidden: false,
            }],
        };

        let resolved = resolve_equipment_appearance(&appearance, &data, 1, 0);

        assert!(
            resolved.hidden_character_geoset_groups.contains(&0),
            "expected vis 644/645 to hide scalp hair group 0: {:?}",
            resolved.hidden_character_geoset_groups
        );
        assert!(
            resolved.hidden_character_geoset_groups.contains(&7),
            "expected vis 644/645 to hide ear-adjacent group 7: {:?}",
            resolved.hidden_character_geoset_groups
        );
        assert!(
            resolved.outfit.geoset_overrides.contains(&(27, 2)),
            "expected equipped head slot to switch character head geoset to 2702: {:?}",
            resolved.outfit.geoset_overrides
        );
        assert!(
            !resolved.outfit.geoset_overrides.iter().any(|(group, _)| *group == 21),
            "expected GeosetGroup_1 == 0 to avoid emitting a 21xx override: {:?}",
            resolved.outfit.geoset_overrides
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

    #[test]
    fn real_mask_display_does_not_fallback_to_blood_elf_collection_model() {
        let data = OutfitData::load(Path::new("data"));
        let appearance = NetEquipmentAppearance {
            entries: vec![shared::components::EquippedAppearanceEntry {
                slot: EquipmentVisualSlot::Head,
                item_id: Some(249913),
                display_info_id: Some(720086),
                inventory_type: 1,
                hidden: false,
            }],
        };

        let resolved = resolve_equipment_appearance(&appearance, &data, 1, 0);
        let Some(runtime) = resolved
            .runtime_models
            .iter()
            .find(|model| model.slot == EquipmentSlot::Head)
        else {
            return;
        };

        let path = runtime.path.to_string_lossy().to_ascii_lowercase();
        assert!(
            !path.contains("_be_m.m2"),
            "head runtime model should not fall back to blood-elf male for human male displays: {}",
            runtime.path.display()
        );
        assert!(
            path.contains("_hu_m.m2") || path.contains("_hum.m2"),
            "head runtime model should resolve to a human-male variant when present: {}",
            runtime.path.display()
        );
    }

    #[test]
    fn real_mask_display_skips_collection_runtime_head_model() {
        let data = OutfitData::load(Path::new("data"));
        let appearance = NetEquipmentAppearance {
            entries: vec![shared::components::EquippedAppearanceEntry {
                slot: EquipmentVisualSlot::Head,
                item_id: Some(249913),
                display_info_id: Some(720086),
                inventory_type: 1,
                hidden: false,
            }],
        };

        let resolved = resolve_equipment_appearance(&appearance, &data, 1, 0);

        assert!(
            resolved.runtime_models.iter().all(|model| model.slot != EquipmentSlot::Head),
            "collection-style head displays should not spawn runtime head attachments: {:?}",
            resolved.runtime_models
        );
    }
}
