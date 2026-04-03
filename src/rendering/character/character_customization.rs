use std::collections::HashSet;

use bevy::ecs::system::SystemParam;
use bevy::prelude::*;

use game_engine::asset::char_texture::CharTextureData;
use game_engine::asset::m2::default_geoset_visible;
use game_engine::customization_data::{CustomizationDb, OptionType};
use shared::components::{CharacterAppearance, EquipmentAppearance as NetEquipmentAppearance};

use crate::equipment::{Equipment, EquipmentItem};
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

#[derive(SystemParam)]
struct CharacterRenderRequestParams<'w, 's> {
    commands: Commands<'w, 's>,
    customization_db: Res<'w, CustomizationDb>,
    char_tex: Res<'w, CharTextureData>,
    outfit_data: Res<'w, game_engine::outfit_data::OutfitData>,
    request_query: Query<
        'w,
        's,
        (
            Entity,
            &'static CharacterRenderRequest,
            Option<&'static AppliedCharacterRenderRequest>,
        ),
    >,
    parent_query: Query<'w, 's, &'static ChildOf>,
    geoset_query: Query<'w, 's, (Entity, &'static GeosetMesh, &'static ChildOf)>,
    visibility_query: Query<'w, 's, &'static mut Visibility>,
    material_query: Query<
        'w,
        's,
        (
            Entity,
            &'static MeshMaterial3d<StandardMaterial>,
            Option<&'static GeosetMesh>,
            Option<&'static BatchTextureType>,
            &'static ChildOf,
        ),
    >,
    equipment_query: Query<'w, 's, &'static mut Equipment>,
    equipment_item_query: Query<'w, 's, (), With<EquipmentItem>>,
    images: ResMut<'w, Assets<Image>>,
    materials: ResMut<'w, Assets<StandardMaterial>>,
}

struct CharacterRenderRequestContext<'a, 'w, 's> {
    customization_db: &'a CustomizationDb,
    char_tex: &'a CharTextureData,
    outfit_data: &'a game_engine::outfit_data::OutfitData,
    parent_query: &'a Query<'w, 's, &'static ChildOf>,
    geoset_query: &'a Query<'w, 's, (Entity, &'static GeosetMesh, &'static ChildOf)>,
    visibility_query: &'a mut Query<'w, 's, &'static mut Visibility>,
    material_query: &'a Query<
        'w,
        's,
        (
            Entity,
            &'static MeshMaterial3d<StandardMaterial>,
            Option<&'static GeosetMesh>,
            Option<&'static BatchTextureType>,
            &'static ChildOf,
        ),
    >,
    equipment_query: &'a mut Query<'w, 's, &'static mut Equipment>,
    equipment_item_query: &'a Query<'w, 's, (), With<EquipmentItem>>,
    images: &'a mut Assets<Image>,
    materials: &'a mut Assets<StandardMaterial>,
    commands: &'a mut Commands<'w, 's>,
}

impl<'a, 'w, 's> CharacterRenderRequestContext<'a, 'w, 's> {
    fn from_params(params: &'a mut CharacterRenderRequestParams<'w, 's>) -> Self {
        Self {
            customization_db: &params.customization_db,
            char_tex: &params.char_tex,
            outfit_data: &params.outfit_data,
            parent_query: &params.parent_query,
            geoset_query: &params.geoset_query,
            visibility_query: &mut params.visibility_query,
            material_query: &params.material_query,
            equipment_query: &mut params.equipment_query,
            equipment_item_query: &params.equipment_item_query,
            images: &mut params.images,
            materials: &mut params.materials,
            commands: &mut params.commands,
        }
    }
}

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
    equipment_item_query: &Query<(), With<EquipmentItem>>,
    material_query: &Query<(
        Entity,
        &MeshMaterial3d<StandardMaterial>,
        Option<&GeosetMesh>,
        Option<&BatchTextureType>,
        &ChildOf,
    )>,
) {
    let empty_overlay_set = game_engine::outfit_data::OutfitResult::default();
    let empty_hidden_groups = HashSet::new();
    let empty_hidden_ids = HashSet::new();
    let overlay_set = equipped_appearance
        .map(|equipped| apply_explicit_equipment_overlays(&empty_overlay_set, equipped))
        .unwrap_or(empty_overlay_set);
    let hidden_groups = equipped_appearance
        .map(|equipped| &equipped.hidden_character_geoset_groups)
        .unwrap_or(&empty_hidden_groups);
    let hidden_ids = equipped_appearance
        .map(|equipped| &equipped.hidden_character_geoset_ids)
        .unwrap_or(&empty_hidden_ids);
    apply_base_skin_and_overlay_textures(
        selection,
        customization_db,
        char_tex,
        &overlay_set,
        equipped_appearance.and_then(|equipped| equipped.merged_cape_texture_fdid),
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
        hidden_ids,
        root,
        parent_query,
        geoset_query,
        visibility_query,
        equipment_item_query,
    );
}

fn apply_base_skin_and_overlay_textures(
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

fn replacement_texture_for_batch(
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

fn component_sections_for_slot(slot: shared::components::EquipmentVisualSlot) -> &'static [u8] {
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

pub(crate) fn collect_appearance_materials(
    selection: CharacterCustomizationSelection,
    customization_db: &CustomizationDb,
) -> Vec<(u16, u32)> {
    let selected_choice_ids = selected_choice_ids(selection, customization_db);
    let fields = [
        (OptionType::SkinColor, selection.appearance.skin_color),
        (OptionType::Face, selection.appearance.face),
        (OptionType::EyeColor, selection.appearance.eye_color),
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

fn sync_character_render_requests(mut params: CharacterRenderRequestParams) {
    let pending_requests = params
        .request_query
        .iter()
        .map(|(entity, request, applied)| (entity, request.clone(), applied.cloned()))
        .collect::<Vec<_>>();
    for (entity, request, applied) in pending_requests {
        if applied.is_some_and(|applied| applied.0 == request) {
            continue;
        }
        if !character_render_targets_ready(
            entity,
            &params.parent_query,
            &params.geoset_query,
            &params.material_query,
        ) {
            continue;
        }
        let mut context = CharacterRenderRequestContext::from_params(&mut params);
        apply_character_render_request(entity, &request, &mut context);
    }
}

fn apply_character_render_request(
    entity: Entity,
    request: &CharacterRenderRequest,
    context: &mut CharacterRenderRequestContext,
) {
    let resolved_equipment = resolve_equipment_appearance(
        &request.equipment_appearance,
        context.outfit_data,
        request.selection.race,
        request.selection.sex,
    );
    log_character_render_apply(entity, request, &resolved_equipment);
    apply_character_customization(
        request.selection,
        context.customization_db,
        context.char_tex,
        Some(&resolved_equipment),
        entity,
        context.images,
        context.materials,
        context.parent_query,
        context.geoset_query,
        context.visibility_query,
        context.equipment_item_query,
        context.material_query,
    );
    info!(
        "character visible geosets entity={entity:?} ids={:?}",
        visible_geoset_ids_for_root(
            entity,
            context.parent_query,
            context.geoset_query,
            context.visibility_query,
        )
    );
    finalize_character_render(
        entity,
        request,
        &resolved_equipment,
        context.equipment_query,
        context.commands,
    );
}

fn finalize_character_render(
    entity: Entity,
    request: &CharacterRenderRequest,
    resolved_equipment: &ResolvedEquipmentAppearance,
    equipment_query: &mut Query<&mut Equipment>,
    commands: &mut Commands,
) {
    if let Ok(mut equipment) = equipment_query.get_mut(entity) {
        apply_runtime_equipment(&mut equipment, resolved_equipment);
    }
    commands
        .entity(entity)
        .insert(AppliedCharacterRenderRequest(request.clone()));
}

fn log_character_render_apply(
    entity: Entity,
    request: &CharacterRenderRequest,
    resolved_equipment: &ResolvedEquipmentAppearance,
) {
    info!(
        "character render apply entity={entity:?} request_entries={:?} geoset_overrides={:?} hidden_groups={:?} runtime_models={:?}",
        request
            .equipment_appearance
            .entries
            .iter()
            .map(|entry| (entry.slot, entry.display_info_id, entry.hidden))
            .collect::<Vec<_>>(),
        resolved_equipment.outfit.geoset_overrides,
        resolved_equipment.hidden_character_geoset_groups,
        resolved_equipment
            .runtime_models
            .iter()
            .map(|model| (&model.slot, model.path.display().to_string()))
            .collect::<Vec<_>>()
    );
}

fn visible_geoset_ids_for_root(
    root: Entity,
    parent_query: &Query<&ChildOf>,
    geoset_query: &Query<(Entity, &GeosetMesh, &ChildOf)>,
    visibility_query: &mut Query<&mut Visibility>,
) -> Vec<u16> {
    let mut ids = geoset_query
        .iter()
        .filter(|(entity, _, child_of)| {
            child_of.parent() == root || is_descendant_of(*entity, root, parent_query)
        })
        .filter_map(
            |(entity, geoset_mesh, _)| match visibility_query.get(entity) {
                Ok(visibility) if matches!(*visibility, Visibility::Inherited) => {
                    Some(geoset_mesh.0)
                }
                _ => None,
            },
        )
        .collect::<Vec<_>>();
    ids.sort_unstable();
    ids.dedup();
    ids
}

fn character_render_targets_ready(
    root: Entity,
    parent_query: &Query<&ChildOf>,
    geoset_query: &Query<(Entity, &GeosetMesh, &ChildOf)>,
    material_query: &Query<(
        Entity,
        &MeshMaterial3d<StandardMaterial>,
        Option<&GeosetMesh>,
        Option<&BatchTextureType>,
        &ChildOf,
    )>,
) -> bool {
    let has_geosets = geoset_query
        .iter()
        .any(|(entity, _, _)| is_descendant_of(entity, root, parent_query));
    let has_materials = material_query
        .iter()
        .any(|(entity, _, _, _, _)| is_descendant_of(entity, root, parent_query));
    has_geosets && has_materials
}

fn apply_geoset_visibility(
    selection: CharacterCustomizationSelection,
    customization_db: &CustomizationDb,
    outfit: &game_engine::outfit_data::OutfitResult,
    hidden_groups: &HashSet<u16>,
    hidden_geoset_ids: &HashSet<u16>,
    root: Entity,
    parent_query: &Query<&ChildOf>,
    geoset_query: &Query<(Entity, &GeosetMesh, &ChildOf)>,
    visibility_query: &mut Query<&mut Visibility>,
    equipment_item_query: &Query<(), With<EquipmentItem>>,
) {
    let mut active_geosets = collect_active_geosets(selection, customization_db);
    apply_hidden_geoset_groups(
        &mut active_geosets,
        hidden_groups,
        selection,
        customization_db,
    );

    let active_types: Vec<u16> = active_geosets.iter().map(|(t, _)| *t).collect();

    for (entity, geoset_mesh, child_of) in geoset_query.iter() {
        if child_of.parent() != root && !is_descendant_of(entity, root, parent_query) {
            continue;
        }
        if has_equipment_item_ancestor(entity, parent_query, equipment_item_query) {
            continue;
        }
        let mut visible = !hidden_geoset_ids.contains(&geoset_mesh.0)
            && is_geoset_visible(geoset_mesh.0, &active_geosets, &active_types);
        visible = apply_exact_geoset_overrides(geoset_mesh.0, visible, &outfit.geoset_overrides);
        if let Ok(mut vis) = visibility_query.get_mut(entity) {
            *vis = if visible {
                Visibility::Inherited
            } else {
                Visibility::Hidden
            };
        }
    }
}

fn is_geoset_visible(
    mesh_part_id: u16,
    active_geosets: &[(u16, u16)],
    active_types: &[u16],
) -> bool {
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
    mesh_part_id == selected_variant || is_group_zero_body_segment(mesh_part_id)
}

/// Group-0 body segments always visible regardless of hair selection.
/// 0-1 are the human male base skin + scalp closure meshes; 27-33 are
/// body segments on models that multiplex body geometry through group 0.
/// Hair variants like 16/17 must remain switchable so helmet hides can suppress them.
fn is_group_zero_body_segment(mesh_part_id: u16) -> bool {
    matches!(mesh_part_id, 0 | 1 | 27..=33)
}

fn selected_choice_ids(
    selection: CharacterCustomizationSelection,
    customization_db: &CustomizationDb,
) -> HashSet<u32> {
    let fields = [
        (OptionType::SkinColor, selection.appearance.skin_color),
        (OptionType::Face, selection.appearance.face),
        (OptionType::EyeColor, selection.appearance.eye_color),
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
    for (opt_type, index) in selected_geoset_fields(selection) {
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

fn selected_geoset_fields(
    selection: CharacterCustomizationSelection,
) -> [(OptionType, Option<u8>); 3] {
    [
        (OptionType::HairStyle, Some(selection.appearance.hair_style)),
        (
            OptionType::FacialHair,
            Some(selection.appearance.facial_style),
        ),
        // CharacterAppearance doesn't persist modern ear choices yet.
        // Pick the first DB choice so render state still drives one sane ear geoset.
        (OptionType::Ears, Some(0)),
    ]
}

fn apply_hidden_geoset_groups(
    active_geosets: &mut Vec<(u16, u16)>,
    hidden_groups: &HashSet<u16>,
    selection: CharacterCustomizationSelection,
    customization_db: &CustomizationDb,
) {
    for &group in hidden_groups {
        active_geosets.retain(|(existing_group, _)| *existing_group != group);
        active_geosets.push((
            group,
            hidden_group_variant(group, selection, customization_db),
        ));
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

fn apply_exact_geoset_overrides(
    mesh_part_id: u16,
    base_visible: bool,
    overrides: &[(u16, u16)],
) -> bool {
    let group = mesh_part_id / 100;
    let mut visible = base_visible;
    for &(override_group, value) in overrides {
        if override_group == group {
            if value == 0 {
                // Exact hide: only affects mesh group*100+0
                if mesh_part_id == group * 100 {
                    visible = false;
                }
            } else {
                // Group-level switch: show only the target variant, hide others
                visible = mesh_part_id == group * 100 + value;
            }
        }
    }
    visible
}

fn has_equipment_item_ancestor(
    entity: Entity,
    parent_query: &Query<&ChildOf>,
    equipment_item_query: &Query<(), With<EquipmentItem>>,
) -> bool {
    let mut current = entity;
    loop {
        if equipment_item_query.get(current).is_ok() {
            return true;
        }
        let Ok(child_of) = parent_query.get(current) else {
            return false;
        };
        current = child_of.parent();
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
#[path = "../../../tests/unit/character_customization_tests.rs"]
mod tests;
