//! Retail micro menu (`MainMenuBarMicroButtons.lua`): native windows toggle from their
//! button, the rest are disabled with their Retail tooltip reason, and every state draws
//! its `UI-HUD-MicroMenu-*` atlas member.

use game_engine_ui_model::micro_menu::{
    ACTION_CHARACTER, CHARACTER_PORTRAIT, MICRO_BUTTONS, MicroButtonState, MicroMenuView,
    OpenWindows, micro_button_index, micro_button_tooltip, micro_menu_screen, unavailable_message,
};
use ui_toolkit::atlas::{ActiveSkin, resolve_region};
use ui_toolkit::frame::WidgetData;
use ui_toolkit::registry::FrameRegistry;
use ui_toolkit::screen::{Screen, SharedContext};
use ui_toolkit::widgets::button::ButtonState;
use ui_toolkit::widgets::texture::TextureSource;

fn build(view: MicroMenuView) -> FrameRegistry {
    let mut registry = FrameRegistry::new(1920.0, 1080.0);
    let mut shared = SharedContext::new();
    shared.insert(ui_toolkit::atlas::ActiveSkin::Modern);
    shared.insert(view);
    Screen::new(micro_menu_screen).sync(&shared, &mut registry);
    registry
}

#[test]
fn both_presets_hide_the_micro_menu_without_destroying_its_buttons() {
    for skin in [ActiveSkin::Modern, ActiveSkin::Forever] {
        let mut registry = FrameRegistry::new(1920.0, 1080.0);
        let mut shared = SharedContext::new();
        shared.insert(skin);
        Screen::new(micro_menu_screen).sync(&shared, &mut registry);
        let menu = registry.get_by_name("MicroMenuContainer").unwrap();
        assert!(
            registry.get(menu).unwrap().hidden,
            "{skin:?} must hide the micro menu"
        );
        for button in MICRO_BUTTONS {
            assert!(registry.get_by_name(button.name).is_some());
        }
    }
}

#[test]
fn an_older_custom_layout_loads_with_the_micro_menu_hidden() {
    use game_engine_core::ui_layout_data::active_layout;
    let path = std::env::temp_dir().join(format!("micro-menu-old-{}.ron", std::process::id()));
    std::fs::write(&path, r#"(edit_mode: (layouts: {"Old": (skin: Forever, settings: (chat: (width: Some(600))))}, active_layout: {"17": "Old"}))"#).unwrap();
    let layout = active_layout(&path, 17).unwrap();
    let mut shared = SharedContext::new();
    shared.insert(ActiveSkin::Forever);
    shared.insert(layout.settings);
    let mut registry = FrameRegistry::new(1920.0, 1080.0);
    Screen::new(micro_menu_screen).sync(&shared, &mut registry);
    let menu = registry
        .get(registry.get_by_name("MicroMenuContainer").unwrap())
        .unwrap();
    assert!(!menu.visible);
    assert_eq!(layout.settings.chat.width, Some(600));
    std::fs::remove_file(path).unwrap();
}

#[test]
fn hiding_and_showing_again_keeps_the_menu_and_portrait_slot_mounted() {
    use game_engine_core::ui_layout_data::LayoutSettings;
    let mut registry = FrameRegistry::new(1920.0, 1080.0);
    let mut shared = SharedContext::new();
    shared.insert(ActiveSkin::Modern);
    let mut screen = Screen::new(micro_menu_screen);
    screen.sync(&shared, &mut registry);
    let menu = registry.get_by_name("MicroMenuContainer").unwrap();
    let portrait = registry.get_by_name(CHARACTER_PORTRAIT.frame).unwrap();
    for visible in [true, false, true] {
        shared.insert(LayoutSettings {
            show_micro_menu: Some(visible),
            ..Default::default()
        });
        screen.sync(&shared, &mut registry);
        assert_eq!(registry.get_by_name("MicroMenuContainer"), Some(menu));
        assert_eq!(
            registry.get_by_name(CHARACTER_PORTRAIT.frame),
            Some(portrait)
        );
        assert_eq!(registry.get(menu).unwrap().visible, visible);
        assert_eq!(registry.get(portrait).unwrap().visible, visible);
    }
}

fn index(name: &str) -> usize {
    micro_button_index(name).expect(name)
}

/// Atlas member a texture child draws; it must resolve to a Modern region.
fn atlas(registry: &FrameRegistry, name: &str) -> String {
    data_root();
    let frame = registry
        .get(registry.get_by_name(name).expect(name))
        .unwrap();
    let Some(WidgetData::Texture(texture)) = &frame.widget_data else {
        panic!("{name} is a texture");
    };
    let TextureSource::Atlas(atlas) = &texture.source else {
        panic!("{name} draws {:?}, not an atlas", texture.source);
    };
    let region = resolve_region(atlas, ActiveSkin::Modern).expect(atlas);
    assert!(
        region.right > region.left && region.bottom > region.top,
        "{atlas}"
    );
    atlas.clone()
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
            .chain(
                tooltip
                    .content
                    .lines
                    .iter()
                    .map(|line| line.left_text.clone()),
            )
            .collect(),
    )
}

#[test]
fn combined_player_spells_micro_button_opens_specialization_when_talents_are_absent() {
    use game_engine_ui_model::micro_menu::player_spells_micro_tab;
    use game_engine_ui_model::spellbook_frame_component::{PlayerSpellsTab, SpellbookFrameState};
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

#[test]
fn retail_order_with_every_button_lit() {
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
    // Every button is lit (user decision 2026-10-02), including those whose window is
    // not converted yet.
    assert!((0..MICRO_BUTTONS.len()).all(|i| view.state(i) == MicroButtonState::Normal));
}

#[test]
fn an_unconverted_button_is_lit_clickable_and_says_why_on_click() {
    let registry = build(MicroMenuView::default());
    let (state, alpha, onclick) = button_state(&registry, "StoreMicroButton");
    assert_eq!((state, alpha), (ButtonState::Normal, 1.0));
    assert_eq!(onclick.as_deref(), Some("micro:StoreMicroButton"));
    assert!(!atlas(&registry, "StoreMicroButtonArt1").ends_with("-Disabled"));
    assert_eq!(
        unavailable_message("micro:StoreMicroButton").as_deref(),
        Some("The shop is currently unavailable.")
    );
    assert_eq!(
        unavailable_message("micro:AchievementMicroButton").as_deref(),
        Some("This feature becomes available at level 10.")
    );
    assert_eq!(unavailable_message(ACTION_CHARACTER), None);
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
    let idle = build(MicroMenuView::default());
    // Hover swaps the icon to its mouseover art over the unchanged up background.
    assert_eq!(
        atlas(&registry, "PlayerSpellsMicroButtonArt0"),
        atlas(&idle, "PlayerSpellsMicroButtonArt0")
    );
    assert_ne!(
        atlas(&registry, "PlayerSpellsMicroButtonArt1"),
        atlas(&idle, "PlayerSpellsMicroButtonArt1")
    );
    // The open window's button draws its down icon over the down background.
    for part in ["QuestLogMicroButtonArt0", "QuestLogMicroButtonArt1"] {
        assert_ne!(atlas(&registry, part), atlas(&idle, part), "{part}");
    }
    // An untouched button keeps its up art.
    assert_eq!(
        atlas(&registry, "MainMenuMicroButtonArt1"),
        atlas(&idle, "MainMenuMicroButtonArt1")
    );
}

#[test]
fn the_character_button_draws_the_portrait_shadow_and_pushed_shadow_when_open() {
    let closed = build(MicroMenuView::default());
    atlas(&closed, "CharacterMicroButtonArt1");
    assert!(closed.get_by_name("CharacterMicroButtonArt2").is_none());
    let open = build(MicroMenuView {
        open: OpenWindows {
            character: true,
            ..OpenWindows::default()
        },
        ..MicroMenuView::default()
    });
    // Open pushes the button: its background goes down and the pushed shadow appears.
    assert_ne!(
        atlas(&open, "CharacterMicroButtonArt0"),
        atlas(&closed, "CharacterMicroButtonArt0")
    );
    atlas(&open, "CharacterMicroButtonArt2");
}

/// Children of `parent`, by name, in draw order.
fn child_names(registry: &FrameRegistry, parent: &str) -> Vec<String> {
    let frame = registry
        .get(registry.get_by_name(parent).expect(parent))
        .unwrap();
    frame
        .children
        .iter()
        .filter_map(|id| registry.get(*id)?.name.clone())
        .collect()
}

#[test]
fn the_character_button_holds_the_player_portrait_slot_under_its_mask() {
    let open_view = MicroMenuView {
        open: OpenWindows {
            character: true,
            ..OpenWindows::default()
        },
        ..MicroMenuView::default()
    };
    for (view, over) in [
        (MicroMenuView::default(), None),
        (open_view.clone(), Some("CharacterMicroButtonArt2")),
    ] {
        let registry = build(view.clone());
        let slot = view.character_portrait();
        let children = child_names(&registry, "CharacterMicroButton");
        let at = |name: &str| children.iter().position(|child| child == name);
        let portrait = at(slot.frame).expect("the character button has a portrait slot");
        // Over `Background` and `Shadow`, under `PushedShadow`.
        assert!(at("CharacterMicroButtonArt1").unwrap() < portrait);
        if let Some(over) = over {
            assert!(
                portrait < at(over).unwrap(),
                "{over} draws over the portrait"
            );
        }
        let (x, y, width, height) = slot.rect;
        assert!(x > 0.0 && y > 0.0 && x + width < 32.0 && y + height < 40.0);
        let (mx, my, mw, mh) = slot.mask_rect;
        assert!(mx <= x && my <= y && mx + mw >= x + width && my + mh >= y + height);
        assert_ne!(slot.mask_fdid, 0);
    }
    // Pushed moves the portrait and its mask (`SetPushed`).
    assert_ne!(open_view.character_portrait(), CHARACTER_PORTRAIT);
    let registry = build(MicroMenuView::default());
    for button in MICRO_BUTTONS.iter().skip(1) {
        assert!(
            !child_names(&registry, button.name)
                .iter()
                .any(|child| child.contains("Portrait")),
            "{} has no portrait",
            button.name
        );
    }
}

#[test]
fn tooltips_name_the_button_and_its_binding() {
    let view = MicroMenuView::default();
    assert_eq!(
        tooltip_rows(&view, "CharacterMicroButton", Some("C")).unwrap(),
        ["Character Info (C)"]
    );
    assert_eq!(
        tooltip_rows(&view, "StoreMicroButton", None).unwrap(),
        ["Shop"]
    );
    assert_eq!(
        tooltip_rows(&view, "AchievementMicroButton", Some("Y")).unwrap(),
        ["Achievements (Y)"]
    );
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
