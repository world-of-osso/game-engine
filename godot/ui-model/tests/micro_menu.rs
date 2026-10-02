//! Retail micro menu (`MainMenuBarMicroButtons.lua`): native windows toggle from their
//! button, the rest are disabled with their Retail tooltip reason, and every state draws
//! its `UI-HUD-MicroMenu-*` atlas member.

use game_engine_ui_model::micro_menu::{
    ACTION_CHARACTER, MICRO_BUTTONS, MicroButtonState, MicroMenuView, OpenWindows,
    micro_button_index, micro_button_tooltip, micro_menu_screen,
};
use ui_toolkit::frame::WidgetData;
use ui_toolkit::registry::FrameRegistry;
use ui_toolkit::screen::{Screen, SharedContext};
use ui_toolkit::widgets::button::ButtonState;

fn build(view: MicroMenuView) -> FrameRegistry {
    let mut registry = FrameRegistry::new(1920.0, 1080.0);
    let mut shared = SharedContext::new();
    shared.insert(view);
    Screen::new(micro_menu_screen).sync(&shared, &mut registry);
    registry
}

fn index(name: &str) -> usize {
    micro_button_index(name).expect(name)
}

/// Normalized `left,top` of a texture child, in 1024×512 sheet pixels.
fn crop(registry: &FrameRegistry, name: &str) -> (f32, f32) {
    let frame = registry
        .get(registry.get_by_name(name).expect(name))
        .unwrap();
    let Some(WidgetData::Texture(texture)) = &frame.widget_data else {
        panic!("{name} is a texture");
    };
    let [left, _, top, _] = texture.tex_coords;
    ((left * 1024.0).round(), (top * 512.0).round())
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

/// Tooltip lines wrap with the real FrizQuadrata metrics.
fn data_root() {
    game_engine_ui_model::paths::set_data_root(
        std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../data"),
    )
    .unwrap();
}

fn tooltip_rows(view: &MicroMenuView, name: &str, key: Option<&str>) -> Option<Vec<String>> {
    data_root();
    let tooltip = micro_button_tooltip(view, index(name), key)?;
    Some(
        std::iter::once(tooltip.content.title.clone())
            .chain(tooltip.content.lines.iter().map(|line| line.left_text.clone()))
            .collect(),
    )
}

#[test]
fn retail_order_with_native_windows_enabled_and_the_rest_disabled() {
    let names: Vec<_> = MICRO_BUTTONS.iter().map(|button| button.name).collect();
    assert_eq!(
        names,
        [
            "CharacterMicroButton",
            "ProfessionMicroButton",
            "PlayerSpellsMicroButton",
            "AchievementMicroButton",
            "QuestLogMicroButton",
            "HousingMicroButton",
            "GuildMicroButton",
            "LFDMicroButton",
            "CollectionsMicroButton",
            "EJMicroButton",
            "StoreMicroButton",
            "MainMenuMicroButton",
        ]
    );
    let view = MicroMenuView::default();
    let enabled: Vec<_> = (0..MICRO_BUTTONS.len())
        .filter(|&i| view.state(i) == MicroButtonState::Normal)
        .map(|i| MICRO_BUTTONS[i].name)
        .collect();
    assert_eq!(
        enabled,
        [
            "CharacterMicroButton",
            "PlayerSpellsMicroButton",
            "QuestLogMicroButton",
            "MainMenuMicroButton",
        ]
    );
}

#[test]
fn a_disabled_button_draws_its_disabled_atlas_at_half_alpha_and_does_not_click() {
    let registry = build(MicroMenuView::default());
    let (state, alpha, _) = button_state(&registry, "StoreMicroButton");
    assert_eq!(state, ButtonState::Disabled);
    assert_eq!(alpha, 0.5);
    // UI-HUD-MicroMenu-ButtonBG-Up, UI-HUD-MicroMenu-Shop-Disabled.
    assert_eq!(crop(&registry, "StoreMicroButtonArt0"), (67.0, 253.0));
    assert_eq!(crop(&registry, "StoreMicroButtonArt1"), (463.0, 253.0));
    let (state, alpha, onclick) = button_state(&registry, "CharacterMicroButton");
    assert_eq!((state, alpha), (ButtonState::Normal, 1.0));
    assert_eq!(onclick.as_deref(), Some(ACTION_CHARACTER));
}

#[test]
fn hover_shows_the_mouseover_atlas_and_an_open_window_pushes_its_button() {
    let view = MicroMenuView {
        open: OpenWindows {
            quest_log: true,
            ..OpenWindows::default()
        },
        hovered: Some(index("PlayerSpellsMicroButton")),
        pressed: None,
    };
    let registry = build(view);
    // UI-HUD-MicroMenu-SpecTalents-Mouseover over ButtonBG-Up.
    assert_eq!(crop(&registry, "PlayerSpellsMicroButtonArt0"), (67.0, 253.0));
    assert_eq!(crop(&registry, "PlayerSpellsMicroButtonArt1"), (529.0, 253.0));
    // UI-HUD-MicroMenu-Questlog-Down over ButtonBG-Down.
    assert_eq!(crop(&registry, "QuestLogMicroButtonArt0"), (67.0, 169.0));
    assert_eq!(crop(&registry, "QuestLogMicroButtonArt1"), (463.0, 1.0));
    // UI-HUD-MicroMenu-GameMenu-Up.
    assert_eq!(crop(&registry, "MainMenuMicroButtonArt1"), (133.0, 421.0));
}

#[test]
fn the_character_button_draws_the_portrait_shadow_and_pushed_shadow_when_open() {
    let closed = build(MicroMenuView::default());
    assert_eq!(crop(&closed, "CharacterMicroButtonArt1"), (397.0, 1.0));
    assert!(closed.get_by_name("CharacterMicroButtonArt2").is_none());
    let open = build(MicroMenuView {
        open: OpenWindows {
            character: true,
            ..OpenWindows::default()
        },
        ..MicroMenuView::default()
    });
    assert_eq!(crop(&open, "CharacterMicroButtonArt0"), (67.0, 169.0));
    // UI-HUD-MicroMenu-Portrait-Down.
    assert_eq!(crop(&open, "CharacterMicroButtonArt2"), (331.0, 421.0));
}

#[test]
fn tooltips_name_the_button_and_its_binding_and_say_why_a_button_is_disabled() {
    let view = MicroMenuView::default();
    assert_eq!(
        tooltip_rows(&view, "CharacterMicroButton", Some("C")).unwrap(),
        ["Character Info (C)"]
    );
    assert_eq!(
        tooltip_rows(&view, "StoreMicroButton", None).unwrap(),
        ["Shop", "The shop is currently unavailable."]
    );
    assert_eq!(
        tooltip_rows(&view, "AchievementMicroButton", Some("Y")).unwrap(),
        [
            "Achievements (Y)",
            "This feature becomes available at level 10."
        ]
    );
    let store = micro_button_tooltip(&view, index("StoreMicroButton"), None).unwrap();
    assert_eq!(store.content.lines[0].left_color, [1.0, 0.125, 0.125, 1.0]);
}

#[test]
fn an_open_game_menu_pushes_its_button_and_disables_the_rest_without_tooltips() {
    let view = MicroMenuView {
        open: OpenWindows {
            game_menu: true,
            character: true,
            ..OpenWindows::default()
        },
        ..MicroMenuView::default()
    };
    assert_eq!(
        view.state(index("MainMenuMicroButton")),
        MicroButtonState::Pushed
    );
    assert_eq!(
        view.state(index("CharacterMicroButton")),
        MicroButtonState::Disabled
    );
    assert!(tooltip_rows(&view, "StoreMicroButton", None).is_none());
    assert_eq!(
        tooltip_rows(&view, "MainMenuMicroButton", None).unwrap(),
        ["Game Menu"]
    );
}
