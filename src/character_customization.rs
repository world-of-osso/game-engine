use bevy::prelude::*;

use game_engine::asset::char_texture::CharTextureData;
use game_engine::customization_data::{CustomizationDb, OptionType};
use game_engine::outfit_data::OutfitData;
use shared::components::CharacterAppearance;

use crate::m2_spawn::GeosetMesh;

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
    geoset_query: &Query<(Entity, &GeosetMesh, &ChildOf)>,
    visibility_query: &mut Query<&mut Visibility>,
    material_query: &Query<(&MeshMaterial3d<StandardMaterial>, &ChildOf)>,
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
        material_query,
    );
    apply_geoset_visibility(
        selection,
        customization_db,
        &outfit,
        root,
        geoset_query,
        visibility_query,
    );
}

fn apply_body_texture(
    selection: CharacterCustomizationSelection,
    customization_db: &CustomizationDb,
    char_tex: &CharTextureData,
    outfit: &game_engine::outfit_data::OutfitResult,
    root: Entity,
    images: &mut Assets<Image>,
    materials: &mut Assets<StandardMaterial>,
    material_query: &Query<(&MeshMaterial3d<StandardMaterial>, &ChildOf)>,
) {
    let all_materials = collect_appearance_materials(selection, customization_db);
    if all_materials.is_empty() {
        return;
    }
    let Some(layout_id) = customization_db.layout_id(selection.race, selection.sex) else {
        return;
    };
    let Some((pixels, w, h)) =
        char_tex.composite_with_items(&all_materials, &outfit.item_textures, layout_id)
    else {
        return;
    };
    let img = crate::rgba_image(pixels, w, h);
    let img_handle = images.add(img);
    for (mat_handle, child_of) in material_query.iter() {
        if child_of.parent() != root {
            continue;
        }
        if let Some(mat) = materials.get_mut(&mat_handle.0) {
            mat.base_color_texture = Some(img_handle.clone());
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
        if child_of.parent() != root {
            continue;
        }
        let group = geoset_mesh.0 / 100;
        let variant = geoset_mesh.0 % 100;
        if !active_types.contains(&group) {
            continue;
        }
        let visible = active_geosets
            .iter()
            .any(|(t, id)| *t == group && *id == variant);
        if let Ok(mut vis) = visibility_query.get_mut(entity) {
            *vis = if visible {
                Visibility::Inherited
            } else {
                Visibility::Hidden
            };
        }
    }
}
