use super::*;
use game_engine_ui_model::battleground::{PvpView, pvp_screen};
use ui_toolkit::screen::{Screen, SharedContext};

#[test]
fn selection_row_hover_and_keyboard_focus_draw_without_push_button_chrome() {
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
        registry: ui_parent::UiParent::for_viewport(1280.0, 720.0).registry(),
        icon_masks: Default::default(),
        postsetup: ScreenPostsetup::None,
    };
    tests::rebuild(&mut model);
    let id = model
        .registry
        .get_by_name("HonorFrameSpecificBattlegroundButton699")
        .unwrap();
    let draw =
        |model: &RegistryModel| parts::project_images(model.registry.get(id).unwrap(), 324.0, 40.0);
    assert!(draw(&model).is_empty());
    let mut sliders = VecDeque::new();
    model.update_input_widgets(UiInput::Hover(id, true), &mut sliders);
    assert_eq!(draw(&model).len(), 1);
    model.update_input_widgets(UiInput::Hover(id, false), &mut sliders);
    assert!(draw(&model).is_empty());
    model.update_input_widgets(UiInput::Focus(id), &mut sliders);
    assert_eq!(draw(&model).len(), 1, "keyboard focus must be visible");
    model.update_input_widgets(UiInput::Blur(id), &mut sliders);
    assert!(draw(&model).is_empty());
}
