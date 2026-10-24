//! Existing combined Player Spells case, retaining its original module/test name.
use game_engine_ui_model::micro_menu::{
    ACTION_CHARACTER, MICRO_BUTTONS, MicroMenuView, micro_button_index, micro_menu_screen,
};
use ui_toolkit::frame::WidgetData;
use ui_toolkit::registry::FrameRegistry;
use ui_toolkit::screen::{Screen, SharedContext};
use ui_toolkit::widgets::button::ButtonState;

fn build(view: MicroMenuView) -> FrameRegistry {
    let mut registry = FrameRegistry::new(1920.0, 1080.0);
    let mut shared = SharedContext::new();
    shared.insert(ui_toolkit::atlas::ActiveSkin::Modern);
    shared.insert(view);
    Screen::new(micro_menu_screen).sync(&shared, &mut registry);
    registry
}

fn index(name: &str) -> usize {
    micro_button_index(name).expect(name)
}

fn button_state(registry: &FrameRegistry, name: &str) -> (ButtonState, f32, Option<String>) {
    let frame = registry
        .get(registry.get_by_name(name).expect(name))
        .unwrap();
    let Some(WidgetData::Button(button)) = &frame.widget_data else {
        panic!("{name} is a button");
    };
    (button.state, frame.alpha, frame.onclick.clone())
}

#[test]
fn combined_player_spells_micro_button_opens_specialization_when_talents_are_absent() {
    use game_engine_ui_model::micro_menu::player_spells_micro_tab;
    use game_engine_ui_model::player_spells_data::PlayerSpellsTab;
    use game_engine_ui_player_spells::spellbook_frame_component::SpellbookFrameState;
    let registry = build(MicroMenuView::default());
    let (_, _, action) = button_state(&registry, "PlayerSpellsMicroButton");
    let tab = player_spells_micro_tab(action.as_deref().unwrap()).unwrap();
    let mut book = SpellbookFrameState::default();
    assert!(book.toggle_tab(false, tab));
    assert_eq!(book.tab, PlayerSpellsTab::Specialization);
    assert_eq!(
        MICRO_BUTTONS[index("PlayerSpellsMicroButton")].binding,
        Some(game_engine_core::input_bindings_data::InputAction::ToggleTalents)
    );
    assert!(player_spells_micro_tab(ACTION_CHARACTER).is_none());
}
