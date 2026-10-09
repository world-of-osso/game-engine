use game_engine_core::client_options_data::ExtraActionBars;
use game_engine_ui_model::hud_edit::EditModeActive;
use game_engine_ui_model::main_action_bar_component::{
    ActionBar, MainActionBarState, main_action_bar_screen, parse_action_button,
};
use ui_toolkit::atlas::ActiveSkin;
use ui_toolkit::registry::FrameRegistry;
use ui_toolkit::screen::{Screen, SharedContext};

#[test]
fn extra_action_bars_explicit_bottom_choices_override_gameplay_and_editor_defaults() {
    for skin in [ActiveSkin::Modern, ActiveSkin::Forever] {
        for editing in [false, true] {
            let mut registry = FrameRegistry::new(1366.0, 768.0);
            let mut shared = SharedContext::new();
            shared.insert(skin);
            shared.insert(EditModeActive(editing));
            let mut screen = Screen::new(main_action_bar_screen);
            for enabled in [false, true, false] {
                shared.insert(MainActionBarState {
                    extra_action_bars: ExtraActionBars {
                        action_bar_2: Some(enabled),
                        action_bar_3: Some(enabled),
                        ..Default::default()
                    },
                    ..Default::default()
                });
                screen.sync(&shared, &mut registry);
                assert_eq!(
                    registry.get_by_name("MultiBarBottomLeft").is_some(),
                    enabled
                );
                assert_eq!(
                    registry.get_by_name("MultiBarBottomRight").is_some(),
                    enabled
                );
                assert!(registry.get_by_name("MainActionBar").is_some());
            }
        }
    }
}

#[test]
fn extra_action_bars_side_buttons_emit_their_existing_retail_action_pages() {
    for skin in [ActiveSkin::Modern, ActiveSkin::Forever] {
        let mut registry = FrameRegistry::new(1366.0, 768.0);
        let mut shared = SharedContext::new();
        shared.insert(skin);
        shared.insert(MainActionBarState {
            extra_action_bars: ExtraActionBars {
                action_bar_4: true,
                action_bar_5: true,
                ..Default::default()
            },
            ..Default::default()
        });
        Screen::new(main_action_bar_screen).sync(&shared, &mut registry);
        for (bar, first_slot, last_slot) in [(ActionBar::Right, 24, 35), (ActionBar::Left, 36, 47)]
        {
            for (index, slot) in [(0, first_slot), (11, last_slot)] {
                let id = registry.get_by_name(&bar.button_name(index)).unwrap();
                let action = registry.click_frame(id).unwrap();
                let (clicked_bar, clicked_index) = parse_action_button(&action).unwrap();
                assert_eq!((clicked_bar, clicked_index), (bar, index));
                assert_eq!(clicked_bar.action_slot(clicked_index), slot);
            }
        }
    }
}
