use std::collections::HashSet;

use bevy::prelude::*;

use game_engine::asset::char_texture::CharTextureData;
use game_engine::customization_data::{CustomizationDb, OptionType};
use shared::components::CharacterAppearance;

use crate::equipment_appearance::ResolvedEquipmentAppearance;
use crate::m2_spawn::{BatchTextureType, GeosetMesh};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct CharacterCustomizationSelection {
    pub(crate) race: u8,
    pub(crate) class: u8,
    pub(crate) sex: u8,
    pub(crate) appearance: CharacterAppearance,
}

pub(crate) fn apply_character_customization(
    selection: CharacterCustomizationSelection,
    customization_db: &CustomizationDb,
    char_tex: &CharTextureData,
    equipped_appearance: Option<&ResolvedEquipmentAppearance>,
    root: Entity,
    images: &mut Assets<Image>,
    materials: &mut Assets<StandardMaterial>,
    parent_query: &Query<&ChildOf>,
    geoset_query: &Query<(Entity, &GeosetMesh, &ChildOf)>,
    visibility_query: &mut Query<&mut Visibility>,
    material_query: &Query<(
        Entity,
        &MeshMaterial3d<StandardMaterial>,
        Option<&BatchTextureType>,
        &ChildOf,
    )>,
) {
    let empty_overlay_set = game_engine::outfit_data::OutfitResult::default();
    let overlay_set = equipped_appearance
        .map(|equipped| apply_explicit_equipment_overlays(&empty_overlay_set, equipped))
        .unwrap_or(empty_overlay_set);
    apply_base_skin_and_overlay_textures(
        selection,
        customization_db,
        char_tex,
        &overlay_set,
        root,
        images,
        materials,
        parent_query,
        material_query,
    );
    apply_geoset_visibility(
        selection,
        customization_db,
        &overlay_set,
        root,
        parent_query,
        geoset_query,
        visibility_query,
    );
}

fn apply_base_skin_and_overlay_textures(
    selection: CharacterCustomizationSelection,
    customization_db: &CustomizationDb,
    char_tex: &CharTextureData,
    overlay_set: &game_engine::outfit_data::OutfitResult,
    root: Entity,
    images: &mut Assets<Image>,
    materials: &mut Assets<StandardMaterial>,
    parent_query: &Query<&ChildOf>,
    material_query: &Query<(
        Entity,
        &MeshMaterial3d<StandardMaterial>,
        Option<&BatchTextureType>,
        &ChildOf,
    )>,
) {
    let all_materials = collect_appearance_materials(selection, customization_db);
    if all_materials.is_empty() {
        return;
    }
    let Some(layout_id) = customization_db.layout_id(selection.race, selection.sex) else {
        return;
    };
    let Some(composited) =
        char_tex.composite_model_textures(&all_materials, &overlay_set.item_textures, layout_id)
    else {
        return;
    };
    let (body_pixels, body_w, body_h) = composited.body;
    let body_handle = images.add(crate::rgba_image(body_pixels, body_w, body_h));
    let head_handle = composited
        .head
        .map(|(pixels, width, height)| images.add(crate::rgba_image(pixels, width, height)));
    for (entity, mat_handle, batch_texture_type, _) in material_query.iter() {
        if !is_descendant_of(entity, root, parent_query) {
            continue;
        }
        let replacement = replacement_texture_for_batch(
            batch_texture_type.map(|t| t.0),
            &body_handle,
            head_handle.as_ref(),
        );
        let Some(replacement) = replacement else {
            continue;
        };
        if let Some(mat) = materials.get_mut(&mat_handle.0) {
            mat.base_color_texture = Some(replacement);
        }
    }
}

pub(crate) fn merge_overlay_texture_sets(
    base_layers: &game_engine::outfit_data::OutfitResult,
    overlay_layers: &game_engine::outfit_data::OutfitResult,
) -> game_engine::outfit_data::OutfitResult {
    let mut merged = base_layers.clone();

    for &(component_section, fdid) in &overlay_layers.item_textures {
        if !merged.item_textures.contains(&(component_section, fdid)) {
            merged.item_textures.push((component_section, fdid));
        }
    }

    for &(group, value) in &overlay_layers.geoset_overrides {
        merged
            .geoset_overrides
            .retain(|(existing_group, _)| *existing_group != group);
        merged.geoset_overrides.push((group, value));
    }

    for &(model_resource_id, model_fdid) in &overlay_layers.model_fdids {
        if !merged
            .model_fdids
            .contains(&(model_resource_id, model_fdid))
        {
            merged.model_fdids.push((model_resource_id, model_fdid));
        }
    }

    merged
}

pub(crate) fn apply_explicit_equipment_overlays(
    base_layers: &game_engine::outfit_data::OutfitResult,
    equipped: &ResolvedEquipmentAppearance,
) -> game_engine::outfit_data::OutfitResult {
    let mut merged = base_layers.clone();

    if !equipped.explicit_slots.is_empty() {
        merged.item_textures.retain(|(section, _)| {
            !equipped
                .explicit_slots
                .iter()
                .flat_map(|slot| component_sections_for_slot(*slot).iter().copied())
                .any(|suppressed| suppressed == *section)
        });
    }

    merge_overlay_texture_sets(&merged, &equipped.outfit)
}

fn replacement_texture_for_batch(
    texture_type: Option<u32>,
    body_handle: &Handle<Image>,
    head_handle: Option<&Handle<Image>>,
) -> Option<Handle<Image>> {
    match texture_type {
        Some(1) => Some(body_handle.clone()),
        Some(6) => Some(head_handle.cloned().unwrap_or_else(|| body_handle.clone())),
        _ => None,
    }
}

fn component_sections_for_slot(slot: shared::components::EquipmentVisualSlot) -> &'static [u8] {
    use shared::components::EquipmentVisualSlot as Slot;

    match slot {
        Slot::Chest | Slot::Shirt => &[0, 3, 4],
        Slot::Tabard => &[3, 4],
        Slot::Wrist => &[1],
        Slot::Hands => &[1, 2],
        Slot::Legs => &[5, 6],
        _ => &[],
    }
}

pub(crate) fn collect_appearance_materials(
    selection: CharacterCustomizationSelection,
    customization_db: &CustomizationDb,
) -> Vec<(u16, u32)> {
    let selected_choice_ids = selected_choice_ids(selection, customization_db);
    let fields = [
        (OptionType::SkinColor, selection.appearance.skin_color),
        (OptionType::Face, selection.appearance.face),
        (OptionType::HairStyle, selection.appearance.hair_style),
        (OptionType::HairColor, selection.appearance.hair_color),
        (OptionType::FacialHair, selection.appearance.facial_style),
    ];
    let mut all = Vec::new();
    for (opt_type, index) in fields {
        if let Some(choice) = customization_db.get_choice_for_class(
            selection.race,
            selection.sex,
            selection.class,
            opt_type,
            index,
        ) {
            all.extend_from_slice(&choice.materials);
            all.extend(
                choice
                    .related_materials
                    .iter()
                    .filter(|material| selected_choice_ids.contains(&material.related_choice_id))
                    .map(|material| (material.target_id, material.fdid)),
            );
        }
    }
    all
}

fn apply_geoset_visibility(
    selection: CharacterCustomizationSelection,
    customization_db: &CustomizationDb,
    outfit: &game_engine::outfit_data::OutfitResult,
    root: Entity,
    parent_query: &Query<&ChildOf>,
    geoset_query: &Query<(Entity, &GeosetMesh, &ChildOf)>,
    visibility_query: &mut Query<&mut Visibility>,
) {
    let mut active_geosets = collect_active_geosets(selection, customization_db);

    for &(group_index, value) in &outfit.geoset_overrides {
        active_geosets.retain(|(group, _)| *group != group_index);
        active_geosets.push((group_index, value));
    }

    let active_types: Vec<u16> = active_geosets.iter().map(|(t, _)| *t).collect();

    for (entity, geoset_mesh, child_of) in geoset_query.iter() {
        if child_of.parent() != root && !is_descendant_of(entity, root, parent_query) {
            continue;
        }
        let group = geoset_mesh.0 / 100;
        let variant = geoset_mesh.0 % 100;
        if !active_types.contains(&group) {
            continue;
        }
        let visible = if group == 0 {
            let selected_variant = active_geosets
                .iter()
                .find(|(t, _)| *t == 0)
                .map(|(_, id)| *id)
                .unwrap_or(0);
            group_zero_visible(geoset_mesh.0, selected_variant)
        } else {
            active_geosets
                .iter()
                .any(|(t, id)| *t == group && *id == variant)
        };
        if let Ok(mut vis) = visibility_query.get_mut(entity) {
            *vis = if visible {
                Visibility::Inherited
            } else {
                Visibility::Hidden
            };
        }
    }
}

fn group_zero_visible(mesh_part_id: u16, selected_variant: u16) -> bool {
    matches!(mesh_part_id, 0 | 1 | 16 | 17 | 27..=33) || mesh_part_id == selected_variant
}

fn selected_choice_ids(
    selection: CharacterCustomizationSelection,
    customization_db: &CustomizationDb,
) -> HashSet<u32> {
    let fields = [
        (OptionType::SkinColor, selection.appearance.skin_color),
        (OptionType::Face, selection.appearance.face),
        (OptionType::HairStyle, selection.appearance.hair_style),
        (OptionType::HairColor, selection.appearance.hair_color),
        (OptionType::FacialHair, selection.appearance.facial_style),
    ];
    fields
        .into_iter()
        .filter_map(|(opt_type, index)| {
            customization_db
                .get_choice_for_class(
                    selection.race,
                    selection.sex,
                    selection.class,
                    opt_type,
                    index,
                )
                .map(|choice| choice.id)
        })
        .collect()
}

fn collect_active_geosets(
    selection: CharacterCustomizationSelection,
    customization_db: &CustomizationDb,
) -> Vec<(u16, u16)> {
    let mut active_geosets: Vec<(u16, u16)> = Vec::new();
    let selected_choice_ids = selected_choice_ids(selection, customization_db);
    let fields = [
        (OptionType::HairStyle, Some(selection.appearance.hair_style)),
        (OptionType::FacialHair, Some(selection.appearance.facial_style)),
        // CharacterAppearance doesn't persist modern ear choices yet.
        // Pick the first DB choice so we drive a single ear geoset instead of
        // leaving both default ear meshes visible on HD models.
        (OptionType::Ears, Some(0)),
    ];
    for (opt_type, index) in fields {
        let Some(index) = index else {
            continue;
        };
        if let Some(choice) = customization_db.get_choice_for_class(
            selection.race,
            selection.sex,
            selection.class,
            opt_type,
            index,
        ) {
            active_geosets.extend_from_slice(&choice.geosets);
            active_geosets.extend(
                choice
                    .related_geosets
                    .iter()
                    .filter(|geoset| selected_choice_ids.contains(&geoset.related_choice_id))
                    .map(|geoset| (geoset.geoset_type, geoset.geoset_id)),
            );
        }
    }
    active_geosets
}

fn is_descendant_of(entity: Entity, root: Entity, parent_query: &Query<&ChildOf>) -> bool {
    let mut current = entity;
    loop {
        let Ok(child_of) = parent_query.get(current) else {
            return false;
        };
        let parent = child_of.parent();
        if parent == root {
            return true;
        }
        current = parent;
    }
}

#[cfg(test)]
mod tests {
    use super::{
        CharacterCustomizationSelection, apply_explicit_equipment_overlays, collect_active_geosets,
        collect_appearance_materials, component_sections_for_slot, group_zero_visible,
        merge_overlay_texture_sets, replacement_texture_for_batch,
    };
    use crate::equipment_appearance::ResolvedEquipmentAppearance;
    use bevy::prelude::{Handle, Image};
    use game_engine::customization_data::CustomizationDb;
    use game_engine::outfit_data::OutfitResult;
    use shared::components::{CharacterAppearance, EquipmentVisualSlot};
    use std::path::Path;

    #[test]
    fn hairstyle_group_zero_keeps_base_body_segments_visible() {
        assert!(group_zero_visible(0, 2));
        assert!(group_zero_visible(1, 2));
        assert!(group_zero_visible(2, 2));
        assert!(group_zero_visible(16, 2));
        assert!(group_zero_visible(17, 2));
        assert!(group_zero_visible(28, 2));

        assert!(!group_zero_visible(5, 2));
        assert!(!group_zero_visible(18, 2));
    }

    #[test]
    fn face_materials_resolve_against_selected_skin_color() {
        let db = CustomizationDb::load(Path::new("data"));
        let base = CharacterCustomizationSelection {
            race: 1,
            class: 1,
            sex: 0,
            appearance: CharacterAppearance {
                sex: 0,
                skin_color: 0,
                face: 0,
                hair_style: 0,
                hair_color: 0,
                facial_style: 0,
            },
        };
        let skin0 = collect_appearance_materials(base, &db);
        let skin1 = collect_appearance_materials(
            CharacterCustomizationSelection {
                appearance: CharacterAppearance {
                    skin_color: 1,
                    ..base.appearance
                },
                ..base
            },
            &db,
        );

        let face0: Vec<_> = skin0
            .iter()
            .filter(|(target_id, _)| *target_id == 5)
            .map(|(_, fdid)| *fdid)
            .collect();
        let face1: Vec<_> = skin1
            .iter()
            .filter(|(target_id, _)| *target_id == 5)
            .map(|(_, fdid)| *fdid)
            .collect();

        assert_eq!(
            face0.len(),
            1,
            "skin color 0 should resolve one face texture"
        );
        assert_eq!(
            face1.len(),
            1,
            "skin color 1 should resolve one face texture"
        );
        assert_ne!(face0[0], face1[0], "face texture should vary by skin color");
    }

    #[test]
    fn human_male_defaults_to_single_round_ear_geoset() {
        let db = CustomizationDb::load(Path::new("data"));
        let geosets = collect_active_geosets(
            CharacterCustomizationSelection {
                race: 1,
                class: 1,
                sex: 0,
                appearance: CharacterAppearance {
                    sex: 0,
                    skin_color: 0,
                    face: 0,
                    hair_style: 0,
                    hair_color: 0,
                    facial_style: 0,
                },
            },
            &db,
        );

        assert!(
            geosets.contains(&(7, 2)),
            "human male should drive the round-ear geoset by default: {geosets:?}"
        );
        assert!(
            !geosets.contains(&(7, 1)),
            "human male should not leave the hidden/default ear mesh active alongside the selected one: {geosets:?}"
        );
    }

    #[test]
    fn merge_overlay_texture_sets_appends_equipment_layers_without_duplicates() {
        let base = OutfitResult {
            item_textures: vec![(3, 100), (4, 200)],
            geoset_overrides: vec![(13, 1)],
            model_fdids: vec![(10, 1000)],
        };
        let equipped = OutfitResult {
            item_textures: vec![(4, 200), (7, 300)],
            geoset_overrides: vec![(13, 2), (15, 3)],
            model_fdids: vec![(10, 1000), (11, 2000)],
        };

        let merged = merge_overlay_texture_sets(&base, &equipped);

        assert_eq!(merged.item_textures, vec![(3, 100), (4, 200), (7, 300)]);
        assert_eq!(merged.geoset_overrides, vec![(13, 2), (15, 3)]);
        assert_eq!(merged.model_fdids, vec![(10, 1000), (11, 2000)]);
    }

    #[test]
    fn apply_explicit_equipment_overlays_keeps_skin_feet_sections_when_feet_hidden() {
        let base = OutfitResult {
            item_textures: vec![(5, 100), (6, 200), (7, 300)],
            ..Default::default()
        };
        let equipped = ResolvedEquipmentAppearance {
            explicit_slots: [EquipmentVisualSlot::Feet].into_iter().collect(),
            ..Default::default()
        };

        let merged = apply_explicit_equipment_overlays(&base, &equipped);

        assert_eq!(merged.item_textures, vec![(5, 100), (6, 200), (7, 300)]);
    }

    #[test]
    fn apply_explicit_equipment_overlays_replaces_conflicting_leg_sections() {
        let base = OutfitResult {
            item_textures: vec![(5, 100), (6, 200), (7, 300)],
            ..Default::default()
        };
        let equipped = ResolvedEquipmentAppearance {
            outfit: OutfitResult {
                item_textures: vec![(5, 400), (6, 500)],
                ..Default::default()
            },
            explicit_slots: [EquipmentVisualSlot::Legs].into_iter().collect(),
            ..Default::default()
        };

        let merged = apply_explicit_equipment_overlays(&base, &equipped);

        assert_eq!(merged.item_textures, vec![(7, 300), (5, 400), (6, 500)]);
    }

    #[test]
    fn replacement_texture_for_head_batches_prefers_head_atlas() {
        let body = Handle::<Image>::default();
        let head = Handle::<Image>::default();

        let replacement = replacement_texture_for_batch(Some(6), &body, Some(&head));

        assert_eq!(replacement, Some(head));
    }

    #[test]
    fn feet_slot_maps_to_foot_component_section() {
        assert_eq!(
            component_sections_for_slot(EquipmentVisualSlot::Feet),
            &[] as &[u8]
        );
    }
}
