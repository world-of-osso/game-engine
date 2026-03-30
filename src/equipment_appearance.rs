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

#[cfg(test)]
#[path = "equipment_cloak_tests.rs"]
mod cloak_tests;

#[cfg(test)]
#[path = "equipment_chest_tests.rs"]
mod chest_tests;

#[cfg(test)]
#[path = "equipment_feet_tests.rs"]
mod feet_tests;

#[cfg(test)]
#[path = "equipment_hands_tests.rs"]
mod hands_tests;

#[cfg(test)]
#[path = "equipment_legs_tests.rs"]
mod legs_tests;

#[cfg(test)]
#[path = "equipment_waist_tests.rs"]
mod waist_tests;

#[cfg(test)]
#[path = "equipment_shoulder_tests.rs"]
mod shoulder_tests;

#[derive(Debug, Clone, Default)]
pub struct ResolvedEquipmentAppearance {
    pub outfit: OutfitResult,
    pub runtime_models: Vec<RuntimeModelAppearance>,
    pub merged_cape_texture_fdid: Option<u32>,
    pub explicit_slots: HashSet<EquipmentVisualSlot>,
    pub hidden_character_geoset_groups: HashSet<u16>,
    pub hidden_character_geoset_ids: HashSet<u16>,
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
    apply_visible_equipment_entry(
        resolved,
        entry.slot,
        display_info_id,
        outfit_data,
        race,
        sex,
    );
}

fn apply_visible_equipment_entry(
    resolved: &mut ResolvedEquipmentAppearance,
    slot: EquipmentVisualSlot,
    display_info_id: u32,
    outfit_data: &OutfitData,
    race: u8,
    sex: u8,
) {
    match slot {
        EquipmentVisualSlot::Head => {
            apply_head_equipment_entry(resolved, display_info_id, outfit_data, race, sex)
        }
        EquipmentVisualSlot::Shoulder => {
            apply_shoulder_equipment_entry(resolved, display_info_id, outfit_data, race, sex)
        }
        EquipmentVisualSlot::Back => {
            apply_back_equipment_entry(resolved, display_info_id, outfit_data, race, sex)
        }
        EquipmentVisualSlot::Chest => {
            apply_chest_equipment_entry(resolved, display_info_id, outfit_data, race, sex)
        }
        EquipmentVisualSlot::Shirt => {
            apply_shirt_equipment_entry(resolved, display_info_id, outfit_data, race, sex)
        }
        EquipmentVisualSlot::Tabard => {
            apply_tabard_equipment_entry(resolved, display_info_id, outfit_data, race, sex)
        }
        EquipmentVisualSlot::Wrist => {
            apply_wrist_equipment_entry(resolved, display_info_id, outfit_data, race, sex)
        }
        EquipmentVisualSlot::Hands => {
            apply_hands_equipment_entry(resolved, display_info_id, outfit_data, race, sex)
        }
        EquipmentVisualSlot::Waist => {
            apply_waist_equipment_entry(resolved, display_info_id, outfit_data, race, sex)
        }
        EquipmentVisualSlot::Legs => {
            apply_legs_equipment_entry(resolved, display_info_id, outfit_data, race, sex)
        }
        EquipmentVisualSlot::Feet => {
            apply_feet_equipment_entry(resolved, display_info_id, outfit_data, race, sex)
        }
        EquipmentVisualSlot::MainHand => {
            apply_main_hand_equipment_entry(resolved, display_info_id, outfit_data, race, sex)
        }
        EquipmentVisualSlot::OffHand => {
            apply_off_hand_equipment_entry(resolved, display_info_id, outfit_data, race, sex)
        }
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
    let has_vis_data = !head.hidden_geoset_groups.is_empty();
    resolved
        .hidden_character_geoset_groups
        .extend(head.hidden_geoset_groups);
    apply_geoset_overrides(&mut display, head.geoset_overrides);
    resolved.outfit =
        crate::character_customization::merge_overlay_texture_sets(&resolved.outfit, &display);
    if let Some((path, skin_fdids)) = head.runtime_model {
        // Old helmets without HelmetGeosetVisData hide hair by default.
        // Modern items (tiaras, circlets) that show hair have vis data
        // with permissive flags.
        if !has_vis_data {
            resolved.hidden_character_geoset_groups.insert(0);
        }
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
    ensure_item_component_textures(&display);
    apply_slot_geoset_overrides(slot, display_info_id, outfit_data, &mut display);
    resolved.outfit =
        crate::character_customization::merge_overlay_texture_sets(&resolved.outfit, &display);
    for runtime_slot in visual_slot_to_runtime_slots(slot) {
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

fn apply_shoulder_equipment_entry(
    resolved: &mut ResolvedEquipmentAppearance,
    display_info_id: u32,
    outfit_data: &OutfitData,
    race: u8,
    sex: u8,
) {
    apply_non_head_equipment_entry(
        resolved,
        EquipmentVisualSlot::Shoulder,
        display_info_id,
        outfit_data,
        race,
        sex,
    );
}

fn apply_back_equipment_entry(
    resolved: &mut ResolvedEquipmentAppearance,
    display_info_id: u32,
    outfit_data: &OutfitData,
    race: u8,
    sex: u8,
) {
    if let Some(cape_texture_fdid) = outfit_data.cape_texture_fdid(display_info_id) {
        let _ = casc_resolver::ensure_texture(cape_texture_fdid);
        resolved.merged_cape_texture_fdid = Some(cape_texture_fdid);
    }
    apply_non_head_equipment_entry(
        resolved,
        EquipmentVisualSlot::Back,
        display_info_id,
        outfit_data,
        race,
        sex,
    );
}

fn apply_chest_equipment_entry(
    resolved: &mut ResolvedEquipmentAppearance,
    display_info_id: u32,
    outfit_data: &OutfitData,
    race: u8,
    sex: u8,
) {
    apply_non_head_equipment_entry(
        resolved,
        EquipmentVisualSlot::Chest,
        display_info_id,
        outfit_data,
        race,
        sex,
    );
}

fn apply_shirt_equipment_entry(
    resolved: &mut ResolvedEquipmentAppearance,
    display_info_id: u32,
    outfit_data: &OutfitData,
    race: u8,
    sex: u8,
) {
    apply_non_head_equipment_entry(
        resolved,
        EquipmentVisualSlot::Shirt,
        display_info_id,
        outfit_data,
        race,
        sex,
    );
}

fn apply_tabard_equipment_entry(
    resolved: &mut ResolvedEquipmentAppearance,
    display_info_id: u32,
    outfit_data: &OutfitData,
    race: u8,
    sex: u8,
) {
    apply_non_head_equipment_entry(
        resolved,
        EquipmentVisualSlot::Tabard,
        display_info_id,
        outfit_data,
        race,
        sex,
    );
}

fn apply_wrist_equipment_entry(
    resolved: &mut ResolvedEquipmentAppearance,
    display_info_id: u32,
    outfit_data: &OutfitData,
    race: u8,
    sex: u8,
) {
    apply_non_head_equipment_entry(
        resolved,
        EquipmentVisualSlot::Wrist,
        display_info_id,
        outfit_data,
        race,
        sex,
    );
}

fn apply_hands_equipment_entry(
    resolved: &mut ResolvedEquipmentAppearance,
    display_info_id: u32,
    outfit_data: &OutfitData,
    race: u8,
    sex: u8,
) {
    apply_non_head_equipment_entry(
        resolved,
        EquipmentVisualSlot::Hands,
        display_info_id,
        outfit_data,
        race,
        sex,
    );
}

fn apply_waist_equipment_entry(
    resolved: &mut ResolvedEquipmentAppearance,
    display_info_id: u32,
    outfit_data: &OutfitData,
    race: u8,
    sex: u8,
) {
    let before_runtime = resolved.runtime_models.len();
    let before_geosets = resolved.outfit.geoset_overrides.len();
    let before_textures = resolved.outfit.item_textures.len();
    apply_non_head_equipment_entry(
        resolved,
        EquipmentVisualSlot::Waist,
        display_info_id,
        outfit_data,
        race,
        sex,
    );
    eprintln!(
        "waist display {} resolved: new_item_textures={:?} new_geosets={:?} new_runtime_models={:?}",
        display_info_id,
        &resolved.outfit.item_textures[before_textures..],
        &resolved.outfit.geoset_overrides[before_geosets..],
        &resolved.runtime_models[before_runtime..]
    );
}

fn apply_legs_equipment_entry(
    resolved: &mut ResolvedEquipmentAppearance,
    display_info_id: u32,
    outfit_data: &OutfitData,
    race: u8,
    sex: u8,
) {
    apply_non_head_equipment_entry(
        resolved,
        EquipmentVisualSlot::Legs,
        display_info_id,
        outfit_data,
        race,
        sex,
    );
}

fn apply_feet_equipment_entry(
    resolved: &mut ResolvedEquipmentAppearance,
    display_info_id: u32,
    outfit_data: &OutfitData,
    race: u8,
    sex: u8,
) {
    apply_non_head_equipment_entry(
        resolved,
        EquipmentVisualSlot::Feet,
        display_info_id,
        outfit_data,
        race,
        sex,
    );
}

fn apply_main_hand_equipment_entry(
    resolved: &mut ResolvedEquipmentAppearance,
    display_info_id: u32,
    outfit_data: &OutfitData,
    race: u8,
    sex: u8,
) {
    apply_non_head_equipment_entry(
        resolved,
        EquipmentVisualSlot::MainHand,
        display_info_id,
        outfit_data,
        race,
        sex,
    );
}

fn apply_off_hand_equipment_entry(
    resolved: &mut ResolvedEquipmentAppearance,
    display_info_id: u32,
    outfit_data: &OutfitData,
    race: u8,
    sex: u8,
) {
    apply_non_head_equipment_entry(
        resolved,
        EquipmentVisualSlot::OffHand,
        display_info_id,
        outfit_data,
        race,
        sex,
    );
}

fn apply_slot_geoset_overrides(
    slot: EquipmentVisualSlot,
    display_info_id: u32,
    outfit_data: &OutfitData,
    display: &mut OutfitResult,
) {
    match slot {
        EquipmentVisualSlot::Chest => {
            if let Some(variant) = outfit_data.chest_geoset_variant(display_info_id) {
                apply_geoset_overrides(display, vec![(22, variant)]);
            }
        }
        EquipmentVisualSlot::Hands => {
            if let Some(variant) = outfit_data.hand_geoset_variant(display_info_id) {
                apply_geoset_overrides(display, vec![(4, variant)]);
            }
        }
        EquipmentVisualSlot::Waist => {
            if let Some(variant) = outfit_data.hand_geoset_variant(display_info_id) {
                apply_geoset_overrides(display, vec![(18, variant)]);
            }
        }
        EquipmentVisualSlot::Legs => {
            let mut overrides = Vec::new();
            if let Some(variant) = outfit_data.pants_geoset_variant(display_info_id) {
                overrides.push((11, variant));
            }
            if let Some(variant) = outfit_data.kneepad_geoset_variant(display_info_id) {
                overrides.push((9, variant));
            }
            if let Some(variant) = outfit_data.trouser_geoset_variant(display_info_id) {
                overrides.push((13, variant));
            }
            if !overrides.is_empty() {
                apply_geoset_overrides(display, overrides);
            }
        }
        EquipmentVisualSlot::Back => {
            if let Some(variant) = outfit_data.cape_geoset_variant(display_info_id) {
                apply_geoset_overrides(display, vec![(15, variant)]);
            }
        }
        EquipmentVisualSlot::Feet => {
            if let Some(variant) = outfit_data.boot_geoset_variant(display_info_id) {
                apply_geoset_overrides(display, vec![(5, variant), (20, variant)]);
            }
        }
        _ => {}
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
            Some((path, skin_fdids))
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
    if let Some((model_path, skin_fdids)) =
        runtime_model_for_slot(slot, display_info_id, display, outfit_data, race, sex)
    {
        resolved.runtime_models.push(RuntimeModelAppearance {
            slot,
            path: model_path,
            skin_fdids,
        });
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
    match slot {
        EquipmentVisualSlot::Head => vec![EquipmentSlot::Head],
        EquipmentVisualSlot::Shoulder => {
            vec![EquipmentSlot::ShoulderLeft, EquipmentSlot::ShoulderRight]
        }
        EquipmentVisualSlot::Back => vec![EquipmentSlot::Back],
        EquipmentVisualSlot::Chest => vec![EquipmentSlot::Chest],
        EquipmentVisualSlot::Waist => vec![EquipmentSlot::Waist],
        EquipmentVisualSlot::Legs => vec![EquipmentSlot::Legs],
        EquipmentVisualSlot::Hands => vec![EquipmentSlot::Hands],
        EquipmentVisualSlot::Feet => vec![EquipmentSlot::Feet],
        EquipmentVisualSlot::MainHand => vec![EquipmentSlot::MainHand],
        EquipmentVisualSlot::OffHand => vec![EquipmentSlot::OffHand],
        _ => Vec::new(),
    }
}

fn runtime_model_for_slot(
    slot: EquipmentSlot,
    display_info_id: u32,
    _display: &OutfitResult,
    outfit_data: &OutfitData,
    race: u8,
    sex: u8,
) -> Option<(PathBuf, [u32; 3])> {
    let (fdid, skin_fdids) = match slot {
        EquipmentSlot::ShoulderLeft => {
            outfit_data.resolve_shoulder_runtime_model(display_info_id, 0, race, sex)?
        }
        EquipmentSlot::ShoulderRight => {
            outfit_data.resolve_shoulder_runtime_model(display_info_id, 1, race, sex)?
        }
        _ => outfit_data.resolve_runtime_model(display_info_id, race, sex)?,
    };
    let path = resolve_model_path(fdid)?;
    ensure_runtime_model_textures(&skin_fdids);
    Some((path, skin_fdids))
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
    let path = casc_resolver::ensure_file_at_path(fdid, &out_path)?;
    let _ = crate::asset::m2::ensure_primary_skin_path(&path);
    Some(path)
}

fn ensure_runtime_model_textures(skin_fdids: &[u32; 3]) {
    for &fdid in skin_fdids {
        if fdid != 0 {
            let _ = casc_resolver::ensure_texture(fdid);
        }
    }
}

fn ensure_item_component_textures(display: &OutfitResult) {
    for &(_, fdid) in &display.item_textures {
        let _ = casc_resolver::ensure_texture(fdid);
    }
}

#[cfg(test)]
#[path = "equipment_appearance_tests.rs"]
mod tests;
