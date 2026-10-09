//! Key Bindings page: one button per action; left-click listens, the next key binds,
//! right-click unbinds, Escape cancels, a taken key moves (docs/specs/key-bindings.md
//! "Binding buttons").

use game_engine_core::client_options_data::{
    ClientOptionsFile, load_options_file_from_path, save_options_file_to_path,
};
use game_engine_core::input_bindings_data::{
    BindingKey, BindingSection, InputAction, InputBinding, InputBindingsData,
};
use game_engine_ui_model::options_menu_component::{OptionsCategory, keybinding_button_name};
use game_engine_ui_model::options_menu_data::{self as policy, OptionsModel};
use ui_toolkit::frame::WidgetData;

use super::keybinding_button_at;
use crate::ui::RegistryModel;
use crate::ui::scroll_lists::tests::{menu, options_model, rect, show};

const Q: InputBinding = InputBinding::Keyboard(BindingKey::KeyQ);
const S: InputBinding = InputBinding::Keyboard(BindingKey::KeyS);

fn keybindings(section: BindingSection) -> (OptionsModel, RegistryModel) {
    let model = options_model(OptionsCategory::Keybindings, section);
    let menu = menu(policy::build_view_model(&model));
    (model, menu)
}

fn redraw(model: &OptionsModel, menu: &mut RegistryModel) {
    show(menu, policy::build_view_model(model));
}

fn text(menu: &RegistryModel, name: &str) -> String {
    let id = menu.registry.get_by_name(name).expect(name);
    match menu.registry.get(id).unwrap().widget_data.as_ref() {
        Some(WidgetData::FontString(data)) => data.text.clone(),
        other => panic!("{name}: expected a font string, got {other:?}"),
    }
}

fn button_text(menu: &RegistryModel, action: InputAction) -> String {
    text(menu, &format!("KeybindingButtonText{}", action.key()))
}

fn output(menu: &RegistryModel) -> String {
    text(menu, "KeybindingOutput")
}

/// Left-click the action's button as the client does: the clicked frame's action.
fn left_click(model: &mut OptionsModel, menu: &mut RegistryModel, action: InputAction) {
    let id = menu
        .registry
        .get_by_name(&keybinding_button_name(action))
        .expect("binding button");
    let clicked = menu.click_action(id).expect("button has a click action");
    let listening = policy::parse_binding_rebind_action(&clicked).expect("a rebind action");
    policy::listen_for_binding(model, listening);
    redraw(model, menu);
}

fn button_centre(menu: &RegistryModel, action: InputAction) -> [f32; 2] {
    let id = menu
        .registry
        .get_by_name(&keybinding_button_name(action))
        .unwrap();
    let rect = menu.registry.get(id).unwrap().layout_rect.clone().unwrap();
    [rect.x + rect.width / 2.0, rect.y + rect.height / 2.0]
}

#[test]
fn left_click_then_a_key_binds_it_and_the_button_shows_it() {
    let defaults = InputBindingsData::default();
    assert!(
        InputAction::ALL
            .iter()
            .all(|a| defaults.binding(*a) != Some(Q))
    );
    let action = InputAction::MultiActionBar1Button1;
    let (mut model, mut menu) = keybindings(BindingSection::ActionBar2);
    assert_eq!(button_text(&menu, action), "Not Bound");
    assert_eq!(output(&menu), "");

    left_click(&mut model, &mut menu, action);
    assert_eq!(button_text(&menu, action), "Press a key\u{2026}");
    assert_eq!(
        output(&menu),
        "Assign Binding for \"Action Bar 2 Button 1\" or Press Escape to Cancel"
    );

    assert!(policy::capture_binding(&mut model, Q));
    redraw(&model, &mut menu);
    assert_eq!(model.draft_bindings.binding(action), Some(Q));
    assert_eq!(button_text(&menu, action), "Q");
    assert_eq!(output(&menu), "Key Bound Successfully");
    // Listening ended: another key binds nothing.
    assert!(!policy::capture_binding(&mut model, S));
    assert_eq!(model.draft_bindings.binding(action), Some(Q));
}

#[test]
fn right_click_on_a_button_unbinds_its_action() {
    let (mut model, mut menu) = keybindings(BindingSection::Movement);
    assert_eq!(button_text(&menu, InputAction::MoveForward), "W");
    let label = menu
        .registry
        .get_by_name(&format!(
            "KeybindingLabel{}",
            InputAction::MoveForward.key()
        ))
        .unwrap();
    let label = menu
        .registry
        .get(label)
        .unwrap()
        .layout_rect
        .clone()
        .unwrap();
    let on_label = [label.x + 4.0, label.y + label.height / 2.0];
    assert_eq!(
        keybinding_button_at(&menu.registry, BindingSection::Movement, on_label),
        None
    );

    let clicked = keybinding_button_at(
        &menu.registry,
        BindingSection::Movement,
        button_centre(&menu, InputAction::MoveBackward),
    )
    .expect("a binding button under the pointer");
    assert_eq!(clicked, InputAction::MoveBackward);
    policy::unbind_action(&mut model, clicked);
    redraw(&model, &mut menu);
    assert_eq!(
        model.draft_bindings.binding(InputAction::MoveBackward),
        None
    );
    assert_eq!(button_text(&menu, InputAction::MoveBackward), "Not Bound");
    assert_eq!(button_text(&menu, InputAction::MoveForward), "W");
}

#[test]
fn escape_cancels_listening_without_a_change() {
    let (mut model, mut menu) = keybindings(BindingSection::Movement);
    left_click(&mut model, &mut menu, InputAction::MoveForward);
    assert!(policy::cancel_binding_capture(&mut model));
    redraw(&model, &mut menu);
    assert_eq!(model.draft_bindings, InputBindingsData::default());
    assert_eq!(button_text(&menu, InputAction::MoveForward), "W");
    assert_eq!(output(&menu), "");
    // Not listening: Escape is left to close the panel.
    assert!(!policy::cancel_binding_capture(&mut model));
}

#[test]
fn a_key_bound_elsewhere_moves_and_names_the_action_that_lost_it() {
    let (mut model, mut menu) = keybindings(BindingSection::Movement);
    assert_eq!(
        model.draft_bindings.binding(InputAction::MoveBackward),
        Some(S)
    );
    left_click(&mut model, &mut menu, InputAction::MoveForward);
    assert!(policy::capture_binding(&mut model, S));
    redraw(&model, &mut menu);
    assert_eq!(
        model.draft_bindings.binding(InputAction::MoveForward),
        Some(S)
    );
    assert_eq!(
        model.draft_bindings.binding(InputAction::MoveBackward),
        None
    );
    assert_eq!(button_text(&menu, InputAction::MoveForward), "S");
    assert_eq!(button_text(&menu, InputAction::MoveBackward), "Not Bound");
    assert_eq!(output(&menu), "Action Move Backward is Now Unbound!");
    let view = policy::build_view_model(&model);
    assert!(view.options.bindings.output.unwrap().error);
}

#[test]
fn page_edits_survive_a_save_and_reload_of_the_options_file() {
    let (mut model, mut menu) = keybindings(BindingSection::Movement);
    left_click(&mut model, &mut menu, InputAction::MoveForward);
    policy::capture_binding(&mut model, S);
    policy::unbind_action(&mut model, InputAction::Jump);
    let snapshot = policy::apply_snapshot(&mut model);
    let file = ClientOptionsFile {
        bindings: snapshot.bindings,
        ..Default::default()
    };
    let path = std::env::temp_dir().join(format!(
        "keybindui-{}-options_settings.ron",
        std::process::id()
    ));
    save_options_file_to_path(&path, &file).unwrap();
    let loaded = load_options_file_from_path(&path).bindings;
    std::fs::remove_file(&path).unwrap();
    assert_eq!(loaded.binding(InputAction::MoveForward), Some(S));
    assert_eq!(loaded.binding(InputAction::MoveBackward), None);
    assert_eq!(loaded.binding(InputAction::Jump), None);
    assert_eq!(
        loaded.binding(InputAction::StrafeLeft),
        InputBindingsData::default().binding(InputAction::StrafeLeft)
    );
}

#[test]
fn sidebarbinds_every_binding_section_remains_visible_inside_the_page() {
    let (_, menu) = keybindings(BindingSection::ActionBar5);
    let tabs = rect(&menu, "KeybindingSectionTabs").unwrap();
    for section in BindingSection::ALL {
        let name = format!("KeybindingSection{}Button", section.key());
        let button = rect(&menu, &name).unwrap();
        assert!(button.x >= tabs.x, "{name} left of page");
        assert!(
            button.x + button.width <= tabs.x + tabs.width + 0.01,
            "{name} exceeds page: {button:?} vs {tabs:?}"
        );
    }
    for (section, action) in [
        (
            BindingSection::ActionBar4,
            InputAction::MultiActionBar3Button12,
        ),
        (
            BindingSection::ActionBar5,
            InputAction::MultiActionBar4Button12,
        ),
    ] {
        let (mut model, mut menu) = keybindings(section);
        for _ in 0..100 {
            if !crate::ui::scroll_lists::wheel(
                &mut menu,
                game_engine_ui_model::options_menu_component::OPTIONS_CONTENT_SCROLL,
                false,
            ) {
                break;
            }
            crate::ui::scroll_lists::tests::rebuild(&mut menu);
        }
        assert!(crate::ui::scroll_lists::tests::shown(
            &menu,
            &format!("KeybindingRow{}", action.key())
        ));
        assert_eq!(button_text(&menu, action), "Not Bound");
        left_click(&mut model, &mut menu, action);
        assert!(policy::capture_binding(&mut model, Q));
        redraw(&model, &mut menu);
        assert_eq!(button_text(&menu, action), "Q");
    }
}
