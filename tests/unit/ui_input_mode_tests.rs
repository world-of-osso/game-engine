use super::*;
use crate::game_state::GameState;
use crate::scenes::character_frame::{CharacterFrameOpen, CharacterFramePlugin};
use game_engine::input_bindings::InputBinding;
use game_engine::ui::event::EventBus;
use game_engine::ui::registry::FrameRegistry;
use ui_toolkit::frame::{WidgetData, WidgetType};
use ui_toolkit::widgets::edit_box::EditBoxData;

/// UiState whose focused frame is a real editbox ("ChatInput").
pub(crate) fn ui_with_focused_editbox() -> UiState {
    let mut registry = FrameRegistry::new(1920.0, 1080.0);
    let id = registry.create_frame("ChatInput", None);
    let frame = registry.get_mut(id).unwrap();
    frame.widget_type = WidgetType::EditBox;
    frame.widget_data = Some(WidgetData::EditBox(EditBoxData::default()));
    UiState {
        registry,
        event_bus: EventBus::new(),
        focused_frame: Some(id),
    }
}

fn panel_app(ui: UiState) -> App {
    let mut app = App::new();
    app.add_plugins(MinimalPlugins);
    app.add_plugins(bevy::state::app::StatesPlugin);
    app.insert_state(GameState::InWorld);
    app.init_resource::<ButtonInput<KeyCode>>();
    app.init_resource::<ButtonInput<MouseButton>>();
    app.init_resource::<InputBindings>();
    app.insert_resource(ui);
    app.add_plugins(UiInputModePlugin);
    app.add_plugins(CharacterFramePlugin);
    app.update();
    app
}

fn tap(app: &mut App, key: KeyCode) {
    let mut keys = app.world_mut().resource_mut::<ButtonInput<KeyCode>>();
    keys.release_all();
    keys.clear();
    keys.press(key);
    app.update();
}

fn character_open(app: &App) -> bool {
    app.world().resource::<CharacterFrameOpen>().0
}

#[test]
fn typing_in_focused_editbox_does_not_open_character_frame() {
    let mut app = panel_app(ui_with_focused_editbox());

    tap(&mut app, KeyCode::KeyC);

    assert_eq!(*app.world().resource::<UiInputMode>(), UiInputMode::Text);
    assert!(!character_open(&app));

    app.world_mut().resource_mut::<UiState>().focused_frame = None;
    tap(&mut app, KeyCode::KeyC);

    assert_eq!(*app.world().resource::<UiInputMode>(), UiInputMode::World);
    assert!(character_open(&app), "C toggles once focus is gone");
}

#[test]
fn focused_non_editbox_frame_stays_in_world_mode() {
    let mut ui = ui_with_focused_editbox();
    let button = ui.registry.create_frame("SomeButton", None);
    ui.focused_frame = Some(button);
    let mut app = panel_app(ui);

    tap(&mut app, KeyCode::KeyC);

    assert_eq!(*app.world().resource::<UiInputMode>(), UiInputMode::World);
    assert!(character_open(&app));
}

#[test]
fn rebound_character_toggle_uses_new_key_only() {
    let mut ui = ui_with_focused_editbox();
    ui.focused_frame = None;
    let mut app = panel_app(ui);
    app.world_mut().resource_mut::<InputBindings>().assign(
        InputAction::ToggleCharacter,
        InputBinding::Keyboard(KeyCode::KeyP),
    );

    tap(&mut app, KeyCode::KeyC);
    assert!(!character_open(&app), "old default C no longer toggles");

    tap(&mut app, KeyCode::KeyP);
    assert!(character_open(&app), "P opens the character frame");
}

#[test]
fn game_menu_modal_resource_selects_modal_mode() {
    let mut ui = ui_with_focused_editbox();
    ui.focused_frame = None;
    let mut app = panel_app(ui);
    app.world_mut()
        .insert_resource(crate::scenes::game_menu::UiModalOpen);

    tap(&mut app, KeyCode::KeyC);

    assert_eq!(*app.world().resource::<UiInputMode>(), UiInputMode::Modal);
    assert!(!character_open(&app));
}

#[test]
fn dragged_action_selects_drag_mode_and_blocks_panel_toggles() {
    use game_engine::player_spells::{ActionDrag, DragSource, DraggedAction};
    let mut ui = ui_with_focused_editbox();
    ui.focused_frame = None;
    let mut app = panel_app(ui);
    app.insert_resource(ActionDrag(Some(DraggedAction {
        action: shared::protocol::ActionRef::Spell(20271),
        source: DragSource::Spellbook,
    })));

    tap(&mut app, KeyCode::KeyC);

    assert_eq!(*app.world().resource::<UiInputMode>(), UiInputMode::Drag);
    assert!(!character_open(&app));
}
