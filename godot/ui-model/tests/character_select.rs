use game_engine_ui_model::CharacterSelectModel;
use game_engine_ui_model::char_select_component::{
    BACK_BUTTON, CHAR_LIST_PANEL, CHAR_SELECT_ROOT, CREATE_CHAR_BUTTON, CampsiteEntry,
    CampsiteState, CharDisplayEntry, CharSelectAction, CharSelectState, DELETE_CANCEL_BUTTON,
    DELETE_CHAR_BUTTON, DELETE_CONFIRM_BUTTON, DELETE_CONFIRM_DIALOG, DELETE_CONFIRM_INPUT,
    DeleteConfirmUiState, ENTER_WORLD_BUTTON, SELECTED_NAME_TEXT, STATUS_TEXT,
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
fn character_select_root_tracks_viewport_after_sync() {
    let mut model = CharacterSelectModel::new(1280.0, 720.0);
    model.sync();
    let id = model.registry.get_by_name(CHAR_SELECT_ROOT.0).unwrap();
    let root = model.registry.get(id).unwrap();
    assert_eq!(
        (root.width, root.height),
        (Dimension::Fixed(1280.0), Dimension::Fixed(720.0))
    );

    model.registry.screen_width = 1600.0;
    model.registry.screen_height = 900.0;
    model.sync();
    let root = model.registry.get(id).unwrap();
    assert_eq!(
        (root.width, root.height),
        (Dimension::Fixed(1600.0), Dimension::Fixed(900.0))
    );
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
        matches!(&backdrop.widget_data, Some(WidgetData::Texture(data)) if matches!(&data.source, TextureSource::Atlas(n) if !n.is_empty()))
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

fn frame<'a>(model: &'a CharacterSelectModel, name: &str) -> &'a ui_toolkit::frame::Frame {
    model
        .registry
        .get(
            model
                .registry
                .get_by_name(name)
                .unwrap_or_else(|| panic!("missing {name}")),
        )
        .unwrap()
}

fn label_color(model: &CharacterSelectModel, name: &str) -> [f32; 4] {
    let Some(WidgetData::FontString(label)) = &frame(model, name).widget_data else {
        panic!("{name} is not a label")
    };
    label.color
}

/// Box visibility and label colour of each tab, left to right.
fn tab_looks(model: &CharacterSelectModel) -> Vec<(bool, [f32; 4])> {
    TOP_NAV_TABS
        .iter()
        .map(|tab| {
            (
                !frame(model, &format!("{tab}Box")).hidden,
                label_color(model, &format!("{tab}Label")),
            )
        })
        .collect()
}

const TOP_NAV_TABS: [&str; 5] = [
    "CharSelectModeTab",
    "CharSelectShopTab",
    "CharSelectMenuTab",
    "CharSelectRealmsTab",
    "CharSelectCampsitesTab",
];
const GOLD: [f32; 4] = [1.0, 0.82, 0.0, 1.0];
const WHITE: [f32; 4] = [1.0, 1.0, 1.0, 1.0];

#[test]
fn top_navigation_tabs_replace_tophud_art_with_retail_labels_and_actions() {
    let mut model = CharacterSelectModel::new(1280.0, 720.0);
    model.sync();
    for art in [
        "CharSelectTopHudLeft",
        "CharSelectTopHudMiddle",
        "CharSelectTopHudRight",
    ] {
        assert!(model.registry.get_by_name(art).is_none(), "{art} remains");
    }
    let row = model.registry.get_by_name("CharSelectTopNavTabs").unwrap();
    let children: Vec<String> = model
        .registry
        .get(row)
        .unwrap()
        .children
        .iter()
        .map(|id| model.registry.get(*id).unwrap().name.clone().unwrap())
        .collect();
    assert_eq!(children, TOP_NAV_TABS);
    let labels: Vec<String> = TOP_NAV_TABS
        .iter()
        .map(|tab| text(&model, &format!("{tab}Label")))
        .collect();
    assert_eq!(labels, ["MODE", "SHOP", "MENU", "REALMS", "CAMPSITES"]);
    let actions: Vec<Option<CharSelectAction>> =
        TOP_NAV_TABS.iter().map(|tab| action(&model, tab)).collect();
    assert_eq!(
        actions,
        [
            None,
            None,
            Some(CharSelectAction::Menu),
            Some(CharSelectAction::Back),
            Some(CharSelectAction::CampsiteToggle),
        ]
    );
    for rule in ["CharSelectTopNavRuleTop", "CharSelectTopNavRuleBottom"] {
        assert_eq!(
            frame(&model, rule)
                .background_color
                .map(|c| c[..3].to_vec()),
            Some(GOLD[..3].to_vec())
        );
    }
    assert_eq!(tab_looks(&model), vec![(false, GOLD); 5]);
}

#[test]
fn hovered_pressed_and_open_campsite_tabs_are_boxed_with_white_labels() {
    let mut model = CharacterSelectModel::new(1280.0, 720.0);
    model.sync();
    let set_button = |model: &mut CharacterSelectModel, name: &str, hovered: bool, state| {
        let id = model.registry.get_by_name(name).unwrap();
        let Some(WidgetData::Button(button)) = &mut model.registry.get_mut(id).unwrap().widget_data
        else {
            panic!("{name} is not a button")
        };
        button.hovered = hovered;
        button.state = state;
    };
    set_button(&mut model, "CharSelectModeTab", true, ButtonState::Normal);
    set_button(
        &mut model,
        "CharSelectRealmsTab",
        false,
        ButtonState::Pushed,
    );
    model.sync();
    let boxed = frame(&model, "CharSelectModeTabBox");
    assert!(matches!(boxed.border, Some(ref b) if b.width == 1.0 && b.color[..3] == GOLD[..3]));
    assert!(
        boxed
            .background_color
            .is_some_and(|c| c[0] < 0.1 && c[3] < 1.0)
    );
    assert_eq!(
        tab_looks(&model),
        vec![
            (true, WHITE),
            (false, GOLD),
            (false, GOLD),
            (true, WHITE),
            (false, GOLD)
        ]
    );

    set_button(&mut model, "CharSelectModeTab", false, ButtonState::Normal);
    set_button(
        &mut model,
        "CharSelectRealmsTab",
        false,
        ButtonState::Normal,
    );
    model.shared.insert(CampsiteState {
        scenes: vec![CampsiteEntry {
            id: 1,
            name: "Adventurer's Rest".into(),
            preview_image: None,
        }],
        panel_visible: true,
        selected_id: Some(1),
        ..Default::default()
    });
    model.sync();
    assert!(!frame(&model, "CampsitePanel").hidden);
    assert_eq!(
        tab_looks(&model),
        vec![
            (false, GOLD),
            (false, GOLD),
            (false, GOLD),
            (false, GOLD),
            (true, WHITE)
        ]
    );
}

fn campsites(count: u32, page: usize) -> CampsiteState {
    CampsiteState {
        scenes: (1..=count)
            .map(|id| CampsiteEntry {
                id,
                name: format!("Camp {id}"),
                preview_image: None,
            })
            .collect(),
        panel_visible: true,
        selected_id: Some(1),
        page,
    }
}

fn listed_campsites(model: &CharacterSelectModel) -> Vec<u32> {
    (1..=8)
        .filter(|id| {
            model
                .registry
                .get_by_name(&format!("CampsiteScene_{id}"))
                .is_some()
        })
        .collect()
}

#[test]
fn eight_campsites_page_four_at_a_time_like_retail_paging_controls() {
    let mut model = CharacterSelectModel::new(1280.0, 720.0);
    model.shared.insert(campsites(8, 0));
    model.sync();
    assert_eq!(listed_campsites(&model), [1, 2, 3, 4]);
    assert_eq!(text(&model, "CampsitePageText"), "Page 1/2");
    assert_eq!(
        action(&model, "CampsitePrevPage"),
        Some(CharSelectAction::CampsitePage(0)),
        "disabled prev stays on the first page"
    );
    assert_eq!(
        action(&model, "CampsiteNextPage"),
        Some(CharSelectAction::CampsitePage(1))
    );

    model.shared.insert(campsites(8, 1));
    model.sync();
    assert_eq!(listed_campsites(&model), [5, 6, 7, 8]);
    assert_eq!(text(&model, "CampsitePageText"), "Page 2/2");
    assert_eq!(
        action(&model, "CampsitePrevPage"),
        Some(CharSelectAction::CampsitePage(0))
    );
    assert_eq!(
        action(&model, "CampsiteNextPage"),
        Some(CharSelectAction::CampsitePage(1)),
        "disabled next stays on the last page"
    );

    model.shared.insert(campsites(5, 7));
    model.sync();
    assert_eq!(
        listed_campsites(&model),
        [5],
        "stale page clamps to the last"
    );
    assert_eq!(text(&model, "CampsitePageText"), "Page 2/2");
    assert_eq!(
        CharSelectAction::parse(&CharSelectAction::CampsitePage(1).to_string()),
        Some(CharSelectAction::CampsitePage(1))
    );
}
