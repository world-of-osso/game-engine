use super::{
    CharacterCustomizationSelection, apply_exact_geoset_overrides,
    apply_explicit_equipment_overlays, apply_hidden_geoset_groups, collect_active_geosets,
    collect_appearance_materials, component_sections_for_slot, group_zero_visible,
    merge_overlay_texture_sets, replacement_texture_for_batch,
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
    assert!(group_zero_visible(28, 2));

    assert!(!group_zero_visible(5, 2));
    assert!(!group_zero_visible(16, 2));
    assert!(!group_zero_visible(17, 2));
    assert!(!group_zero_visible(18, 2));
}

#[test]
fn exact_geoset_override_zero_only_hides_the_exact_mesh_part() {
    assert!(!apply_exact_geoset_overrides(2000, true, &[(20, 0)]));
    assert!(apply_exact_geoset_overrides(2001, true, &[(20, 0)]));
}

#[test]
fn exact_geoset_override_enables_only_the_matching_mesh_part() {
    assert!(apply_exact_geoset_overrides(2202, false, &[(22, 2)]));
    assert!(!apply_exact_geoset_overrides(2201, false, &[(22, 2)]));
}

#[test]
fn face_materials_resolve_against_selected_skin_color() {
    let db = CustomizationDb::load(Path::new("data"));
    let face0 = resolved_face_target_fdids(selection_with_skin_color(0), &db);
    let face1 = resolved_face_target_fdids(selection_with_skin_color(1), &db);

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
                eye_color: 0,
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

    let replacement =
        replacement_texture_for_batch(Some(6), &body, Some(&head), Some(&hair), None, None);

    assert_eq!(replacement, Some(hair));
}

#[test]
fn replacement_texture_for_type_two_batches_uses_cape_texture() {
    let mut images = Assets::<Image>::default();
    let body = images.add(Image::default());
    let head = images.add(Image::default());
    let hair = images.add(Image::default());
    let cape = images.add(Image::default());

    let replacement =
        replacement_texture_for_batch(Some(2), &body, Some(&head), Some(&hair), None, Some(&cape));

    assert_eq!(replacement, Some(cape));
}

#[test]
fn replacement_texture_for_type_nineteen_batches_uses_eye_texture() {
    let mut images = Assets::<Image>::default();
    let body = images.add(Image::default());
    let head = images.add(Image::default());
    let hair = images.add(Image::default());
    let eye = images.add(Image::default());

    let replacement =
        replacement_texture_for_batch(Some(19), &body, Some(&head), Some(&hair), Some(&eye), None);

    assert_eq!(replacement, Some(eye));
}

#[test]
fn feet_slot_maps_to_foot_component_section() {
    assert_eq!(
        component_sections_for_slot(EquipmentVisualSlot::Feet),
        &[6u8, 7] as &[u8]
    );
}

#[test]
fn human_male_eye_color_selection_provides_default_eye_texture() {
    let db = CustomizationDb::load(Path::new("data"));
    let selection = CharacterCustomizationSelection {
        race: 1,
        class: 1,
        sex: 0,
        appearance: CharacterAppearance {
            sex: 0,
            skin_color: 2,
            face: 3,
            eye_color: 0,
            hair_style: 4,
            hair_color: 5,
            facial_style: 1,
        },
    };

    let materials = collect_appearance_materials(selection, &db);

    assert!(
        materials
            .iter()
            .any(|(target_id, fdid)| *target_id == 25 && *fdid == 3484643),
        "expected current human male appearance to resolve the default eye texture: {materials:?}"
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
            eye_color: 0,
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
            eye_color: 0,
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
