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
fn housing_editor_uses_confirmed_storage_and_never_changes_quantity_optimistically() {
    let auth = authority(true);
    let mut editor = EditorInteraction::default();
    assert!(editor.begin_place(&auth, &bookcase(999)).is_err());
    editor.begin_place(&auth, &bookcase(1693)).unwrap();
    assert_eq!(
        editor.commit_at(&auth, [4.0, -2.0, 8.0]).unwrap(),
        HousingCommand::PlaceDecor {
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
