use std::collections::HashSet;

use bevy::prelude::*;

use game_engine::asset::m2::default_geoset_visible;
use game_engine::asset::char_texture::CharTextureData;
use game_engine::customization_data::{CustomizationDb, OptionType};
use shared::components::{CharacterAppearance, EquipmentAppearance as NetEquipmentAppearance};

use crate::equipment::Equipment;
use crate::equipment_appearance::{
    ResolvedEquipmentAppearance, apply_runtime_equipment, resolve_equipment_appearance,
};
use crate::m2_spawn::{BatchTextureType, GeosetMesh};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct CharacterCustomizationSelection {
    pub(crate) race: u8,
    pub(crate) class: u8,
    pub(crate) sex: u8,
    pub(crate) appearance: CharacterAppearance,
}

#[derive(Component, Clone, Debug, PartialEq, Eq)]
pub(crate) struct CharacterRenderRequest {
    pub(crate) selection: CharacterCustomizationSelection,
    pub(crate) equipment_appearance: NetEquipmentAppearance,
}

#[derive(Component, Clone, Debug, PartialEq, Eq)]
struct AppliedCharacterRenderRequest(CharacterRenderRequest);

pub(crate) struct CharacterCustomizationPlugin;

impl Plugin for CharacterCustomizationPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Update, sync_character_render_requests);
    }
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
    let empty_hidden_groups = HashSet::new();
    let overlay_set = equipped_appearance
        .map(|equipped| apply_explicit_equipment_overlays(&empty_overlay_set, equipped))
        .unwrap_or(empty_overlay_set);
    let hidden_groups = equipped_appearance
        .map(|equipped| &equipped.hidden_character_geoset_groups)
        .unwrap_or(&empty_hidden_groups);
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
        hidden_groups,
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
    let hair_handle = composited
        .hair
        .map(|(pixels, width, height)| images.add(crate::rgba_image(pixels, width, height)));
    for (entity, mat_handle, batch_texture_type, _) in material_query.iter() {
        if !is_descendant_of(entity, root, parent_query) {
            continue;
        }
        let replacement = replacement_texture_for_batch(
            batch_texture_type.map(|t| t.0),
            &body_handle,
            head_handle.as_ref(),
            hair_handle.as_ref(),
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
    hair_handle: Option<&Handle<Image>>,
) -> Option<Handle<Image>> {
    match texture_type {
        Some(1) => Some(body_handle.clone()),
        Some(6) => Some(
            hair_handle
                .cloned()
                .or_else(|| head_handle.cloned())
                .unwrap_or_else(|| body_handle.clone()),
        ),
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
        Slot::Feet => &[6, 7],
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

#[allow(clippy::too_many_arguments)]
fn sync_character_render_requests(
    mut commands: Commands,
    customization_db: Res<CustomizationDb>,
    char_tex: Res<CharTextureData>,
    outfit_data: Res<game_engine::outfit_data::OutfitData>,
    request_query: Query<
        (
            Entity,
            &CharacterRenderRequest,
            Option<&AppliedCharacterRenderRequest>,
        ),
    >,
    parent_query: Query<&ChildOf>,
    geoset_query: Query<(Entity, &GeosetMesh, &ChildOf)>,
    mut visibility_query: Query<&mut Visibility>,
    material_query: Query<(
        Entity,
        &MeshMaterial3d<StandardMaterial>,
        Option<&BatchTextureType>,
        &ChildOf,
    )>,
    mut equipment_query: Query<&mut Equipment>,
    mut images: ResMut<Assets<Image>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    for (entity, request, applied) in &request_query {
        if applied.is_some_and(|applied| applied.0 == *request) {
            continue;
        }
        let resolved_equipment = resolve_equipment_appearance(
            &request.equipment_appearance,
            &outfit_data,
            request.selection.race,
            request.selection.sex,
        );
        apply_character_customization(
            request.selection,
            &customization_db,
            &char_tex,
            Some(&resolved_equipment),
            entity,
            &mut images,
            &mut materials,
            &parent_query,
            &geoset_query,
            &mut visibility_query,
            &material_query,
        );
        if let Ok(mut equipment) = equipment_query.get_mut(entity) {
            apply_runtime_equipment(&mut equipment, &resolved_equipment);
        }
        commands
            .entity(entity)
            .insert(AppliedCharacterRenderRequest(request.clone()));
    }
}

fn apply_geoset_visibility(
    selection: CharacterCustomizationSelection,
    customization_db: &CustomizationDb,
    outfit: &game_engine::outfit_data::OutfitResult,
    hidden_groups: &HashSet<u16>,
    root: Entity,
    parent_query: &Query<&ChildOf>,
    geoset_query: &Query<(Entity, &GeosetMesh, &ChildOf)>,
    visibility_query: &mut Query<&mut Visibility>,
) {
    let mut active_geosets = collect_active_geosets(selection, customization_db);
    apply_hidden_geoset_groups(&mut active_geosets, hidden_groups, selection, customization_db);

    for &(group_index, value) in &outfit.geoset_overrides {
        active_geosets.retain(|(group, _)| *group != group_index);
        active_geosets.push((group_index, value));
    }

    let active_types: Vec<u16> = active_geosets.iter().map(|(t, _)| *t).collect();

    for (entity, geoset_mesh, child_of) in geoset_query.iter() {
        if child_of.parent() != root && !is_descendant_of(entity, root, parent_query) {
            continue;
        }
        let visible = is_geoset_visible(geoset_mesh.0, &active_geosets, &active_types);
        if let Ok(mut vis) = visibility_query.get_mut(entity) {
            *vis = if visible {
                Visibility::Inherited
            } else {
                Visibility::Hidden
            };
        }
    }
}

fn is_geoset_visible(mesh_part_id: u16, active_geosets: &[(u16, u16)], active_types: &[u16]) -> bool {
    let group = mesh_part_id / 100;
    let variant = mesh_part_id % 100;
    if !active_types.contains(&group) {
        return default_geoset_visible(mesh_part_id);
    }
    if group == 0 {
        let selected_variant = active_geosets
            .iter()
            .find(|(t, _)| *t == 0)
            .map(|(_, id)| *id)
            .unwrap_or(0);
        return group_zero_visible(mesh_part_id, selected_variant);
    }
    active_geosets
        .iter()
        .any(|(t, id)| *t == group && *id == variant)
}

fn group_zero_visible(mesh_part_id: u16, selected_variant: u16) -> bool {
    mesh_part_id == 0 || mesh_part_id == selected_variant
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
        (
            OptionType::FacialHair,
            Some(selection.appearance.facial_style),
        ),
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

fn apply_hidden_geoset_groups(
    active_geosets: &mut Vec<(u16, u16)>,
    hidden_groups: &HashSet<u16>,
    selection: CharacterCustomizationSelection,
    customization_db: &CustomizationDb,
) {
    for &group in hidden_groups {
        active_geosets.retain(|(existing_group, _)| *existing_group != group);
        active_geosets.push((group, hidden_group_variant(group, selection, customization_db)));
    }
}

fn hidden_group_variant(
    group: u16,
    selection: CharacterCustomizationSelection,
    customization_db: &CustomizationDb,
) -> u16 {
    if group == 0 {
        customization_db
            .scalp_fallback_hair_geoset(selection.race, selection.sex)
            .unwrap_or(1)
    } else {
        1
    }
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
        CharacterCustomizationSelection, apply_explicit_equipment_overlays,
        apply_hidden_geoset_groups, collect_active_geosets, collect_appearance_materials,
        component_sections_for_slot, group_zero_visible, merge_overlay_texture_sets,
        replacement_texture_for_batch,
    };
    use crate::equipment_appearance::ResolvedEquipmentAppearance;
    use bevy::prelude::{Assets, Image};
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
        let face0 = resolved_face_target_fdids(selection_with_skin_color(0), &db);
        let face1 = resolved_face_target_fdids(selection_with_skin_color(1), &db);

        assert_eq!(face0.len(), 1, "skin color 0 should resolve one face texture");
        assert_eq!(face1.len(), 1, "skin color 1 should resolve one face texture");
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
    fn replacement_texture_for_type_six_batches_prefers_hair_atlas() {
        let mut images = Assets::<Image>::default();
        let body = images.add(Image::default());
        let head = images.add(Image::default());
        let hair = images.add(Image::default());

        let replacement = replacement_texture_for_batch(Some(6), &body, Some(&head), Some(&hair));

        assert_eq!(replacement, Some(hair));
    }

    #[test]
    fn feet_slot_maps_to_foot_component_section() {
        assert_eq!(
            component_sections_for_slot(EquipmentVisualSlot::Feet),
            &[6u8, 7] as &[u8]
        );
    }

    #[test]
    fn hidden_helmet_groups_use_scalp_fallback_for_group_zero() {
        let db = CustomizationDb::load(Path::new("data"));
        let mut active_geosets = vec![(0, 5), (1, 4), (7, 2)];
        let hidden_groups = [0, 7].into_iter().collect();
        let selection = CharacterCustomizationSelection {
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

        apply_hidden_geoset_groups(&mut active_geosets, &hidden_groups, selection, &db);

        assert!(active_geosets.contains(&(0, 0)));
        assert!(active_geosets.contains(&(7, 1)));
        assert!(!active_geosets.contains(&(0, 5)));
        assert!(!active_geosets.contains(&(7, 2)));
        assert!(active_geosets.contains(&(1, 4)));
    }

    fn selection_with_skin_color(skin_color: u8) -> CharacterCustomizationSelection {
        CharacterCustomizationSelection {
            race: 1,
            class: 1,
            sex: 0,
            appearance: CharacterAppearance {
                sex: 0,
                skin_color,
                face: 0,
                hair_style: 0,
                hair_color: 0,
                facial_style: 0,
            },
        }
    }

    fn resolved_face_target_fdids(
        selection: CharacterCustomizationSelection,
        db: &CustomizationDb,
    ) -> Vec<u32> {
        collect_appearance_materials(selection, db)
            .iter()
            .filter(|(target_id, _)| *target_id == 5)
            .map(|(_, fdid)| *fdid)
            .collect()
    }
}
