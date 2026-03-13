use bevy::prelude::*;

use game_engine::asset::char_texture::CharTextureData;
use game_engine::customization_data::{CustomizationDb, OptionType};
use game_engine::outfit_data::OutfitData;
use shared::components::CharacterAppearance;

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
    outfit_data: &OutfitData,
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
    let outfit = outfit_data.resolve_outfit(selection.race, selection.class, selection.sex);
    apply_body_texture(
        selection,
        customization_db,
        char_tex,
        &outfit,
        root,
        images,
        materials,
        parent_query,
        material_query,
    );
    apply_geoset_visibility(
        selection,
        customization_db,
        &outfit,
        root,
        parent_query,
        geoset_query,
        visibility_query,
    );
}

fn apply_body_texture(
    selection: CharacterCustomizationSelection,
    customization_db: &CustomizationDb,
    char_tex: &CharTextureData,
    _outfit: &game_engine::outfit_data::OutfitResult,
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
    let Some(composited) = char_tex.composite_model_textures(&all_materials, &[], layout_id) else {
        return;
    };
    let (body_pixels, body_w, body_h) = composited.body;
    let body_handle = images.add(crate::rgba_image(body_pixels, body_w, body_h));
    for (entity, mat_handle, batch_texture_type, _) in material_query.iter() {
        if !is_descendant_of(entity, root, parent_query) {
            continue;
        }
        let replacement = match batch_texture_type.map(|t| t.0) {
            Some(1) => Some(body_handle.clone()),
            Some(6) => Some(body_handle.clone()),
            _ => None,
        };
        let Some(replacement) = replacement else {
            continue;
        };
        if let Some(mat) = materials.get_mut(&mat_handle.0) {
            mat.base_color_texture = Some(replacement);
        }
    }
}

pub(crate) fn collect_appearance_materials(
    selection: CharacterCustomizationSelection,
    customization_db: &CustomizationDb,
) -> Vec<(u16, u32)> {
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
    let mut active_geosets: Vec<(u16, u16)> = Vec::new();
    let fields = [
        (OptionType::HairStyle, selection.appearance.hair_style),
        (OptionType::FacialHair, selection.appearance.facial_style),
    ];
    for (opt_type, index) in fields {
        if let Some(choice) = customization_db.get_choice_for_class(
            selection.race,
            selection.sex,
            selection.class,
            opt_type,
            index,
        ) {
            active_geosets.extend_from_slice(&choice.geosets);
        }
    }

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
    use super::group_zero_visible;

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
}
