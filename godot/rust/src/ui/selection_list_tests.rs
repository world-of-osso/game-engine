use super::scroll_lists::tests;
use super::*;
use game_engine_ui_model::selection_list::{highlight_layer_shown, is_selection_row};
use game_engine_ui_pvp::battleground::{PvpView, pvp_screen};
use ui_toolkit::screen::{Screen, SharedContext};

fn pvp_model() -> RegistryModel {
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
    model
}

fn frame_id(model: &RegistryModel, name: &str) -> u64 {
    model.registry.get_by_name(name).expect(name)
}

fn hover_shown(model: &RegistryModel, row: &str) -> bool {
    let id = frame_id(model, &format!("{row}RowHighlight"));
    highlight_layer_shown(&model.registry, model.registry.get(id).unwrap())
}

#[test]
fn selection_row_hover_and_keyboard_focus_draw_without_push_button_chrome() {
    let mut model = pvp_model();
    let row = "HonorFrameSpecificBattlegroundButton699";
    let id = frame_id(&model, row);
    assert!(is_selection_row(
        &model.registry,
        model.registry.get(id).unwrap()
    ));
    assert!(
        parts::project_images(model.registry.get(id).unwrap(), 324.0, 40.0).is_empty(),
        "an idle row draws no button chrome of its own"
    );
    let selected = frame_id(
        &model,
        "HonorFrameSpecificBattlegroundButton1014RowSelected",
    );
    let artwork = parts::project_images(model.registry.get(selected).unwrap(), 320.0, 39.0);
    assert_eq!(artwork.len(), 1);
    assert!(
        artwork[0].additive,
        "PvPMegaQueue selected art is ADD (Blizzard_PVPUI.xml:209) and keeps the label visible"
    );
    let hover = frame_id(&model, &format!("{row}RowHighlight"));
    assert!(parts::project_images(model.registry.get(hover).unwrap(), 320.0, 39.0)[0].additive);
    let mut sliders = VecDeque::new();
    assert!(!hover_shown(&model, row));
    model.update_input_widgets(UiInput::Hover(id, true), &mut sliders);
    assert!(hover_shown(&model, row));
    model.update_input_widgets(UiInput::Hover(id, false), &mut sliders);
    assert!(!hover_shown(&model, row));
    model.update_input_widgets(UiInput::Focus(id), &mut sliders);
    assert!(hover_shown(&model, row), "keyboard focus must be visible");
    model.update_input_widgets(UiInput::Blur(id), &mut sliders);
    assert!(!hover_shown(&model, row));
    let last = frame_id(&model, "HonorFrameSpecificBattlegroundButton7");
    model.update_input_widgets(UiInput::Focus(last), &mut sliders);
    tests::rebuild(&mut model);
    let row_rect = tests::rect(&model, "HonorFrameSpecificBattlegroundButton7").unwrap();
    let list = tests::rect(&model, game_engine_ui_pvp::battleground::SPECIFIC_LIST).unwrap();
    assert!(
        row_rect.y >= list.y && row_rect.y + row_rect.height <= list.y + list.height,
        "keyboard focus reveals clipped rows"
    );
    model.shared.insert(PvpView {
        visible: true,
        pending: true,
        ..Default::default()
    });
    tests::rebuild(&mut model);
    let mut actions = VecDeque::new();
    model.queue_click_action(&mut actions, id);
    assert!(
        actions.is_empty(),
        "disabled rows ignore pointer and keyboard activation"
    );
    model.update_input_widgets(UiInput::Hover(id, true), &mut sliders);
    assert!(!hover_shown(&model, row), "disabled rows do not hover");
}

#[test]
fn selection_row_authored_rows_stay_keyboard_rows_without_template_hover() {
    use game_engine_ui_model::selection_list::{AUTHORED, row};
    let mut registry = ui_parent::UiParent::for_viewport(800.0, 600.0).registry();
    let shared = SharedContext::new();
    let mut screen = Screen::new(|_| {
        let mut elements = row(
            "AuthoredRow".into(),
            "pick",
            true,
            (0.0, 0.0, 200.0, 20.0),
            (&AUTHORED, false),
            Vec::new(),
        );
        elements.extend(
            ui_toolkit::rsx! { button { name: "PlainButton", width: 80.0,
            height: 20.0, top: 40.0, pos_type: "absolute", button_default_skin: false,
            onclick: "act" } },
        );
        elements
    });
    screen.sync(&shared, &mut registry);
    let id = registry.get_by_name("AuthoredRow").unwrap();
    assert!(
        is_selection_row(&registry, registry.get(id).unwrap()),
        "keyboard eligibility does not depend on which hover art a template draws"
    );
    let plain = registry.get_by_name("PlainButton").unwrap();
    assert!(!is_selection_row(&registry, registry.get(plain).unwrap()));
}
