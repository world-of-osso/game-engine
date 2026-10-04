//! Retail micro menu (`MainMenuBarMicroButtons.lua`): native windows toggle from their
//! button, the rest are disabled with their Retail tooltip reason, and every state draws
//! its `UI-HUD-MicroMenu-*` atlas member.

use game_engine_ui_model::micro_menu::{
    ACTION_CHARACTER, CHARACTER_PORTRAIT, MICRO_BUTTONS, MicroButtonState, MicroMenuView,
    OpenWindows, micro_button_index, micro_button_tooltip, micro_menu_screen, unavailable_message,
};
use ui_toolkit::atlas::{ActiveSkin, AtlasSource, resolve_region};
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

fn index(name: &str) -> usize {
    micro_button_index(name).expect(name)
}

/// `left,top` of a texture child's Modern atlas member, in 1024×512 sheet pixels.
fn crop(registry: &FrameRegistry, name: &str) -> (f32, f32) {
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
    assert_eq!(region.source, AtlasSource::FileDataId(4_708_813), "{atlas}");
    ((region.left * 1024.0).round(), (region.top * 512.0).round())
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
    // Not UI-HUD-MicroMenu-Shop-Disabled.
    assert_ne!(crop(&registry, "StoreMicroButtonArt1"), (463.0, 253.0));
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
    // UI-HUD-MicroMenu-SpecTalents-Mouseover over ButtonBG-Up.
    assert_eq!(
        crop(&registry, "PlayerSpellsMicroButtonArt0"),
        (67.0, 253.0)
    );
    assert_eq!(
        crop(&registry, "PlayerSpellsMicroButtonArt1"),
        (529.0, 253.0)
    );
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
