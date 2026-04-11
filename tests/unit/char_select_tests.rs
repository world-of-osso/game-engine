use super::*;
use std::collections::VecDeque;

use bevy::input::ButtonInput;
use bevy::input::keyboard::KeyboardInput;
use bevy::window::PrimaryWindow;
use shared::protocol::CharacterListEntry;

use game_engine::ui::automation::{UiAutomationAction, UiAutomationPlugin, UiAutomationQueue};
use game_engine::ui::event::EventBus;
use game_engine::ui::frame::WidgetData;
use game_engine::ui::registry::FrameRegistry;
use game_engine::ui::screens::char_select_component::{
    BACK_BUTTON, CharSelectAction, DELETE_CHAR_BUTTON, ENTER_WORLD_BUTTON, MENU_BUTTON,
};
use game_engine::ui::strata::FrameStrata;
use game_engine::ui::widgets::button::ButtonState;
use ui_toolkit::layout::recompute_layouts;

fn test_registry() -> FrameRegistry {
    FrameRegistry::new(1920.0, 1080.0)
}

fn build_screen(state: CharSelectState) -> FrameRegistry {
    let mut reg = test_registry();
    let mut shared = ui_toolkit::screen::SharedContext::new();
    shared.insert(state);
    shared.insert(DeleteConfirmUiState::default());
    Screen::new(char_select_screen).sync(&shared, &mut reg);
    recompute_layouts(&mut reg);
    reg
}

fn build_screen_with_real_layout(state: CharSelectState) -> FrameRegistry {
    let mut reg = test_registry();
    let mut shared = ui_toolkit::screen::SharedContext::new();
    shared.insert(state);
    shared.insert(DeleteConfirmUiState::default());
    Screen::new(char_select_screen).sync(&shared, &mut reg);
    let cs = CharSelectUi::resolve(&reg);
    super::apply_post_setup(&mut reg, &cs);
    recompute_layouts(&mut reg);
    reg
}

fn build_screen_with_campsites(state: CharSelectState, campsite: CampsiteState) -> FrameRegistry {
    let mut reg = test_registry();
    let mut shared = ui_toolkit::screen::SharedContext::new();
    shared.insert(state);
    shared.insert(campsite);
    shared.insert(DeleteConfirmUiState::default());
    Screen::new(char_select_screen).sync(&shared, &mut reg);
    recompute_layouts(&mut reg);
    reg
}

fn build_screen_with_delete_confirm(
    state: CharSelectState,
    delete_confirm: DeleteConfirmUiState,
) -> FrameRegistry {
    let mut reg = test_registry();
    let mut shared = ui_toolkit::screen::SharedContext::new();
    shared.insert(state);
    shared.insert(delete_confirm);
    Screen::new(char_select_screen).sync(&shared, &mut reg);
    recompute_layouts(&mut reg);
    reg
}

fn frame_center(reg: &FrameRegistry, name: &str) -> Vec2 {
    let rect = reg
        .get_by_name(name)
        .and_then(|id| reg.get(id))
        .and_then(|frame| frame.layout_rect.clone())
        .unwrap_or_else(|| panic!("{name} has no layout_rect"));
    Vec2::new(rect.x + rect.width * 0.5, rect.y + rect.height * 0.5)
}

fn one_scene_campsite_state() -> CampsiteState {
    CampsiteState {
        scenes: vec![CampsiteEntry {
            id: 1,
            name: "Forest".to_string(),
            preview_image: None,
        }],
        panel_visible: true,
        selected_id: Some(1),
    }
}

fn build_test_app() -> App {
    let mut app = App::new();
    app.add_plugins(MinimalPlugins);
    app.add_plugins(bevy::state::app::StatesPlugin);
    app.add_plugins(UiAutomationPlugin);
    app.add_plugins(CharSelectPlugin);
    app.add_message::<KeyboardInput>();
    app.insert_resource(UiState {
        registry: FrameRegistry::new(0.0, 0.0),
        event_bus: EventBus::new(),
        focused_frame: None,
    });
    app.insert_resource(ButtonInput::<MouseButton>::default());
    app.insert_resource(CharacterList(vec![CharacterListEntry {
        character_id: 1,
        name: "Elara".to_string(),
        level: 1,
        race: 1,
        class: 1,
        appearance: shared::components::CharacterAppearance::default(),
        equipment_appearance: shared::components::EquipmentAppearance::default(),
    }]));
    app.insert_state(GameState::CharSelect);
    let mut window = Window::default();
    window.resolution.set(1280.0, 720.0);
    app.world_mut().spawn((window, PrimaryWindow));
    app
}

fn assert_single_anchor(
    reg: &FrameRegistry,
    frame_id: u64,
    point: game_engine::ui::anchor::AnchorPoint,
    relative_point: game_engine::ui::anchor::AnchorPoint,
    relative_to: Option<u64>,
    x_offset: f32,
    y_offset: f32,
) {
    let frame = reg.get(frame_id).expect("frame");
    assert_eq!(frame.anchors.len(), 1);
    assert_eq!(frame.anchors[0].point, point);
    assert_eq!(frame.anchors[0].relative_point, relative_point);
    assert_eq!(frame.anchors[0].relative_to, relative_to);
    assert_eq!(frame.anchors[0].x_offset, x_offset);
    assert_eq!(frame.anchors[0].y_offset, y_offset);
}

#[test]
fn screen_builds_with_empty_char_list() {
    let reg = build_screen(CharSelectState::default());
    assert!(reg.get_by_name("CharSelectRoot").is_some());
    assert!(reg.get_by_name("EnterWorld").is_some());
    assert!(reg.get_by_name("BackToLogin").is_some());
    let ui = CharSelectUi::resolve(&reg);
    assert_eq!(ui.delete_button, None);
}

#[test]
fn screen_builds_with_characters() {
    let reg = build_screen(CharSelectState {
        characters: vec![CharDisplayEntry {
            name: "TestChar".to_string(),
            info: "Level 60   Race 1   Class 1".to_string(),
            status: "Ready".to_string(),
        }],
        selected_index: Some(0),
        ..Default::default()
    });
    assert!(reg.get_by_name("CharCard_0").is_some());
    assert!(reg.get_by_name("CharCard_0Name").is_some());
}

#[test]
fn char_select_screen_builds_all_critical_frames() {
    let reg = build_screen(CharSelectState {
        characters: vec![CharDisplayEntry {
            name: "TestChar".to_string(),
            info: "Level 60   Race 1   Class 1".to_string(),
            status: "Ready".to_string(),
        }],
        selected_index: Some(0),
        ..Default::default()
    });

    for frame_name in [
        "CharacterListCards",
        "CharCard_0",
        ENTER_WORLD_BUTTON.0,
        DELETE_CHAR_BUTTON.0,
        BACK_BUTTON.0,
        MENU_BUTTON.0,
    ] {
        assert!(
            reg.get_by_name(frame_name).is_some(),
            "expected {frame_name} to exist in char select screen"
        );
    }
}

#[test]
fn character_card_list_is_anchored_to_top_left() {
    use game_engine::ui::anchor::AnchorPoint;

    let reg = build_screen(CharSelectState {
        characters: vec![CharDisplayEntry {
            name: "TestChar".to_string(),
            info: "Level 60   Race 1   Class 1".to_string(),
            status: "Ready".to_string(),
        }],
        selected_index: Some(0),
        ..Default::default()
    });

    let cards_id = reg
        .get_by_name("CharacterListCards")
        .expect("CharacterListCards");
    let list_panel_id = reg
        .get_by_name("CharacterListPanel")
        .expect("CharacterListPanel");
    assert_single_anchor(
        &reg,
        cards_id,
        AnchorPoint::TopLeft,
        AnchorPoint::TopLeft,
        Some(list_panel_id),
        19.0,
        -94.0,
    );
}

#[test]
fn character_cards_are_vertically_stacked_with_consistent_gap() {
    let reg = build_screen(CharSelectState {
        characters: vec![
            CharDisplayEntry {
                name: "Theron".to_string(),
                info: "Level 60   Race 1   Class 1".to_string(),
                status: "Ready".to_string(),
            },
            CharDisplayEntry {
                name: "Elara".to_string(),
                info: "Level 60   Race 1   Class 1".to_string(),
                status: "Ready".to_string(),
            },
        ],
        selected_index: Some(0),
        ..Default::default()
    });

    let card0 = reg
        .get_by_name("CharCard_0")
        .and_then(|id| reg.get(id))
        .and_then(|frame| frame.layout_rect.clone())
        .expect("CharCard_0 layout_rect");
    let card1 = reg
        .get_by_name("CharCard_1")
        .and_then(|id| reg.get(id))
        .and_then(|frame| frame.layout_rect.clone())
        .expect("CharCard_1 layout_rect");

    assert!(
        card0.y < card1.y,
        "expected CharCard_1 below CharCard_0, got {} <= {}",
        card1.y,
        card0.y
    );
    assert_eq!(
        card1.y - card0.y,
        card0.height + 10.0,
        "expected char card vertical spacing to match card height plus 10px gap"
    );
}

#[test]
fn char_select_action_buttons_occupy_expected_screen_regions() {
    let reg = build_screen_with_real_layout(CharSelectState {
        characters: vec![CharDisplayEntry {
            name: "Theron".to_string(),
            info: "Level 60   Race 1   Class 1".to_string(),
            status: "Ready".to_string(),
        }],
        selected_index: Some(0),
        ..Default::default()
    });

    let enter_world = frame_center(&reg, ENTER_WORLD_BUTTON.0);
    let delete_char = frame_center(&reg, DELETE_CHAR_BUTTON.0);
    let back = frame_center(&reg, BACK_BUTTON.0);

    assert!(
        (enter_world.x - 960.0).abs() <= 20.0,
        "expected EnterWorld centered near screen midpoint, got x={}",
        enter_world.x
    );
    assert!(
        enter_world.y > 540.0,
        "expected EnterWorld in lower half, got y={}",
        enter_world.y
    );
    assert!(
        delete_char.x > 960.0 && delete_char.y > 540.0,
        "expected DeleteChar in lower-right region, got ({}, {})",
        delete_char.x,
        delete_char.y
    );
    assert!(
        back.x < 960.0 && back.y > 540.0,
        "expected BackToLogin in lower-left region, got ({}, {})",
        back.x,
        back.y
    );
}

#[test]
fn selected_character_shows_delete_button_with_empty_text_and_icon_child() {
    let reg = build_screen(CharSelectState {
        characters: vec![CharDisplayEntry {
            name: "TestChar".to_string(),
            info: "Level 60   Race 1   Class 1".to_string(),
            status: "Ready".to_string(),
        }],
        selected_index: Some(0),
        ..Default::default()
    });

    let delete_id = reg.get_by_name("DeleteChar").expect("DeleteChar");
    let delete_frame = reg.get(delete_id).expect("delete frame");
    let Some(WidgetData::Button(button)) = delete_frame.widget_data.as_ref() else {
        panic!("DeleteChar should be a button");
    };
    assert_eq!(button.text, "");

    let icon_id = reg.get_by_name("DeleteCharIcon").expect("DeleteCharIcon");
    let icon_frame = reg.get(icon_id).expect("icon frame");
    assert_eq!(icon_frame.parent_id, Some(delete_id));

    let Some(WidgetData::Texture(icon_texture)) = icon_frame.widget_data.as_ref() else {
        panic!("DeleteCharIcon should be a texture");
    };
    assert!(
        matches!(
            &icon_texture.source,
            game_engine::ui::widgets::texture::TextureSource::File(path)
                if path == "data/ui/delete-trash-icon-gold.ktx2"
        ),
        "DeleteCharIcon should point at the generated trash icon texture"
    );
}

#[test]
fn no_selection_hides_delete_button_and_icon() {
    let reg = build_screen(CharSelectState::default());
    assert!(reg.get_by_name("DeleteChar").is_none());
    assert!(reg.get_by_name("DeleteCharIcon").is_none());
}

#[test]
fn delete_confirmation_modal_requires_timer_and_phrase() {
    let reg = build_screen_with_delete_confirm(
        CharSelectState {
            characters: vec![CharDisplayEntry {
                name: "Elara".to_string(),
                info: "Level 1   Race 1   Class 1".to_string(),
                status: "Ready".to_string(),
            }],
            selected_index: Some(0),
            ..Default::default()
        },
        DeleteConfirmUiState {
            visible: true,
            character_name: "Elara".to_string(),
            typed_text: "DEL".to_string(),
            countdown_text: "Delete unlocks in 2s".to_string(),
            confirm_enabled: false,
        },
    );

    assert!(reg.get_by_name("DeleteCharacterDialog").is_some());
    assert!(reg.get_by_name("DeleteCharacterConfirmInput").is_some());

    let confirm = reg
        .get(
            reg.get_by_name("DeleteCharacterConfirmButton")
                .expect("confirm button"),
        )
        .expect("confirm frame");
    let Some(WidgetData::Button(button)) = confirm.widget_data.as_ref() else {
        panic!("DeleteCharacterConfirmButton should be a button");
    };
    assert_eq!(button.state, ButtonState::Disabled);

    let countdown = reg
        .get(
            reg.get_by_name("DeleteCharacterDialogCountdown")
                .expect("countdown"),
        )
        .expect("countdown frame");
    let Some(WidgetData::FontString(text)) = countdown.widget_data.as_ref() else {
        panic!("DeleteCharacterDialogCountdown should be a fontstring");
    };
    assert_eq!(text.text, "Delete unlocks in 2s");
}

#[test]
fn delete_confirmation_modal_enables_confirm_after_phrase_and_timer() {
    let reg = build_screen_with_delete_confirm(
        CharSelectState {
            characters: vec![CharDisplayEntry {
                name: "Elara".to_string(),
                info: "Level 1   Race 1   Class 1".to_string(),
                status: "Ready".to_string(),
            }],
            selected_index: Some(0),
            ..Default::default()
        },
        DeleteConfirmUiState {
            visible: true,
            character_name: "Elara".to_string(),
            typed_text: "DELETE".to_string(),
            countdown_text: "Ready. Press Delete Forever to remove this character.".to_string(),
            confirm_enabled: true,
        },
    );

    let confirm = reg
        .get(
            reg.get_by_name("DeleteCharacterConfirmButton")
                .expect("confirm button"),
        )
        .expect("confirm frame");
    let Some(WidgetData::Button(button)) = confirm.widget_data.as_ref() else {
        panic!("DeleteCharacterConfirmButton should be a button");
    };
    assert!(button.enabled);

    let input = reg
        .get(
            reg.get_by_name("DeleteCharacterConfirmInput")
                .expect("confirm input"),
        )
        .expect("input frame");
    let Some(WidgetData::EditBox(editbox)) = input.widget_data.as_ref() else {
        panic!("DeleteCharacterConfirmInput should be an editbox");
    };
    assert_eq!(editbox.text, "DELETE");
}

#[test]
fn character_cards_use_tinted_atlas_textures_without_css_border() {
    let reg = build_screen(CharSelectState {
        characters: vec![CharDisplayEntry {
            name: "TestChar".to_string(),
            info: "Level 60   Race 1   Class 1".to_string(),
            status: "Ready".to_string(),
        }],
        selected_index: Some(0),
        ..Default::default()
    });

    let card_id = reg.get_by_name("CharCard_0").expect("CharCard_0");
    let card = reg.get(card_id).expect("card frame");
    assert!(
        card.border.is_none(),
        "card should rely on atlas art, not CSS border"
    );

    let backdrop_id = reg
        .get_by_name("CharCard_0Backdrop")
        .expect("CharCard_0Backdrop");
    let backdrop = reg.get(backdrop_id).expect("backdrop frame");
    let Some(WidgetData::Texture(backdrop_tex)) = backdrop.widget_data.as_ref() else {
        panic!("backdrop should be a texture");
    };
    assert_eq!(backdrop_tex.vertex_color, [0.76, 0.70, 0.57, 0.96]);

    let selected_id = reg
        .get_by_name("CharCard_0Selected")
        .expect("CharCard_0Selected");
    let selected_frame = reg.get(selected_id).expect("selected frame");
    let Some(WidgetData::Texture(selected_tex)) = selected_frame.widget_data.as_ref() else {
        panic!("selected highlight should be a texture");
    };
    assert_eq!(selected_tex.vertex_color, [0.82, 0.74, 0.46, 0.9]);
}

#[test]
fn character_card_frames_set_expected_onclick_actions() {
    let reg = build_screen(CharSelectState {
        characters: vec![
            CharDisplayEntry {
                name: "Theron".to_string(),
                info: "Level 1   Race 1   Class 1".to_string(),
                status: "Ready".to_string(),
            },
            CharDisplayEntry {
                name: "Elara".to_string(),
                info: "Level 1   Race 1   Class 1".to_string(),
                status: "Ready".to_string(),
            },
        ],
        selected_index: Some(0),
        selected_name: "Theron".to_string(),
        ..Default::default()
    });

    let card0 = reg
        .get(reg.get_by_name("CharCard_0").expect("CharCard_0"))
        .expect("CharCard_0 frame");
    let card1 = reg
        .get(reg.get_by_name("CharCard_1").expect("CharCard_1"))
        .expect("CharCard_1 frame");

    assert_eq!(card0.onclick.as_deref(), Some("select_char:0"));
    assert_eq!(card1.onclick.as_deref(), Some("select_char:1"));
}

#[test]
fn find_clicked_action_returns_character_card_action_from_card_center() {
    let reg = build_screen(CharSelectState {
        characters: vec![
            CharDisplayEntry {
                name: "Theron".to_string(),
                info: "Level 1   Race 1   Class 1".to_string(),
                status: "Ready".to_string(),
            },
            CharDisplayEntry {
                name: "Elara".to_string(),
                info: "Level 1   Race 1   Class 1".to_string(),
                status: "Ready".to_string(),
            },
        ],
        selected_index: Some(0),
        selected_name: "Theron".to_string(),
        ..Default::default()
    });

    let card0_center = frame_center(&reg, "CharCard_0");
    let card1_center = frame_center(&reg, "CharCard_1");
    let ui = UiState {
        registry: reg,
        event_bus: EventBus::new(),
        focused_frame: None,
    };

    assert_eq!(
        crate::scenes::char_select::input::find_clicked_action(&ui, card0_center.x, card0_center.y)
            .as_deref(),
        Some("select_char:0")
    );
    assert_eq!(
        crate::scenes::char_select::input::find_clicked_action(&ui, card1_center.x, card1_center.y)
            .as_deref(),
        Some("select_char:1")
    );
}

#[test]
fn find_clicked_action_walks_up_from_character_card_text_to_parent_onclick() {
    let reg = build_screen(CharSelectState {
        characters: vec![CharDisplayEntry {
            name: "Elara".to_string(),
            info: "Level 1   Race 1   Class 1".to_string(),
            status: "Ready".to_string(),
        }],
        selected_index: Some(0),
        selected_name: "Elara".to_string(),
        ..Default::default()
    });
    let label_center = frame_center(&reg, "CharCard_0Name");
    let ui = UiState {
        registry: reg,
        event_bus: EventBus::new(),
        focused_frame: None,
    };

    assert_eq!(
        crate::scenes::char_select::input::find_clicked_action(&ui, label_center.x, label_center.y)
            .as_deref(),
        Some("select_char:0")
    );
}

#[test]
fn walk_up_for_onclick_reaches_character_card_action_from_texture_child() {
    let reg = build_screen(CharSelectState {
        characters: vec![CharDisplayEntry {
            name: "Elara".to_string(),
            info: "Level 1   Race 1   Class 1".to_string(),
            status: "Ready".to_string(),
        }],
        selected_index: Some(0),
        selected_name: "Elara".to_string(),
        ..Default::default()
    });
    let backdrop_id = reg
        .get_by_name("CharCard_0Backdrop")
        .expect("CharCard_0Backdrop");

    assert_eq!(
        crate::ui_input::walk_up_for_onclick(&reg, backdrop_id).as_deref(),
        Some("select_char:0")
    );
}

#[test]
fn screen_does_not_include_inline_create_panel() {
    let reg = build_screen(CharSelectState::default());
    assert!(reg.get_by_name("CreatePanel").is_none());
}

#[test]
fn character_list_backdrop_uses_atlas_slice_metadata() {
    let ns = atlas_nine_slice("glues-characterselect-card-all-bg", 386.0, 520.0)
        .expect("atlas-backed nine-slice");
    assert_eq!(ns.uv_edge_sizes, Some([14.0, 11.0, 14.0, 17.0]));
    let display = ns.edge_sizes.expect("display edge sizes");
    assert_eq!(display, [14.0, 11.0, 14.0, 17.0]);
}

#[test]
fn campsite_tab_is_anchored_to_top_center_without_offsets() {
    use game_engine::ui::anchor::AnchorPoint;
    let reg = build_screen_with_campsites(CharSelectState::default(), one_scene_campsite_state());
    let root_id = reg.get_by_name("CharSelectRoot").expect("CharSelectRoot");
    let bar_id = reg.get_by_name("CampsiteMenuBar").expect("CampsiteMenuBar");
    let tab_id = reg.get_by_name("CampsiteTab").expect("CampsiteTab");
    assert_single_anchor(
        &reg,
        bar_id,
        AnchorPoint::Top,
        AnchorPoint::Top,
        Some(root_id),
        0.0,
        0.0,
    );
    assert_single_anchor(
        &reg,
        tab_id,
        AnchorPoint::TopLeft,
        AnchorPoint::TopLeft,
        Some(bar_id),
        357.0,
        -1.0,
    );
}

#[test]
fn campsite_panel_is_anchored_to_top_center_without_offsets() {
    use game_engine::ui::anchor::AnchorPoint;
    let reg = build_screen_with_campsites(CharSelectState::default(), one_scene_campsite_state());
    let root_id = reg.get_by_name("CharSelectRoot").expect("CharSelectRoot");
    let panel_id = reg.get_by_name("CampsitePanel").expect("CampsitePanel");
    assert_single_anchor(
        &reg,
        panel_id,
        AnchorPoint::Top,
        AnchorPoint::Top,
        Some(root_id),
        0.0,
        -58.0,
    );
}

#[test]
fn campsite_overlay_renders_in_dialog_strata() {
    let reg = build_screen_with_campsites(
        CharSelectState {
            selected_index: Some(0),
            selected_name: "Elara".to_string(),
            ..Default::default()
        },
        one_scene_campsite_state(),
    );

    let menu_bar = reg
        .get(reg.get_by_name("CampsiteMenuBar").expect("CampsiteMenuBar"))
        .expect("menu bar");
    let panel = reg
        .get(reg.get_by_name("CampsitePanel").expect("CampsitePanel"))
        .expect("panel");
    let card = reg
        .get(reg.get_by_name("CampsiteScene_1").expect("CampsiteScene_1"))
        .expect("card");

    assert_eq!(menu_bar.strata, FrameStrata::Dialog);
    assert_eq!(panel.strata, FrameStrata::Dialog);
    assert_eq!(card.strata, FrameStrata::Dialog);
}

#[test]
fn automation_click_create_char_transitions_to_char_create() {
    let mut app = build_test_app();
    app.insert_resource(UiAutomationQueue(VecDeque::from([
        UiAutomationAction::ClickFrame("CreateChar".to_string()),
    ])));

    // Frame 1: builds UI + writes click event
    // Frame 2: dispatches event → sets next state
    // Frame 3: state transition applies
    app.update();
    app.update();
    app.update();

    assert_eq!(
        *app.world().resource::<State<GameState>>().get(),
        GameState::CharCreate
    );
    assert!(
        app.world().resource::<UiAutomationQueue>().is_empty(),
        "expected CreateChar click to be consumed by CharSelect automation"
    );
}

#[test]
fn automation_click_delete_char_opens_confirmation_dialog() {
    let mut app = build_test_app();
    app.insert_resource(UiAutomationQueue(VecDeque::from([
        UiAutomationAction::ClickFrame("DeleteChar".to_string()),
    ])));

    app.update();
    app.update();

    let delete_confirm = app.world().resource::<DeleteCharacterConfirmationState>();
    let target = delete_confirm
        .target
        .as_ref()
        .expect("delete confirm target");
    assert_eq!(target.name, "Elara");
}

#[test]
fn parse_click_action_event_logs_known_select_action() {
    let (parsed_action, parsed_action_label) =
        crate::scenes::char_select::input::parse_click_action_event("select_char:1");

    assert_eq!(parsed_action, Some(CharSelectAction::SelectChar(1)));
    assert_eq!(parsed_action_label, "select_char:1");
}

#[test]
fn parse_click_action_event_marks_unknown_action_unparsed() {
    let (parsed_action, parsed_action_label) =
        crate::scenes::char_select::input::parse_click_action_event("not_an_action");

    assert_eq!(parsed_action, None);
    assert_eq!(parsed_action_label, "unparsed");
}

#[test]
fn automation_type_uppercases_delete_confirmation_input() {
    let mut app = build_test_app();
    app.update();
    app.world_mut()
        .resource_mut::<DeleteCharacterConfirmationState>()
        .target = Some(DeleteCharacterTarget {
        character_id: 1,
        name: "Elara".to_string(),
    });
    app.update();

    app.insert_resource(UiAutomationQueue(VecDeque::from([
        UiAutomationAction::TypeText("delete".to_string()),
    ])));

    app.update();

    let delete_confirm = app.world().resource::<DeleteCharacterConfirmationState>();
    assert_eq!(delete_confirm.typed_text, "DELETE");

    let ui = app.world().resource::<UiState>();
    let input_id = ui
        .registry
        .get_by_name("DeleteCharacterConfirmInput")
        .expect("delete confirmation input");
    let Some(WidgetData::EditBox(editbox)) = ui
        .registry
        .get(input_id)
        .and_then(|frame| frame.widget_data.as_ref())
    else {
        panic!("DeleteCharacterConfirmInput should be an editbox");
    };
    assert_eq!(editbox.text, "DELETE");
}
