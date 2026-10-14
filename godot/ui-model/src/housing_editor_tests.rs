use crate::housing::HousingAuthority;
use crate::housing_editor::EditorInteraction;
use shared::protocol::{
    DecorTransform, HouseLayout, HousingCommand, HousingState, PlacedDecor, StoredDecor,
};

fn bookcase(id: u32) -> game_engine_core::housing::Decor {
    game_engine_core::housing::Decor {
        id,
        name: "Small Val'sharah Bookcase".into(),
        item_id: 245259,
        model_type: 1,
        model_fdid: 1096761,
        initial_rotation: [0.0; 3],
        initial_scale: 1.0,
        thumbnail_fdid: 7421877,
        order: 328,
        subcategories: vec![],
        sources: vec![],
        dyeable: false,
    }
}

fn authority(can_edit: bool) -> HousingAuthority {
    HousingAuthority {
        state: Some(HousingState {
            build_key: game_engine_core::housing::RETAIL_BUILD_KEY.into(),
            stored: vec![StoredDecor {
                decor_id: 1693,
                quantity: 1,
            }],
            layout: Some(HouseLayout {
                house_id: 41,
                can_edit,
                decor: vec![PlacedDecor {
                    dyes: Vec::new(),
                    instance_id: 7,
                    decor_id: 80,
                    transform: DecorTransform {
                        position: [1.0, 2.0, 3.0],
                        yaw: 0.5,
                        pitch: 0.0,
                        roll: 0.0,
                        scale: 1.0,
                    },
                }],
            }),
            ..Default::default()
        }),
        error: None,
    }
}

#[test]
fn housing_cleanup_hover_and_click_store_only_the_confirmed_target_without_optimistic_changes() {
    use crate::housing_editor::EditorMode;
    let auth = authority(true);
    let confirmed = auth.state.clone();
    let mut editor = EditorInteraction::default();
    editor.begin_place(&auth, &bookcase(1693)).unwrap();
    editor.set_mode(EditorMode::Cleanup);
    assert_eq!(editor.placing, None);
    editor.hover(&auth, Some(7), [100.0, 200.0]).unwrap();
    assert_eq!(editor.hovered, Some(7));
    assert_eq!(editor.selected, None);
    assert_eq!(editor.pointer_click(&auth, None, None).unwrap(), None);
    assert_eq!(
        editor.pointer_click(&auth, Some(7), None).unwrap(),
        Some(HousingCommand::RemoveDecor {
            house_id: 41,
            instance_id: 7
        })
    );
    assert_eq!(editor.selected, None);
    assert_eq!(editor.hovered, None);
    assert_eq!(auth.state, confirmed);
    assert!(editor.pointer_click(&auth, Some(999), None).is_err());
    editor.hover(&auth, Some(7), [50.0, 60.0]).unwrap();
    editor.set_mode(EditorMode::Basic);
    assert_eq!(editor.hovered, None);
}

#[test]
fn housing_cleanup_visitor_may_hover_but_cannot_remove_or_change_saved_layout() {
    use crate::housing_editor::EditorMode;
    let visitor = authority(false);
    let confirmed = visitor.state.clone();
    let mut editor = EditorInteraction::default();
    editor.set_mode(EditorMode::Cleanup);
    editor.hover(&visitor, Some(7), [100.0, 200.0]).unwrap();
    assert_eq!(editor.hovered, Some(7));
    assert!(
        editor
            .pointer_click(&visitor, Some(7), None)
            .unwrap_err()
            .contains("Visitors")
    );
    assert_eq!(visitor.state, confirmed);
    editor.hover(&visitor, None, [0.0; 2]).unwrap();
    assert_eq!(editor.hovered, None);
}

#[test]
fn housing_editor_uses_confirmed_storage_and_never_changes_quantity_optimistically() {
    let auth = authority(true);
    let mut editor = EditorInteraction::default();
    assert!(editor.begin_place(&auth, &bookcase(999)).is_err());
    editor.begin_place(&auth, &bookcase(1693)).unwrap();
    assert_eq!(
        editor.commit_at(&auth, [4.0, -2.0, 8.0]).unwrap(),
        HousingCommand::PlaceDecor {
            variant_id: None,
            house_id: 41,
            decor_id: 1693,
            transform: DecorTransform {
                position: [4.0, -2.0, 8.0],
                yaw: 0.0,
                pitch: 0.0,
                roll: 0.0,
                scale: 1.0,
            }
        }
    );
    assert_eq!(auth.quantity(1693), Some(1));
    assert!(editor.commit_at(&auth, [0.0; 3]).is_err());
}

#[test]
fn housing_editor_second_click_commits_movement_even_when_the_ray_still_hits_selected_decor() {
    let auth = authority(true);
    let mut editor = EditorInteraction::default();
    assert_eq!(
        editor
            .pointer_click(&auth, Some(7), Some([1.0, 2.0, 3.0]))
            .unwrap(),
        None
    );
    assert_eq!(editor.selected, Some(7));
    assert_eq!(
        editor
            .pointer_click(&auth, Some(7), Some([5.0, 2.0, 7.0]))
            .unwrap(),
        Some(HousingCommand::MoveDecor {
            house_id: 41,
            instance_id: 7,
            position: [5.0, 2.0, 7.0]
        })
    );
    let visitor = authority(false);
    assert_eq!(
        editor
            .pointer_click(&visitor, Some(7), Some([8.0, 2.0, 9.0]))
            .unwrap(),
        None
    );
    assert_eq!(
        visitor.state.unwrap().layout.unwrap().decor[0]
            .transform
            .position,
        [1.0, 2.0, 3.0]
    );
}

#[test]
fn housing_editor_saved_selection_moves_rotates_removes_and_visitor_is_read_only() {
    let auth = authority(true);
    let mut editor = EditorInteraction::default();
    editor.select(&auth, 7).unwrap();
    assert_eq!(
        editor.rotate(&auth, 0.25).unwrap(),
        HousingCommand::RotateDecor {
            house_id: 41,
            instance_id: 7,
            yaw: 0.75
        }
    );
    assert_eq!(
        editor.commit_at(&auth, [5.0, -2.0, 7.0]).unwrap(),
        HousingCommand::MoveDecor {
            house_id: 41,
            instance_id: 7,
            position: [5.0, -2.0, 7.0]
        }
    );
    assert_eq!(
        editor.remove(&auth).unwrap(),
        HousingCommand::RemoveDecor {
            house_id: 41,
            instance_id: 7
        }
    );
    let visitor = authority(false);
    editor.select(&visitor, 7).unwrap();
    assert!(editor.begin_place(&visitor, &bookcase(1693)).is_err());
    assert!(editor.commit_at(&visitor, [0.0; 3]).is_err());
    assert!(editor.rotate(&visitor, 1.0).is_err());
    assert!(editor.remove(&visitor).is_err());
}

#[test]
fn housing_expert_mouse_drag_stages_from_press_not_previous_motion_and_never_saves_on_release() {
    use crate::housing_editor::{EditorMode, PrecisionSubmode};
    let auth = authority(true);
    let confirmed = auth.state.clone();
    let mut editor = EditorInteraction::default();
    editor.set_mode(EditorMode::Expert);
    editor.select(&auth, 7).unwrap();
    editor.begin_mouse_drag(&auth).unwrap();
    editor.update_mouse_drag(1.5).unwrap();
    editor.update_mouse_drag(2.0).unwrap();
    assert_eq!(editor.expert_transform().unwrap().position, [3.0, 2.0, 3.0]);
    assert!(editor.is_manipulating());
    editor.end_mouse_drag();
    assert!(!editor.is_manipulating());
    assert_eq!(auth.state, confirmed);
    editor.set_submode(PrecisionSubmode::Rotate);
    editor.next_axis();
    editor.begin_mouse_drag(&auth).unwrap();
    editor.update_mouse_drag(0.25).unwrap();
    assert_eq!(editor.expert_transform().unwrap().yaw, 0.75);
    editor.end_mouse_drag();
    editor.set_submode(PrecisionSubmode::Scale);
    editor.scale_allowed = true;
    editor.begin_mouse_drag(&auth).unwrap();
    editor.update_mouse_drag(0.5).unwrap();
    let valid = editor.expert_transform().unwrap();
    assert_eq!(valid.scale, 1.5);
    assert!(editor.update_mouse_drag(-2.0).is_err());
    assert!(editor.update_mouse_drag(f32::NAN).is_err());
    assert_eq!(editor.expert_transform(), Some(valid));
    editor.end_mouse_drag();
    assert_eq!(
        editor.commit_expert(&auth).unwrap(),
        HousingCommand::TransformDecor {
            house_id: 41,
            instance_id: 7,
            transform: valid,
        }
    );
    assert_eq!(auth.state, confirmed);
    editor.cancel();
    assert_eq!(editor.expert_transform(), None);
    assert!(!editor.is_manipulating());
}

#[test]
fn housing_expert_mouse_drag_refuses_visitors_wmo_scale_and_unselected_or_basic_targets() {
    use crate::housing_editor::{EditorMode, PrecisionSubmode};
    let auth = authority(true);
    let mut editor = EditorInteraction::default();
    editor.select(&auth, 7).unwrap();
    assert!(editor.begin_mouse_drag(&auth).is_err());
    editor.set_mode(EditorMode::Expert);
    assert!(editor.begin_mouse_drag(&auth).is_err());
    editor.select(&auth, 7).unwrap();
    editor.set_submode(PrecisionSubmode::Scale);
    assert!(editor.begin_mouse_drag(&auth).is_err());
    editor.set_submode(PrecisionSubmode::Translate);
    let visitor = authority(false);
    editor.select(&visitor, 7).unwrap();
    assert!(editor.begin_mouse_drag(&visitor).is_err());
    assert!(editor.update_mouse_drag(1.0).is_err());
}

#[test]
fn housing_expert_stages_xyz_rotation_scale_and_cancel_without_storage_mutation() {
    use crate::housing_editor::{EditorMode, PrecisionSubmode, RotationAxis};
    let auth = authority(true);
    let mut editor = EditorInteraction::default();
    editor.set_mode(EditorMode::Expert);
    editor.select(&auth, 7).unwrap();
    assert_eq!(editor.rotation_axis(), RotationAxis::X);
    editor.set_submode(PrecisionSubmode::Rotate);
    editor.increment_expert(1.0).unwrap();
    assert_eq!(editor.expert_transform().unwrap().pitch, 1.0);
    editor.next_axis();
    assert_eq!(editor.rotation_axis(), RotationAxis::Y);
    editor.increment_expert(0.25).unwrap();
    editor.next_axis();
    assert_eq!(editor.rotation_axis(), RotationAxis::Z);
    editor.increment_expert(-0.5).unwrap();
    editor.next_axis();
    assert_eq!(editor.rotation_axis(), RotationAxis::X);
    editor.set_submode(PrecisionSubmode::Scale);
    assert!(editor.increment_expert(0.25).is_err());
    editor.scale_allowed = true; // Authenticated M2 ModelType1 in this fixture.
    editor.increment_expert(0.25).unwrap();
    let draft = editor.expert_transform().unwrap();
    assert_eq!(
        (draft.pitch, draft.yaw, draft.roll, draft.scale),
        (1.0, 0.75, -0.5, 1.25)
    );
    assert_eq!(
        auth.state.as_ref().unwrap().layout.as_ref().unwrap().decor[0]
            .transform
            .yaw,
        0.5
    );
    assert_eq!(auth.quantity(1693), Some(1));
    assert_eq!(
        editor.commit_expert(&auth).unwrap(),
        HousingCommand::TransformDecor {
            house_id: 41,
            instance_id: 7,
            transform: draft
        }
    );
    editor.cancel();
    assert!(editor.expert_transform().is_none());
    assert_eq!(editor.mode, EditorMode::Expert);
    let visitor = authority(false);
    editor.select(&visitor, 7).unwrap();
    assert!(editor.commit_expert(&visitor).is_err());
}

#[test]
fn housing_basic_catalog_click_ignores_initial_release_then_commits_overlay_release() {
    let auth = authority(true);
    let saved = auth.state.clone();
    let mut editor = EditorInteraction::default();
    editor.begin_place(&auth, &bookcase(1693)).unwrap();
    assert_eq!(
        editor
            .basic_pointer_release(&auth, Some([4.0, 2.0, 8.0]))
            .unwrap(),
        None
    );
    assert_eq!(editor.placing, Some(1693));
    assert!(editor.arm_move_overlay());
    assert_eq!(editor.placing, Some(1693));
    assert!(
        matches!(editor.basic_pointer_release(&auth,Some([4.0,2.0,8.0])).unwrap(), Some(HousingCommand::PlaceDecor {decor_id:1693,transform,..}) if transform.position==[4.0,2.0,8.0])
    );
    assert_eq!(auth.state, saved);
}

#[test]
fn housing_basic_owned_preview_rotation_stays_local_until_real_placement_commit() {
    let auth = authority(true);
    let saved = auth.state.clone();
    let mut editor = EditorInteraction::default();
    editor.begin_place(&auth, &bookcase(1693)).unwrap();
    editor.rotate_placement(&auth, 0.25).unwrap();
    assert_eq!(
        editor.placement_transform([4.0, 2.0, 8.0]).unwrap().yaw,
        0.25
    );
    assert_eq!(auth.state, saved);
    assert!(editor.rotate_placement(&auth, f32::INFINITY).is_err());
    assert!(editor.rotate_placement(&authority(false), 0.25).is_err());
    assert!(
        matches!(editor.commit_at(&auth,[4.0,2.0,8.0]).unwrap(),HousingCommand::PlaceDecor {decor_id:1693,transform,..} if transform.yaw==0.25)
    );
    assert!(editor.rotate_placement(&auth, 0.25).is_err());
}

#[test]
fn housing_basic_owned_catalog_drag_commits_first_release_and_cancellation_clears_capture() {
    let auth = authority(true);
    let saved = auth.state.clone();
    let mut editor = EditorInteraction::default();
    editor.begin_place_drag(&auth, &bookcase(1693)).unwrap();
    assert!(
        matches!(editor.basic_pointer_release(&auth,Some([8.0,2.0,4.0])).unwrap(),Some(HousingCommand::PlaceDecor {decor_id:1693,transform,..}) if transform.position==[8.0,2.0,4.0])
    );
    assert_eq!(auth.state, saved);
    editor.begin_place_drag(&auth, &bookcase(1693)).unwrap();
    editor.cancel();
    assert_eq!(
        editor
            .basic_pointer_release(&auth, Some([8.0, 2.0, 4.0]))
            .unwrap(),
        None
    );
    assert!(
        editor
            .begin_place_drag(&authority(false), &bookcase(1693))
            .is_err()
    );
    assert!(editor.begin_place_drag(&auth, &bookcase(999)).is_err());
}

#[test]
fn housing_basic_move_commits_only_armed_overlay_release_and_keeps_invalid_preview() {
    let auth = authority(true);
    let mut editor = EditorInteraction::default();
    editor.select(&auth, 7).unwrap();
    assert_eq!(
        editor
            .basic_pointer_release(&auth, Some([2.0, 2.0, 3.0]))
            .unwrap(),
        None
    );
    assert!(editor.arm_move_overlay());
    assert!(editor.basic_pointer_release(&auth, None).is_err());
    assert_eq!(editor.selected, Some(7));
    assert!(editor.arm_move_overlay());
    assert_eq!(
        editor
            .basic_pointer_release(&auth, Some([9.0, 2.0, 8.0]))
            .unwrap(),
        Some(HousingCommand::MoveDecor {
            house_id: 41,
            instance_id: 7,
            position: [9.0, 2.0, 8.0]
        })
    );
}
