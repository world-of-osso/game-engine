use game_engine_ui_model::CharacterSelectModel;
use game_engine_ui_model::char_select_component::{
    BACK_BUTTON, CHAR_LIST_PANEL, CHAR_SELECT_ROOT, CREATE_CHAR_BUTTON, CharDisplayEntry,
    CharSelectAction, CharSelectState, DELETE_CANCEL_BUTTON, DELETE_CHAR_BUTTON,
    DELETE_CONFIRM_BUTTON, DELETE_CONFIRM_DIALOG, DELETE_CONFIRM_INPUT, DeleteConfirmUiState,
    ENTER_WORLD_BUTTON, SELECTED_NAME_TEXT, STATUS_TEXT,
};
use ui_toolkit::frame::{Dimension, WidgetData};
use ui_toolkit::layout_values::{PositionType, Val};
use ui_toolkit::widgets::button::ButtonState;
use ui_toolkit::widgets::texture::TextureSource;

fn text(model: &CharacterSelectModel, name: &str) -> String {
    let frame = model
        .registry
        .get(model.registry.get_by_name(name).unwrap())
        .unwrap();
    let Some(WidgetData::FontString(label)) = &frame.widget_data else {
        panic!("{name} is not a label")
    };
    label.text.clone()
}

fn action(model: &CharacterSelectModel, name: &str) -> Option<CharSelectAction> {
    let frame = model
        .registry
        .get(model.registry.get_by_name(name).unwrap())
        .unwrap();
    frame.onclick.as_deref().and_then(CharSelectAction::parse)
}

fn roster(selected_index: Option<usize>) -> CharSelectState {
    CharSelectState {
        characters: vec![
            CharDisplayEntry {
                name: "Alyra".into(),
                info: "Level 70 Paladin".into(),
                status: "Stormwind".into(),
            },
            CharDisplayEntry {
                name: "Borin".into(),
                info: "Level 42 Shaman".into(),
                status: "Orgrimmar".into(),
            },
        ],
        selected_index,
        selected_name: selected_index
            .map_or("Character Selection", |i| {
                if i == 0 { "Alyra" } else { "Borin" }
            })
            .into(),
        status_text: "Choose a hero".into(),
    }
}

#[test]
fn authored_roster_retains_names_layout_resources_and_actions() {
    let mut model = CharacterSelectModel::new(1920.0, 1080.0);
    model.shared.insert(roster(Some(0)));
    model.sync();
    let registry = &model.registry;
    let root = registry
        .get(registry.get_by_name(CHAR_SELECT_ROOT.0).unwrap())
        .unwrap();
    let panel = registry
        .get(registry.get_by_name(CHAR_LIST_PANEL.0).unwrap())
        .unwrap();
    assert_eq!(panel.parent_id, Some(root.id));
    assert_eq!(panel.position_type, PositionType::Absolute);
    assert_eq!(
        (panel.width, panel.height),
        (Dimension::Fixed(386.0), Dimension::Fixed(520.0))
    );
    assert_eq!(panel.position.left, Val::Percent(100.0));
    assert_eq!(panel.margin.top, Val::Px(164.0));
    let cards = registry
        .get(registry.get_by_name("CharacterListCards").unwrap())
        .unwrap();
    for (i, name, level, status) in [
        (0, "Alyra", "Level 70 Paladin", "Stormwind"),
        (1, "Borin", "Level 42 Shaman", "Orgrimmar"),
    ] {
        let card_name = format!("CharCard_{i}");
        let card = registry
            .get(registry.get_by_name(&card_name).unwrap())
            .unwrap();
        assert_eq!(card.parent_id, Some(cards.id));
        assert_eq!(
            (card.width, card.height),
            (Dimension::Fixed(347.0), Dimension::Fixed(95.0))
        );
        assert_eq!(
            action(&model, &card_name),
            Some(CharSelectAction::SelectChar(i))
        );
        assert_eq!(text(&model, &format!("CharCard_{i}Name")), name);
        assert_eq!(text(&model, &format!("CharCard_{i}Info")), level);
        assert_eq!(text(&model, &format!("CharCard_{i}Status")), status);
        let selected = registry
            .get(
                registry
                    .get_by_name(&format!("CharCard_{i}Selected"))
                    .unwrap(),
            )
            .unwrap();
        assert_eq!(selected.hidden, i != 0);
    }
    let backdrop = registry
        .get(registry.get_by_name("CharCard_0Backdrop").unwrap())
        .unwrap();
    assert!(
        matches!(&backdrop.widget_data, Some(WidgetData::Texture(data)) if data.source == TextureSource::Atlas("glues-characterselect-card-singles".into()))
    );
    assert_eq!(text(&model, SELECTED_NAME_TEXT.0), "Alyra");
    assert_eq!(text(&model, STATUS_TEXT.0), "Choose a hero");
    for (name, expected) in [
        (ENTER_WORLD_BUTTON.0, CharSelectAction::EnterWorld),
        (CREATE_CHAR_BUTTON.0, CharSelectAction::CreateToggle),
        (DELETE_CHAR_BUTTON.0, CharSelectAction::DeleteChar),
        (BACK_BUTTON.0, CharSelectAction::Back),
    ] {
        assert_eq!(action(&model, name), Some(expected));
    }
}

#[test]
fn roster_selection_and_delete_confirmation_follow_shared_state() {
    let mut model = CharacterSelectModel::new(1920.0, 1080.0);
    model.shared.insert(roster(Some(0)));
    model.sync();
    model.shared.insert(roster(Some(1)));
    model.shared.insert(DeleteConfirmUiState {
        visible: true,
        character_name: "Borin".into(),
        typed_text: String::new(),
        countdown_text: "Wait 5 seconds".into(),
        confirm_enabled: false,
    });
    model.sync();
    assert_eq!(text(&model, SELECTED_NAME_TEXT.0), "Borin");
    assert!(
        model
            .registry
            .get(model.registry.get_by_name("CharCard_0Selected").unwrap())
            .unwrap()
            .hidden
    );
    assert!(
        !model
            .registry
            .get(model.registry.get_by_name("CharCard_1Selected").unwrap())
            .unwrap()
            .hidden
    );
    assert!(
        model
            .registry
            .get_by_name(DELETE_CONFIRM_DIALOG.0)
            .is_some()
    );
    assert_eq!(
        text(&model, "DeleteCharacterDialogWarning"),
        "This will permanently delete Borin."
    );
    assert_eq!(
        text(&model, "DeleteCharacterDialogCountdown"),
        "Wait 5 seconds"
    );
    assert_eq!(
        action(&model, DELETE_CANCEL_BUTTON.0),
        Some(CharSelectAction::CancelDeleteChar)
    );
    let disabled = model
        .registry
        .get(model.registry.get_by_name(DELETE_CONFIRM_BUTTON.0).unwrap())
        .unwrap();
    assert_eq!(disabled.onclick.as_deref(), Some(""));
    assert!(
        matches!(&disabled.widget_data, Some(WidgetData::Button(data)) if data.state == ButtonState::Disabled)
    );
    model.shared.insert(DeleteConfirmUiState {
        visible: true,
        character_name: "Borin".into(),
        typed_text: "DELETE".into(),
        countdown_text: String::new(),
        confirm_enabled: true,
    });
    model.sync();
    assert_eq!(
        action(&model, DELETE_CONFIRM_BUTTON.0),
        Some(CharSelectAction::ConfirmDeleteChar)
    );
    let input = model
        .registry
        .get(model.registry.get_by_name(DELETE_CONFIRM_INPUT.0).unwrap())
        .unwrap();
    assert!(matches!(&input.widget_data, Some(WidgetData::EditBox(data)) if data.text == "DELETE"));
    model.shared.insert(DeleteConfirmUiState::default());
    model.shared.insert(roster(None));
    model.sync();
    assert!(
        model
            .registry
            .get_by_name(DELETE_CONFIRM_DIALOG.0)
            .is_none()
    );
    assert!(model.registry.get_by_name(DELETE_CHAR_BUTTON.0).is_none());
    assert_eq!(text(&model, SELECTED_NAME_TEXT.0), "Character Selection");
    assert_eq!(
        action(&model, ENTER_WORLD_BUTTON.0),
        Some(CharSelectAction::EnterWorld)
    );
}
