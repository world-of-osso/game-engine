//! PVPUIFrame's SpecificScrollBox pans by one 40-unit row per stepper press and two per
//! wheel notch (ScrollBoxListView.lua:637-661), exposing all implemented queue rows.

use game_engine_ui_model::battleground::{PvpView, SPECIFIC_LIST, pvp_screen};
use ui_toolkit::screen::{Screen, SharedContext};

use super::tests::{rebuild, rect};
use super::*;
use crate::ui::ui_parent::UiParent;
use crate::ui::{RegistryModel, ScreenPostsetup};

const EYE_ROW: &str = "HonorFrameSpecificBattlegroundButton7";
const LAST_ROW: &str = "HonorFrameSpecificBattlegroundButton699";

fn queue_frame() -> RegistryModel {
    game_engine_ui_model::paths::set_data_root(
        std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../data"),
    )
    .unwrap();
    let mut shared = SharedContext::new();
    shared.insert(PvpView {
        visible: true,
        ..Default::default()
    });
    let mut model = RegistryModel {
        screen: Screen::new(pvp_screen),
        shared,
        registry: UiParent::for_viewport(1280.0, 720.0).registry(),
        icon_masks: Default::default(),
        postsetup: ScreenPostsetup::None,
    };
    rebuild(&mut model);
    model
}

fn row_inside_list(model: &RegistryModel, name: &str) -> bool {
    let (row, list) = (
        rect(model, name).unwrap(),
        rect(model, SPECIFIC_LIST).unwrap(),
    );
    row.y >= list.y && row.y + row.height <= list.y + list.height
}

#[test]
fn specific_list_stepper_and_wheel_reveal_the_fifth_battleground_row() {
    let mut model = queue_frame();
    let top = rect(&model, EYE_ROW).unwrap().y;
    assert!(
        !row_inside_list(&model, EYE_ROW),
        "fifth row starts clipped"
    );
    let forward = model
        .registry
        .get_by_name(&forward_stepper_name(SPECIFIC_LIST))
        .unwrap();
    assert!(press_stepper(&mut model, forward));
    rebuild(&mut model);
    assert_eq!(rect(&model, EYE_ROW).unwrap().y, top - 40.0);
    assert!(row_inside_list(&model, EYE_ROW));
    press_stepper(&mut model, forward);
    rebuild(&mut model);
    assert_eq!(
        rect(&model, EYE_ROW).unwrap().y,
        top - 80.0,
        "sixth row requires a second step"
    );
    assert!(row_inside_list(&model, LAST_ROW));
    assert!(
        model
            .registry
            .get_by_name("HonorFrameSpecificBattlegroundButton708")
            .is_none()
    );
    assert!(
        model
            .registry
            .get_by_name("HonorFrameSpecificBattlegroundButton894")
            .is_none()
    );
    let end = rect(&model, EYE_ROW).unwrap().y;
    press_stepper(&mut model, forward);
    rebuild(&mut model);
    assert_eq!(
        rect(&model, EYE_ROW).unwrap().y,
        end,
        "clamped after the final row is visible"
    );
    assert!(row_inside_list(&model, LAST_ROW));

    let mut model = queue_frame();
    assert!(wheel(&mut model, SPECIFIC_LIST, false));
    rebuild(&mut model);
    assert_eq!(
        rect(&model, EYE_ROW).unwrap().y,
        top - 80.0,
        "a notch pans two rows to expose the sixth row"
    );
    assert!(row_inside_list(&model, LAST_ROW));
}
