use game_engine_ui_model::char_select_component::{SELECTED_NAME_TEXT, STATUS_TEXT};
use game_engine_ui_model::char_select_data::CharSelectNames;
use game_engine_ui_model::{CharacterSelectModel, char_select_state_from_roster};

fn names() -> CharSelectNames {
    CharSelectNames::load(&std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../data"))
        .unwrap()
}
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
    let state = char_select_state_from_roster(&roster, Some(1), &names()).unwrap();
    assert_eq!(state.selected_index, Some(1));
    let mut model = CharacterSelectModel::new(1920.0, 1080.0);
    model.shared.insert(state);
    model.sync();

    assert_eq!(text(&model, "CharCard_0Name"), "Alyra");
    assert_eq!(text(&model, "CharCard_0Info"), "Level 70 Human Paladin");
    assert_eq!(text(&model, "CharCard_0Status"), "Ready to enter world");
    assert_eq!(text(&model, "CharCard_1Name"), "Borin");
    assert_eq!(text(&model, "CharCard_1Info"), "Level 42 Troll Shaman");
    assert_eq!(text(&model, "CharCard_1Status"), "Ready to enter world");
    assert_eq!(text(&model, SELECTED_NAME_TEXT.0), "Borin");
    assert_eq!(
        text(&model, STATUS_TEXT.0),
        "Realm: World of Osso    Level 42 Troll Shaman"
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
    let unselected = char_select_state_from_roster(&roster, None, &names()).unwrap();
    assert_eq!(unselected.selected_index, None);
    assert_eq!(unselected.selected_name, "Character Selection");
    assert_eq!(
        unselected.status_text,
        "Select a character to enter the world"
    );
    assert_eq!(unselected.characters[0].name, "Alyra");

    let empty = char_select_state_from_roster(&[], None, &names()).unwrap();
    assert!(empty.characters.is_empty());
    assert_eq!(empty.selected_index, None);
    assert_eq!(empty.selected_name, "Character Selection");
    assert_eq!(empty.status_text, "No characters available on this realm");
}

#[test]
fn stale_selection_keeps_original_index_and_unselected_prompt() {
    let roster = [character(101, "Alyra", 70, 1, 2)];
    let state = char_select_state_from_roster(&roster, Some(4), &names()).unwrap();
    assert_eq!(state.selected_index, Some(4));
    assert_eq!(state.selected_name, "Character Selection");
    assert_eq!(state.status_text, "Select a character to enter the world");
}

#[test]
fn skyborne_names_are_loaded_from_active_data_in_both_skins() {
    let names = names();
    for skin in [
        ui_toolkit::atlas::ActiveSkin::Modern,
        ui_toolkit::atlas::ActiveSkin::Forever,
    ] {
        ui_toolkit::atlas::set_thread_skin(skin);
        let roster = [
            character(30, "Skyhighwar", 1, 95, 1),
            character(40, "Skywinddru", 1, 96, 11),
        ];
        let state = char_select_state_from_roster(&roster, Some(1), &names).unwrap();
        let mut model = CharacterSelectModel::new(1920.0, 1080.0);
        model.shared.insert(state);
        model.sync();
        assert_eq!(
            text(&model, "CharCard_0Info"),
            "Level 1 High Order Skyborne Warrior"
        );
        assert_eq!(
            text(&model, "CharCard_1Info"),
            "Level 1 Windshaper Skyborne Druid"
        );
    }
}

#[test]
fn catalog_names_are_not_hardcoded_and_missing_ids_are_explicit_errors() {
    use std::fs::{create_dir_all, remove_dir_all, write};
    let root = std::env::temp_dir().join(format!("rosterfix-names-{}", std::process::id()));
    let retail = root.join("db2/12.1.0.69933");
    let forever = root.join("db2/1.60.1.70205");
    create_dir_all(&retail).unwrap();
    create_dir_all(&forever).unwrap();
    write(
        retail.join("ChrRaces.csv"),
        "ID,Name_lang\n1,Human\n95,Placeholder\n96,Placeholder\n",
    )
    .unwrap();
    write(
        retail.join("ChrClasses.csv"),
        "ID,Name_lang\n11,Localized Druid\n",
    )
    .unwrap();
    write(
        forever.join("ChrRaces.csv"),
        "ID,Name_lang\n95,Localized High Order\n96,Localized Windshaper\n",
    )
    .unwrap();
    let names = CharSelectNames::load(&root).unwrap();
    assert_eq!(
        names.info(1, 95, 11).unwrap(),
        "Level 1 Localized High Order Localized Druid"
    );
    assert_eq!(
        names.info(1, 96, 11).unwrap(),
        "Level 1 Localized Windshaper Localized Druid"
    );
    assert!(
        names
            .info(1, 96, 99)
            .unwrap_err()
            .contains("class name for ID 99")
    );
    remove_dir_all(root).unwrap();
}
