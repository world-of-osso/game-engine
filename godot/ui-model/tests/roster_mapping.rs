use game_engine_ui_model::char_select_component::{SELECTED_NAME_TEXT, STATUS_TEXT};
use game_engine_ui_model::{CharacterSelectModel, char_select_state_from_roster};
use shared::components::{CharacterAppearance, EquipmentAppearance};
use shared::protocol::CharacterListEntry;
use ui_toolkit::frame::WidgetData;

fn character(id: u64, name: &str, level: u16, race: u8, class: u8) -> CharacterListEntry {
    CharacterListEntry {
        character_id: id,
        name: name.into(),
        level,
        race,
        class,
        appearance: CharacterAppearance::default(),
        equipment_appearance: EquipmentAppearance::default(),
    }
}

fn text(model: &CharacterSelectModel, name: &str) -> String {
    let id = model.registry.get_by_name(name).expect("named frame");
    let frame = model.registry.get(id).expect("frame");
    let Some(WidgetData::FontString(label)) = &frame.widget_data else {
        panic!("{name} is not a label");
    };
    label.text.clone()
}

#[test]
fn protocol_roster_projects_original_card_text_and_selected_character() {
    let roster = [
        character(101, "Alyra", 70, 1, 2),
        character(202, "Borin", 42, 8, 7),
    ];
    let state = char_select_state_from_roster(&roster, Some(1));
    assert_eq!(state.selected_index, Some(1));
    let mut model = CharacterSelectModel::new(1920.0, 1080.0);
    model.shared.insert(state);
    model.sync();

    assert_eq!(text(&model, "CharCard_0Name"), "Alyra");
    assert_eq!(
        text(&model, "CharCard_0Info"),
        "Level 70   Race 1   Class 2"
    );
    assert_eq!(text(&model, "CharCard_0Status"), "Ready to enter world");
    assert_eq!(text(&model, "CharCard_1Name"), "Borin");
    assert_eq!(
        text(&model, "CharCard_1Info"),
        "Level 42   Race 8   Class 7"
    );
    assert_eq!(text(&model, "CharCard_1Status"), "Ready to enter world");
    assert_eq!(text(&model, SELECTED_NAME_TEXT.0), "Borin");
    assert_eq!(
        text(&model, STATUS_TEXT.0),
        "Realm: World of Osso    Level 42    Race 8    Class 7"
    );
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
}

#[test]
fn unselected_and_empty_rosters_keep_original_prompts() {
    let roster = [character(101, "Alyra", 70, 1, 2)];
    let unselected = char_select_state_from_roster(&roster, None);
    assert_eq!(unselected.selected_index, None);
    assert_eq!(unselected.selected_name, "Character Selection");
    assert_eq!(
        unselected.status_text,
        "Select a character to enter the world"
    );
    assert_eq!(unselected.characters[0].name, "Alyra");

    let empty = char_select_state_from_roster(&[], None);
    assert!(empty.characters.is_empty());
    assert_eq!(empty.selected_index, None);
    assert_eq!(empty.selected_name, "Character Selection");
    assert_eq!(empty.status_text, "No characters available on this realm");
}

#[test]
fn stale_selection_keeps_original_index_and_unselected_prompt() {
    let roster = [character(101, "Alyra", 70, 1, 2)];
    let state = char_select_state_from_roster(&roster, Some(4));
    assert_eq!(state.selected_index, Some(4));
    assert_eq!(state.selected_name, "Character Selection");
    assert_eq!(state.status_text, "Select a character to enter the world");
}
