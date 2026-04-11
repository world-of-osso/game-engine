pub(super) use super::*;
pub(super) use std::collections::VecDeque;

pub(super) use bevy::ecs::system::RunSystemOnce;
pub(super) use bevy::input::ButtonInput;
pub(super) use bevy::input::keyboard::KeyboardInput;
pub(super) use bevy::window::PrimaryWindow;
pub(super) use game_engine::ui::automation::{
    UiAutomationAction, UiAutomationPlugin, UiAutomationQueue,
};
pub(super) use game_engine::ui::event::EventBus;
pub(super) use game_engine::ui::frame::WidgetData;
pub(super) use game_engine::ui::registry::FrameRegistry;
pub(super) use game_engine::ui::screens::char_select_component::{
    BACK_BUTTON, CharSelectAction, DELETE_CHAR_BUTTON, ENTER_WORLD_BUTTON,
};
pub(super) use game_engine::ui::strata::FrameStrata;
pub(super) use game_engine::ui::widgets::button::ButtonState;
pub(super) use shared::protocol::CharacterListEntry;
pub(super) use ui_toolkit::layout::recompute_layouts;

mod campsite_tests;
mod click_tests;
mod interaction_tests;
mod layout_tests;

pub(super) fn test_registry() -> FrameRegistry {
    FrameRegistry::new(1920.0, 1080.0)
}

pub(super) fn build_screen(state: CharSelectState) -> FrameRegistry {
    let mut reg = test_registry();
    let mut shared = ui_toolkit::screen::SharedContext::new();
    shared.insert(state);
    shared.insert(DeleteConfirmUiState::default());
    Screen::new(char_select_screen).sync(&shared, &mut reg);
    recompute_layouts(&mut reg);
    reg
}

pub(super) fn build_screen_with_real_layout(state: CharSelectState) -> FrameRegistry {
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

pub(super) fn build_screen_with_campsites(
    state: CharSelectState,
    campsite: CampsiteState,
) -> FrameRegistry {
    let mut reg = test_registry();
    let mut shared = ui_toolkit::screen::SharedContext::new();
    shared.insert(state);
    shared.insert(campsite);
    shared.insert(DeleteConfirmUiState::default());
    Screen::new(char_select_screen).sync(&shared, &mut reg);
    recompute_layouts(&mut reg);
    reg
}

pub(super) fn build_screen_with_campsites_real_layout(
    state: CharSelectState,
    campsite: CampsiteState,
) -> FrameRegistry {
    let mut reg = test_registry();
    let mut shared = ui_toolkit::screen::SharedContext::new();
    shared.insert(state);
    shared.insert(campsite);
    shared.insert(DeleteConfirmUiState::default());
    Screen::new(char_select_screen).sync(&shared, &mut reg);
    let cs = CharSelectUi::resolve(&reg);
    super::apply_post_setup(&mut reg, &cs);
    recompute_layouts(&mut reg);
    reg
}

pub(super) fn build_screen_with_delete_confirm(
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

pub(super) fn build_screen_with_delete_confirm_real_layout(
    state: CharSelectState,
    delete_confirm: DeleteConfirmUiState,
) -> FrameRegistry {
    let mut reg = test_registry();
    let mut shared = ui_toolkit::screen::SharedContext::new();
    shared.insert(state);
    shared.insert(delete_confirm);
    Screen::new(char_select_screen).sync(&shared, &mut reg);
    let cs = CharSelectUi::resolve(&reg);
    super::apply_post_setup(&mut reg, &cs);
    recompute_layouts(&mut reg);
    reg
}

pub(super) fn frame_center(reg: &FrameRegistry, name: &str) -> Vec2 {
    let rect = reg
        .get_by_name(name)
        .and_then(|id| reg.get(id))
        .and_then(|frame| frame.layout_rect.clone())
        .unwrap_or_else(|| panic!("{name} has no layout_rect"));
    Vec2::new(rect.x + rect.width * 0.5, rect.y + rect.height * 0.5)
}

pub(super) fn one_scene_campsite_state() -> CampsiteState {
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

pub(super) fn build_test_app() -> App {
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

pub(super) fn assert_single_anchor(
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
