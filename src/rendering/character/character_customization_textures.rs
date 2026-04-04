use std::collections::HashSet;

use bevy::prelude::*;

use super::{
    BatchTextureType, CharTextureData, CharacterCustomizationSelection, CustomizationDb,
    GeosetMesh, ResolvedEquipmentAppearance, collect_appearance_materials, is_descendant_of,
};

pub(super) fn apply_base_skin_and_overlay_textures(
    selection: CharacterCustomizationSelection,
    customization_db: &CustomizationDb,
    char_tex: &CharTextureData,
    overlay_set: &game_engine::outfit_data::OutfitResult,
    merged_cape_texture_fdid: Option<u32>,
    root: Entity,
    images: &mut Assets<Image>,
    materials: &mut Assets<StandardMaterial>,
    parent_query: &Query<&ChildOf>,
    material_query: &Query<(
        Entity,
        &MeshMaterial3d<StandardMaterial>,
        Option<&GeosetMesh>,
        Option<&BatchTextureType>,
        &ChildOf,
    )>,
) {
    let Some(replacements) = build_replacement_texture_handles(
        selection,
        customization_db,
        char_tex,
        overlay_set,
        merged_cape_texture_fdid,
        images,
    ) else {
        return;
    };
    apply_replacement_textures_to_materials(
        root,
        materials,
        parent_query,
        material_query,
        &replacements,
    );
}

struct ReplacementTextureHandles {
    body: Handle<Image>,
    head: Option<Handle<Image>>,
    hair: Option<Handle<Image>>,
    eye: Option<Handle<Image>>,
    cape: Option<Handle<Image>>,
}

fn build_replacement_texture_handles(
    selection: CharacterCustomizationSelection,
    customization_db: &CustomizationDb,
    char_tex: &CharTextureData,
    overlay_set: &game_engine::outfit_data::OutfitResult,
    merged_cape_texture_fdid: Option<u32>,
    images: &mut Assets<Image>,
) -> Option<ReplacementTextureHandles> {
    let all_materials = collect_appearance_materials(selection, customization_db);
    if all_materials.is_empty() {
        return None;
    }
    let layout_id = customization_db.layout_id(selection.race, selection.sex)?;
    let composited =
        char_tex.composite_model_textures(&all_materials, &overlay_set.item_textures, layout_id)?;
    let (body_pixels, body_w, body_h) = composited.body;
    Some(ReplacementTextureHandles {
        body: images.add(crate::rgba_image(body_pixels, body_w, body_h)),
        head: composited
            .head
            .map(|(pixels, width, height)| images.add(crate::rgba_image(pixels, width, height))),
        hair: composited
            .hair
            .map(|(pixels, width, height)| images.add(crate::rgba_image(pixels, width, height))),
        eye: char_tex
            .replacement_texture_fdid(&all_materials, layout_id, 19)
            .and_then(crate::asset::asset_cache::texture)
            .and_then(|path| crate::asset::blp::load_blp_to_image(&path).ok())
            .map(|image| images.add(image)),
        cape: merged_cape_texture_fdid
            .and_then(crate::asset::asset_cache::texture)
            .and_then(|path| crate::asset::blp::load_blp_to_image(&path).ok())
            .map(|image| images.add(image)),
    })
}

fn apply_replacement_textures_to_materials(
    root: Entity,
    materials: &mut Assets<StandardMaterial>,
    parent_query: &Query<&ChildOf>,
    material_query: &Query<(
        Entity,
        &MeshMaterial3d<StandardMaterial>,
        Option<&GeosetMesh>,
        Option<&BatchTextureType>,
        &ChildOf,
    )>,
    replacements: &ReplacementTextureHandles,
) {
    for (entity, mat_handle, _geoset_mesh, batch_texture_type, _) in material_query.iter() {
        if !is_descendant_of(entity, root, parent_query) {
            continue;
        }
        let Some(replacement) = replacement_texture_for_batch(
            batch_texture_type.map(|t| t.0),
            &replacements.body,
            replacements.head.as_ref(),
            replacements.hair.as_ref(),
            replacements.eye.as_ref(),
            replacements.cape.as_ref(),
        ) else {
            continue;
        };
        if let Some(mat) = materials.get_mut(&mat_handle.0) {
            mat.base_color_texture = Some(replacement);
            mat.base_color = Color::WHITE;
        }
    }
}

pub(crate) fn merge_overlay_texture_sets(
    base_layers: &game_engine::outfit_data::OutfitResult,
    overlay_layers: &game_engine::outfit_data::OutfitResult,
) -> game_engine::outfit_data::OutfitResult {
    let mut merged = base_layers.clone();
    let mut seen_item_textures = merged.item_textures.iter().copied().collect::<HashSet<_>>();
    let mut seen_model_fdids = merged.model_fdids.iter().copied().collect::<HashSet<_>>();

    for &(component_section, fdid) in &overlay_layers.item_textures {
        if seen_item_textures.insert((component_section, fdid)) {
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
        if seen_model_fdids.insert((model_resource_id, model_fdid)) {
            merged.model_fdids.push((model_resource_id, model_fdid));
        }
    }

    merged
}

pub(super) fn apply_explicit_equipment_overlays(
    base_layers: &game_engine::outfit_data::OutfitResult,
    equipped: &ResolvedEquipmentAppearance,
) -> game_engine::outfit_data::OutfitResult {
    let mut merged = base_layers.clone();

    if !equipped.explicit_slots.is_empty() {
        merged.item_textures.retain(|(section, _)| {
            let claimed_by_slot = equipped
                .explicit_slots
                .iter()
                .flat_map(|slot| component_sections_for_slot(*slot).iter().copied())
                .any(|suppressed| suppressed == *section);
            let has_replacement = equipped
                .outfit
                .item_textures
                .iter()
                .any(|&(s, _)| s == *section);
            !(claimed_by_slot && has_replacement)
        });
    }

    merge_overlay_texture_sets(&merged, &equipped.outfit)
}

pub(crate) fn replacement_texture_for_batch(
    texture_type: Option<u32>,
    body_handle: &Handle<Image>,
    head_handle: Option<&Handle<Image>>,
    hair_handle: Option<&Handle<Image>>,
    eye_handle: Option<&Handle<Image>>,
    cape_handle: Option<&Handle<Image>>,
) -> Option<Handle<Image>> {
    match texture_type {
        Some(1) => Some(body_handle.clone()),
        Some(2) => cape_handle.cloned(),
        Some(6) => Some(
            hair_handle
                .cloned()
                .or_else(|| head_handle.cloned())
                .unwrap_or_else(|| body_handle.clone()),
        ),
        Some(19) => eye_handle.cloned(),
        _ => None,
    }
}

pub(crate) fn component_sections_for_slot(
    slot: shared::components::EquipmentVisualSlot,
) -> &'static [u8] {
    use shared::components::EquipmentVisualSlot as Slot;

    match slot {
        Slot::Chest | Slot::Shirt => &[0, 3, 4],
        Slot::Tabard => &[3, 4],
        Slot::Wrist => &[1],
        Slot::Hands => &[1, 2],
        Slot::Waist => &[4, 5],
        Slot::Legs => &[5, 6],
        Slot::Feet => &[6, 7],
        _ => &[],
    }
}
