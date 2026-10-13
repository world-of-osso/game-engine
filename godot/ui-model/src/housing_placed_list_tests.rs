use crate::housing::{HousingAuthority, HousingDashboard, HousingSurface};
use crate::housing_editor::EditorMode;
use game_engine_core::housing::HousingCatalog;
use shared::protocol::{DecorTransform, HouseLayout, HousingCommand, HousingState, PlacedDecor};
use std::sync::Arc;

fn fixture() -> (HousingDashboard, HousingAuthority) {
    let data = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../data");
    let mut model = HousingDashboard::new(Arc::new(
        HousingCatalog::load(&data.join("cache/housing.sqlite")).unwrap(),
    ));
    model.surface = HousingSurface::Editor;
    model.editor.set_mode(EditorMode::Expert);
    let transform = DecorTransform {
        position: [-1000.0, -2.0, 996.0],
        yaw: 0.0,
        pitch: 0.0,
        roll: 0.0,
        scale: 1.0,
    };
    let authority = HousingAuthority {
        state: Some(HousingState {
            build_key: game_engine_core::housing::RETAIL_BUILD_KEY.into(),
            layout: Some(HouseLayout {
                house_id: 1,
                can_edit: true,
                decor: vec![
                    PlacedDecor {
                        instance_id: 26,
                        decor_id: 1693,
                        transform,
                        dyes: vec![],
                    },
                    PlacedDecor {
                        instance_id: 41,
                        decor_id: 2330,
                        transform,
                        dyes: vec![],
                    },
                ],
            }),
            ..Default::default()
        }),
        error: None,
    };
    (model, authority)
}

#[test]
fn housing_placed_list_sorts_real_layout_and_keeps_selection_menu_and_authority_separate() {
    let (mut model, authority) = fixture();
    let before = authority.clone();
    assert_eq!(
        model
            .placed_list_entries(&authority)
            .unwrap()
            .iter()
            .map(|(placed, _)| placed.instance_id)
            .collect::<Vec<_>>(),
        vec![41, 26]
    );
    model.toggle_placed_list().unwrap();
    assert!(model.placed_list.open);
    model.toggle_placed_list_selection(&authority, 26).unwrap();
    assert_eq!(model.editor.selected, Some(26));
    assert!(model.placed_list.open);
    model.toggle_placed_list_selection(&authority, 26).unwrap();
    assert_eq!(model.editor.selected, None);
    model
        .open_placed_list_menu(&authority, 41, [100.0, 200.0])
        .unwrap();
    assert_eq!(model.placed_list.menu, Some((41, [100.0, 200.0])));
    assert_eq!(
        model.placed_list_remove_request(&authority).unwrap(),
        HousingCommand::RemoveDecor {
            house_id: 1,
            instance_id: 41
        }
    );
    assert_eq!(authority, before);
    model
        .placed_list
        .set_position([2000.0, 2000.0], [1280.0, 720.0])
        .unwrap();
    assert_eq!(model.placed_list.position, Some([880.0, 320.0]));
    assert!(
        model
            .placed_list
            .set_position([f32::NAN, 0.0], [1280.0, 720.0])
            .is_err()
    );
    assert_eq!(model.placed_list.position, Some([880.0, 320.0]));
    model.close_placed_list();
    assert!(!model.placed_list.open);
    assert_eq!(model.placed_list.menu, None);
}

#[test]
fn housing_placed_list_refuses_stale_or_unknown_sources_and_never_turns_visitor_selection_into_editing()
 {
    let (mut model, mut authority) = fixture();
    model.toggle_placed_list().unwrap();
    authority
        .state
        .as_mut()
        .unwrap()
        .layout
        .as_mut()
        .unwrap()
        .can_edit = false;
    model.toggle_placed_list_selection(&authority, 26).unwrap();
    assert_eq!(model.editor.selected, Some(26));
    assert!(model.editor.expert_transform().is_none());
    model
        .open_placed_list_menu(&authority, 26, [100.0, 200.0])
        .unwrap();
    assert!(model.placed_list_remove_request(&authority).is_err());
    assert!(
        model
            .open_placed_list_menu(&authority, 999, [100.0, 200.0])
            .is_err()
    );
    authority
        .state
        .as_mut()
        .unwrap()
        .layout
        .as_mut()
        .unwrap()
        .decor
        .retain(|placed| placed.instance_id != 26);
    model.reconcile_placed_list(&authority);
    assert_eq!(model.editor.selected, None);
    assert_eq!(model.placed_list.menu, None);
    assert_eq!(model.placed_list_entries(&authority).unwrap().len(), 1);
    authority
        .state
        .as_mut()
        .unwrap()
        .layout
        .as_mut()
        .unwrap()
        .decor[0]
        .decor_id = 999999;
    assert!(model.placed_list_entries(&authority).is_err());
    model.editor.set_mode(EditorMode::Basic);
    assert!(model.toggle_placed_list().is_err());
}
