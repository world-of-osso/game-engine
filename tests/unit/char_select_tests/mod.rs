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
#[path = "../../../src/ui/screens/menu_character_layout_test_support.rs"]
mod layout_support;
pub(super) use layout_support::compute_layout as recompute_layouts;

mod campsite_tests;
mod click_tests;
mod interaction_tests;
mod layout_tests;
mod shared_state_tests;

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
    app.init_resource::<game_engine::network_runtime::messages::ConnectionSender>();
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

/// Assert the frame's computed bounds sit (`dx`, `dy`) from the target's top-left
/// corner in screen pixels (Y downward).
pub(super) fn assert_bounds_offset_from_top_left(
    reg: &FrameRegistry,
    frame_id: u64,
    relative_to: Option<u64>,
    dx: f32,
    dy: f32,
) {
    let actual = computed_bounds(reg, frame_id);
    let target = target_bounds(reg, relative_to);
    assert!(
        (actual.x - target.x - dx).abs() <= 1.0 && (actual.y - target.y - dy).abs() <= 1.0,
        "native placement {actual:?} vs target {target:?} + ({dx}, {dy})"
    );
}

/// Assert the frame's computed bounds are horizontally centered on the target
/// with their top edge `dy` pixels below the target's top edge.
pub(super) fn assert_top_edge_centered(
    reg: &FrameRegistry,
    frame_id: u64,
    relative_to: Option<u64>,
    dy: f32,
) {
    let actual = computed_bounds(reg, frame_id);
    let target = target_bounds(reg, relative_to);
    assert!(
        (actual.x + actual.width / 2.0 - (target.x + target.width / 2.0)).abs() <= 1.0
            && (actual.y - target.y - dy).abs() <= 1.0,
        "native top-center placement {actual:?} vs target {target:?} + dy {dy}"
    );
}

fn computed_bounds(reg: &FrameRegistry, frame_id: u64) -> ui_toolkit::layout::LayoutRect {
    reg.get(frame_id)
        .expect("frame")
        .layout_rect
        .clone()
        .expect("native frame bounds")
}

fn target_bounds(reg: &FrameRegistry, relative_to: Option<u64>) -> ui_toolkit::layout::LayoutRect {
    relative_to
        .map(|id| computed_bounds(reg, id))
        .unwrap_or_else(|| reg.screen_rect())
}
