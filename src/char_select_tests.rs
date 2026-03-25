use super::char_select_input::*;
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
use game_engine::ui::strata::FrameStrata;

fn test_registry() -> FrameRegistry {
    FrameRegistry::new(1920.0, 1080.0)
}

fn build_screen(state: CharSelectState) -> FrameRegistry {
    let mut reg = test_registry();
    let mut shared = ui_toolkit::screen::SharedContext::new();
    shared.insert(state);
    Screen::new(char_select_screen).sync(&shared, &mut reg);
    reg
}

fn build_screen_with_campsites(state: CharSelectState, campsite: CampsiteState) -> FrameRegistry {
    let mut reg = test_registry();
    let mut shared = ui_toolkit::screen::SharedContext::new();
    shared.insert(state);
    shared.insert(campsite);
    Screen::new(char_select_screen).sync(&shared, &mut reg);
    reg
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
