//! PVPUIFrame's SpecificScrollBox pans by one row per stepper press and two per
//! wheel notch (ScrollBoxListView.lua:637-661), exposing all implemented queue rows.

use game_engine_ui_model::battleground::{PvpView, SPECIFIC_LIST, pvp_screen};
use ui_toolkit::screen::{Screen, SharedContext};

use super::tests::{rebuild, rect};
use super::*;
use crate::ui::ui_parent::UiParent;
use crate::ui::{RegistryModel, ScreenPostsetup};

const EYE_ROW: &str = "HonorFrameSpecificBattlegroundButton7";
const SIXTH_ROW: &str = "HonorFrameSpecificBattlegroundButton699";

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
fn specific_list_stepper_and_wheel_reveal_all_rows_beyond_the_retail_viewport() {
    let mut model = queue_frame();
    let top = rect(&model, EYE_ROW).unwrap().y;
    let list = rect(&model, SPECIFIC_LIST).unwrap();
    let content = rect(&model, "HonorFrameSpecificScrollChild").unwrap();
    let max_offset = (content.height - list.height).max(0.0);
    let step = game_engine_ui_model::battleground::pvp_pan_extent(SPECIFIC_LIST).unwrap() as f32;
    let rows = [
        EYE_ROW,
        SIXTH_ROW,
        "HonorFrameSpecificBattlegroundButton708",
        "HonorFrameSpecificBattlegroundButton894",
        "HonorFrameSpecificBattlegroundButton1110",
        "HonorFrameSpecificBattlegroundButton1",
        "HonorFrameSpecificBattlegroundButton30",
        "HonorFrameArena2v2Button",
        "HonorFrameArena3v3Button",
    ];
    let required_offsets: Vec<_> = rows
        .iter()
        .map(|name| {
            let row = rect(&model, name).unwrap();
            let overflow = (row.y + row.height - (list.y + list.height)).max(0.0);
            (overflow / step).ceil() * step
        })
        .collect();
    assert!(
        row_inside_list(&model, EYE_ROW),
        "Retail292px viewport includes fifth row"
    );
    assert!(max_offset > 0.0, "Concrete queue inventory must overflow");
    let forward = model
        .registry
        .get_by_name(&forward_stepper_name(SPECIFIC_LIST))
        .unwrap();
    let mut offset = 0.0;
    for (name, required_offset) in rows.iter().zip(required_offsets) {
        while offset < required_offset.min(max_offset) {
            assert!(press_stepper(&mut model, forward));
            rebuild(&mut model);
            offset = (offset + step).min(max_offset);
            assert_eq!(rect(&model, EYE_ROW).unwrap().y, top - offset);
        }
        assert!(row_inside_list(&model, name), "{name} visible at {offset}");
    }
    assert_eq!(offset, max_offset);
    press_stepper(&mut model, forward);
    rebuild(&mut model);
    assert_eq!(rect(&model, EYE_ROW).unwrap().y, top - max_offset);

    let mut model = queue_frame();
    assert!(wheel(&mut model, SPECIFIC_LIST, false));
    rebuild(&mut model);
    assert_eq!(rect(&model, EYE_ROW).unwrap().y, top - 2.0 * step);
    assert!(row_inside_list(&model, SIXTH_ROW));
    assert!(!row_inside_list(
        &model,
        "HonorFrameSpecificBattlegroundButton1"
    ));
    let forward = model
        .registry
        .get_by_name(&forward_stepper_name(SPECIFIC_LIST))
        .unwrap();
    assert!(press_stepper(&mut model, forward));
    rebuild(&mut model);
    assert!(row_inside_list(
        &model,
        "HonorFrameSpecificBattlegroundButton1"
    ));
    assert!(!row_inside_list(
        &model,
        "HonorFrameSpecificBattlegroundButton30"
    ));
    assert!(press_stepper(&mut model, forward));
    rebuild(&mut model);
    assert!(row_inside_list(
        &model,
        "HonorFrameSpecificBattlegroundButton30"
    ));
    assert!(!row_inside_list(&model, "HonorFrameArena2v2Button"));
}
