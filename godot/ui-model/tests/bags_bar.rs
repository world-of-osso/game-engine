use game_engine_ui_model::bags_bar_component::{BagBarState, bags_bar_screen};
use ui_toolkit::frame::{Dimension, Frame, WidgetData};
use ui_toolkit::layout_values::Val;
use ui_toolkit::registry::FrameRegistry;
use ui_toolkit::screen::{Screen, SharedContext};
use ui_toolkit::widgets::texture::{TextureData, TextureSource};

fn frame<'a>(registry: &'a FrameRegistry, name: &str) -> &'a Frame {
    registry
        .get(registry.get_by_name(name).expect(name))
        .unwrap()
}

fn left_top(frame: &Frame) -> (f32, f32) {
    let (Val::Px(x), Val::Px(y)) = (frame.position.left, frame.position.top) else {
        panic!("{:?} is not placed in px", frame.name)
    };
    (x, y)
}

#[test]
fn bags_bar_has_no_money_in_either_preset() {
    for skin in [
        ui_toolkit::atlas::ActiveSkin::Modern,
        ui_toolkit::atlas::ActiveSkin::Forever,
    ] {
        let mut registry = FrameRegistry::new(1280.0, 720.0);
        let mut shared = SharedContext::new();
        shared.insert(skin);
        shared.insert(BagBarState::default());
        Screen::new(bags_bar_screen).sync(&shared, &mut registry);
        assert!(
            registry.get_by_name("BagsBarMoneyDisplay").is_none(),
            "{skin:?}"
        );
        assert_eq!(frame(&registry, "BagsBar").width, Dimension::Fixed(208.0));
    }
}

#[test]
fn standalone_bar_preserves_authored_buttons_art_and_geometry() {
    let mut registry = FrameRegistry::new(1280.0, 720.0);
    let mut shared = SharedContext::new();
    shared.insert(ui_toolkit::atlas::ActiveSkin::Modern);
    Screen::new(bags_bar_screen).sync(&shared, &mut registry);
    let bar = frame(&registry, "BagsBar");
    // Five bag buttons 30, BagBarExpandToggle 10, backpack 48.
    assert_eq!(bar.width, Dimension::Fixed(208.0));
    assert_eq!(bar.height, Dimension::Fixed(47.0));
    assert_eq!(bar.position.right, Val::Px(6.0));
    assert_eq!(bar.position.bottom, Val::Px(49.0));
    // Right to left: backpack, four bag slots, reagent slot; each vertically centred in
    // the bar and clear of the slot to its right.
    let mut right_neighbour_left = 208.0;
    for (name, action, size) in [
        ("MainMenuBarBackpackButton", "bag_toggle:0", 48.0),
        ("CharacterBag0Slot", "bag_toggle:1", 30.0),
        ("CharacterBag1Slot", "bag_toggle:2", 30.0),
        ("CharacterBag2Slot", "bag_toggle:3", 30.0),
        ("CharacterBag3Slot", "bag_toggle:4", 30.0),
        ("CharacterReagentBag0Slot", "bag_toggle:5", 30.0),
    ] {
        let button = frame(&registry, name);
        assert_eq!(button.onclick.as_deref(), Some(action));
        assert_eq!(button.width, Dimension::Fixed(size));
        assert_eq!(button.height, Dimension::Fixed(size));
        let (x, y) = left_top(button);
        assert!(
            x >= 0.0 && x + size <= right_neighbour_left,
            "{name} at {x}"
        );
        assert_eq!(y + size / 2.0, 47.0 / 2.0, "{name} off the bar's centre");
        right_neighbour_left = x;
        let art = frame(&registry, &format!("{name}Art"));
        assert_eq!(art.width, Dimension::Fixed(size));
        assert_eq!(art.height, Dimension::Fixed(size));
        match art.widget_data.as_ref() {
            Some(WidgetData::Texture(texture)) => {
                assert!(
                    matches!(&texture.source, TextureSource::Atlas(n) if !n.is_empty()),
                    "{name} art has no atlas"
                );
            }
            other => panic!("{name} art is not a texture: {other:?}"),
        }
    }
    assert!(registry.get_by_name("MicroMenuContainer").is_none());
    assert!(registry.get_by_name("ActionButton1").is_none());
}

fn text_of<'a>(registry: &'a FrameRegistry, name: &str) -> &'a str {
    match frame(registry, name).widget_data.as_ref() {
        Some(WidgetData::FontString(text)) => &text.text,
        other => panic!("{name} is not a font string: {other:?}"),
    }
}

/// `MainMenuBarBackpackMixin:UpdateFreeSlots`: `Count` reads `(%s)` of the free slots,
/// centred 10 below the backpack's centre (MainMenuBarBagButtons.lua:239-240, 278-289).
#[test]
fn backpack_count_shows_free_slots_under_its_centre() {
    let mut registry = FrameRegistry::new(1280.0, 720.0);
    let mut shared = SharedContext::new();
    shared.insert(ui_toolkit::atlas::ActiveSkin::Modern);
    let mut screen = Screen::new(bags_bar_screen);
    for (free_slots, expected) in [(16, "(16)"), (3, "(3)"), (0, "(0)")] {
        shared.insert(BagBarState {
            free_slots,
            ..Default::default()
        });
        screen.sync(&shared, &mut registry);
        assert_eq!(
            text_of(&registry, "MainMenuBarBackpackButtonCount"),
            expected
        );
    }
    let count = frame(&registry, "MainMenuBarBackpackButtonCount");
    assert_eq!(count.width, Dimension::Fixed(48.0));
    assert_eq!(count.height, Dimension::Fixed(14.0));
    // 48-high backpack: centre 24, +10 down, minus half the 14-high text.
    assert_eq!(count.position.top, Val::Px(27.0));
}

fn texture_of<'a>(registry: &'a FrameRegistry, name: &str) -> &'a TextureData {
    match frame(registry, name).widget_data.as_ref() {
        Some(WidgetData::Texture(texture)) => texture,
        other => panic!("{name} is not a texture: {other:?}"),
    }
}

/// `BaseBagSlotButtonMixin:UpdateTextures`: a slot holding a bag shows the bag's icon,
/// clipped to the `CircleMask` rect, under `bag-border` (`bag-reagent-border` for the
/// reagent slot); an empty slot keeps the `-empty` art and no icon.
#[test]
fn equipped_bags_show_their_icon_under_the_filled_slot_art() {
    let mut registry = FrameRegistry::new(1280.0, 720.0);
    let mut shared = SharedContext::new();
    shared.insert(ui_toolkit::atlas::ActiveSkin::Modern);
    shared.insert(BagBarState {
        bag_icons: [Some(133_633), None, None, Some(133_622), Some(4_549_293)],
        ..Default::default()
    });
    Screen::new(bags_bar_screen).sync(&shared, &mut registry);
    let empty_art = texture_of(&registry, "CharacterBag1SlotArt").source.clone();
    assert_eq!(
        texture_of(&registry, "CharacterBag2SlotArt").source,
        empty_art,
        "empty slots share the empty art"
    );
    for (slot, fdid) in [
        ("CharacterBag0Slot", 133_633),
        ("CharacterBag3Slot", 133_622),
        ("CharacterReagentBag0Slot", 4_549_293),
    ] {
        let icon_name = format!("{slot}IconTexture");
        let icon = texture_of(&registry, &icon_name);
        assert_eq!(icon.source, TextureSource::FileDataId(fdid));
        assert_eq!(
            icon.tex_coords,
            [2.0 / 30.0, 26.0 / 30.0, 2.0 / 30.0, 26.0 / 30.0]
        );
        let icon_frame = frame(&registry, &icon_name);
        assert_eq!(icon_frame.width, Dimension::Fixed(24.0));
        assert_eq!(icon_frame.position.left, Val::Px(2.0));
        assert_eq!(icon_frame.position.top, Val::Px(2.0));
        let drawn = texture_of(&registry, &format!("{slot}Art"));
        assert!(
            matches!(&drawn.source, TextureSource::Atlas(n) if !n.is_empty()),
            "{slot} has no filled art"
        );
        assert_ne!(drawn.source, empty_art, "{slot} keeps the empty art");
    }
    for slot in ["CharacterBag1Slot", "CharacterBag2Slot"] {
        assert!(
            registry
                .get_by_name(&format!("{slot}IconTexture"))
                .is_none()
        );
    }
}

/// `BagBarExpandToggle`: 10x16 `bag-arrow` left of the backpack, turned by pi while
/// expanded; collapsed, the four bag slots hide and the reagent slot moves up to the
/// toggle (MainMenuBarBagButtons.lua:259, 391, 404-425; BagsBar.lua:95-106).
#[test]
fn expand_toggle_collapses_the_four_bag_slots() {
    let mut registry = FrameRegistry::new(1280.0, 720.0);
    let mut shared = SharedContext::new();
    shared.insert(ui_toolkit::atlas::ActiveSkin::Modern);
    let mut screen = Screen::new(bags_bar_screen);
    shared.insert(BagBarState::default());
    screen.sync(&shared, &mut registry);
    let toggle = frame(&registry, "BagBarExpandToggle");
    assert_eq!(toggle.onclick.as_deref(), Some("bag_bar_expand_toggle"));
    assert_eq!(toggle.width, Dimension::Fixed(10.0));
    assert_eq!(toggle.height, Dimension::Fixed(16.0));
    // Directly left of the backpack, centred on the bar.
    let (toggle_x, toggle_y) = left_top(toggle);
    let backpack_x = left_top(frame(&registry, "MainMenuBarBackpackButton")).0;
    assert_eq!(toggle_x + 10.0, backpack_x);
    assert_eq!(toggle_y + 8.0, 47.0 / 2.0);
    let arrow = texture_of(&registry, "BagBarExpandToggleNormalTexture");
    assert!(matches!(&arrow.source, TextureSource::Atlas(n) if !n.is_empty()));
    assert_eq!(arrow.rotation, std::f32::consts::PI);

    shared.insert(BagBarState {
        collapsed: true,
        ..Default::default()
    });
    screen.sync(&shared, &mut registry);
    let hidden = |name: &str| {
        registry
            .get_by_name(name)
            .is_none_or(|id| registry.get(id).is_none_or(|f| f.hidden || !f.visible))
    };
    for i in 0..4 {
        assert!(
            hidden(&format!("CharacterBag{i}Slot")),
            "bag {i} still shown"
        );
    }
    // The reagent slot moves up against the toggle.
    assert_eq!(
        left_top(frame(&registry, "CharacterReagentBag0Slot")).0 + 30.0,
        left_top(frame(&registry, "BagBarExpandToggle")).0
    );
    assert_eq!(
        texture_of(&registry, "BagBarExpandToggleNormalTexture").rotation,
        0.0
    );
    assert!(!hidden("MainMenuBarBackpackButton"));
}

#[test]
fn backpack_draws_only_its_round_art_without_a_square_button_skin() {
    let mut registry = FrameRegistry::new(1280.0, 720.0);
    let mut shared = SharedContext::new();
    shared.insert(ui_toolkit::atlas::ActiveSkin::Modern);
    Screen::new(bags_bar_screen).sync(&shared, &mut registry);
    // MainMenuBarBackpackButton is an ItemButton with only the bag-main atlas; Retail
    // draws no square behind it.
    match &frame(&registry, "MainMenuBarBackpackButton").widget_data {
        Some(WidgetData::Button(button)) => assert!(!button.use_default_skin),
        other => panic!("backpack is not a button: {other:?}"),
    }
}
