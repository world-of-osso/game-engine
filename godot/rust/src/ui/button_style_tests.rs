//! Character-creation push buttons must project exactly like character-select buttons.

use game_engine_ui_model::char_create_component::{CharCreateMode, CharCreateUiState};
use game_engine_ui_model::char_select_component::BACK_BUTTON as SELECT_BACK;
use game_engine_ui_model::{CharacterCreateModel, CharacterSelectModel};
use ui_toolkit::frame::{Frame, WidgetData};
use ui_toolkit::widgets::button::ButtonState;

use super::parts::{project_button_text, project_images};

const STATES: [(ButtonState, bool); 4] = [
    (ButtonState::Normal, false),
    (ButtonState::Normal, true),
    (ButtonState::Pushed, false),
    (ButtonState::Disabled, false),
];

fn frame_named(registry: &ui_toolkit::registry::FrameRegistry, name: &str) -> Frame {
    let id = registry
        .get_by_name(name)
        .unwrap_or_else(|| panic!("missing {name}"));
    registry.get(id).unwrap().clone()
}

fn in_state(frame: &Frame, state: ButtonState, hovered: bool) -> Frame {
    let mut frame = frame.clone();
    let Some(WidgetData::Button(button)) = &mut frame.widget_data else {
        panic!("{:?} is not a button", frame.name);
    };
    button.state = state;
    button.hovered = hovered;
    frame
}

/// Texture sources and crops of the projected skin, independent of button size.
fn skin(frame: &Frame) -> Vec<String> {
    let parts = project_images(frame, 200.0, 48.0);
    assert!(!parts.is_empty(), "{:?} projects no skin", frame.name);
    parts
        .iter()
        .map(|part| format!("{:?} {:?} {:?}", part.source, part.crop, part.color))
        .collect()
}

fn create_frames(mode: CharCreateMode, names: &[&str]) -> Vec<Frame> {
    let mut model = CharacterCreateModel::new(1920.0, 1080.0);
    model.shared.insert(CharCreateUiState {
        mode,
        random_name_available: true,
        ..Default::default()
    });
    model.sync();
    names
        .iter()
        .map(|name| frame_named(&model.registry, name))
        .collect()
}

#[test]
fn character_create_buttons_use_character_select_button_skin() {
    let mut select = CharacterSelectModel::new(1920.0, 1080.0);
    select.sync();
    let reference = frame_named(&select.registry, SELECT_BACK.0);
    let mut buttons = create_frames(
        CharCreateMode::RaceClass,
        &["CharCreateBack", "CharCreateNext"],
    );
    buttons.extend(create_frames(
        CharCreateMode::Customize,
        &[
            "CharCreateButton",
            "CharCreateRandomize",
            "CharCreateRandomName",
            "Camera_reset",
            "Camera_zoom_out",
            "Camera_zoom_in",
            "Camera_rotate_left",
            "Camera_rotate_right",
        ],
    ));
    for button in &buttons {
        for (state, hovered) in STATES {
            assert_eq!(
                skin(&in_state(button, state, hovered)),
                skin(&in_state(&reference, state, hovered)),
                "{:?} {state:?} hovered={hovered}",
                button.name
            );
        }
    }
}

#[test]
fn character_create_navigation_labels_use_character_select_button_text() {
    let mut select = CharacterSelectModel::new(1920.0, 1080.0);
    select.sync();
    let reference = frame_named(&select.registry, SELECT_BACK.0);
    let race = create_frames(
        CharCreateMode::RaceClass,
        &["CharCreateBack", "CharCreateNext"],
    );
    let customize = create_frames(CharCreateMode::Customize, &["CharCreateButton"]);
    for (button, label) in race
        .iter()
        .zip(["Back", "Customize"])
        .chain(customize.iter().zip(["Create Character"]))
    {
        for (state, hovered) in STATES {
            let text = project_button_text(&in_state(button, state, hovered))
                .unwrap_or_else(|| panic!("{:?} has no button text", button.name));
            let expected = project_button_text(&in_state(&reference, state, hovered)).unwrap();
            assert_eq!(text.content, label);
            assert_eq!(
                (text.font, text.color, text.justify_h, text.justify_v),
                (
                    expected.font,
                    expected.color,
                    expected.justify_h,
                    expected.justify_v
                ),
                "{:?} {state:?}",
                button.name
            );
        }
    }
}
