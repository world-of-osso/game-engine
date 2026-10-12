use crate::housing::HousingAuthority;
use crate::housing_editor::EditorInteraction;
use shared::protocol::{
    DecorTransform, HouseLayout, HousingCommand, HousingState, PlacedDecor, StoredDecor,
};

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
    assert!(editor.begin_place(&auth, 999).is_err());
    editor.begin_place(&auth, 1693).unwrap();
    assert_eq!(
        editor.commit_at(&auth, [4.0, -2.0, 8.0]).unwrap(),
        HousingCommand::PlaceDecor {
            house_id: 41,
            decor_id: 1693,
            transform: DecorTransform {
                position: [4.0, -2.0, 8.0],
                yaw: 0.0
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
    assert!(editor.begin_place(&visitor, 1693).is_err());
    assert!(editor.commit_at(&visitor, [0.0; 3]).is_err());
    assert!(editor.rotate(&visitor, 1.0).is_err());
    assert!(editor.remove(&visitor).is_err());
}
