use game_engine_ui_model::bags_bar_component::{BagBarState, bags_bar_screen};
use ui_toolkit::frame::{Dimension, Frame, WidgetData};
use ui_toolkit::layout_values::Val;
use ui_toolkit::registry::FrameRegistry;
use ui_toolkit::screen::{Screen, SharedContext};
use ui_toolkit::widgets::texture::TextureSource;

fn frame<'a>(registry: &'a FrameRegistry, name: &str) -> &'a Frame {
    registry
        .get(registry.get_by_name(name).expect(name))
        .unwrap()
}

fn money_text(registry: &FrameRegistry) -> &str {
    match frame(registry, "BagsBarMoneyDisplay").widget_data.as_ref() {
        Some(WidgetData::FontString(text)) => &text.text,
        other => panic!("money is not a font string: {other:?}"),
    }
}

#[test]
fn standalone_bar_preserves_authored_buttons_art_and_geometry() {
    let mut registry = FrameRegistry::new(1280.0, 720.0);
    Screen::new(bags_bar_screen).sync(&SharedContext::new(), &mut registry);
    let bar = frame(&registry, "BagsBar");
    assert_eq!(bar.width, Dimension::Fixed(328.0));
    assert_eq!(bar.height, Dimension::Fixed(47.0));
    assert_eq!(bar.position.right, Val::Px(6.0));
    assert_eq!(bar.position.bottom, Val::Px(49.0));
    for (name, action, size, x, y) in [
        (
            "MainMenuBarBackpackButton",
            "bag_toggle:0",
            48.0,
            280.0,
            -0.5,
        ),
        ("CharacterBag0Slot", "bag_toggle:1", 30.0, 250.0, 8.5),
        ("CharacterBag1Slot", "bag_toggle:2", 30.0, 220.0, 8.5),
        ("CharacterBag2Slot", "bag_toggle:3", 30.0, 190.0, 8.5),
        ("CharacterBag3Slot", "bag_toggle:4", 30.0, 160.0, 8.5),
    ] {
        let button = frame(&registry, name);
        assert_eq!(button.onclick.as_deref(), Some(action));
        assert_eq!(button.width, Dimension::Fixed(size));
        assert_eq!(button.height, Dimension::Fixed(size));
        assert_eq!(button.position.left, Val::Px(x));
        assert_eq!(button.position.top, Val::Px(y));
        let art = frame(&registry, &format!("{name}Art"));
        assert_eq!(art.width, Dimension::Fixed(size));
        assert_eq!(art.height, Dimension::Fixed(size));
        match art.widget_data.as_ref() {
            Some(WidgetData::Texture(texture)) => {
                assert_eq!(texture.source, TextureSource::FileDataId(4_691_255));
                let expected_crop = if name == "MainMenuBarBackpackButton" {
                    [1.0 / 512.0, 97.0 / 512.0, 1.0 / 128.0, 97.0 / 128.0]
                } else {
                    [295.0 / 512.0, 356.0 / 512.0, 64.0 / 128.0, 125.0 / 128.0]
                };
                assert_eq!(texture.tex_coords, expected_crop);
            }
            other => panic!("{name} art is not a texture: {other:?}"),
        }
    }
    let money = frame(&registry, "BagsBarMoneyDisplay");
    assert_eq!(money.width, Dimension::Fixed(154.0));
    assert_eq!(money.height, Dimension::Fixed(14.0));
    assert_eq!(money.position.top, Val::Px(16.5));
    assert_eq!(money_text(&registry), "0g 0s 0c");
    assert!(registry.get_by_name("MicroMenuContainer").is_none());
    assert!(registry.get_by_name("ActionButton1").is_none());
}

#[test]
fn money_state_sync_matches_original_updater_denominations() {
    let mut registry = FrameRegistry::new(1280.0, 720.0);
    let mut shared = SharedContext::new();
    let mut screen = Screen::new(bags_bar_screen);
    for (money, expected) in [
        (12345, "1g 23s 45c"),
        (345, "3s 45c"),
        (45, "45c"),
        (0, "0c"),
    ] {
        shared.insert(BagBarState {
            money,
            free_slots: 0,
        });
        screen.sync(&shared, &mut registry);
        assert_eq!(money_text(&registry), expected);
    }
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
    let mut screen = Screen::new(bags_bar_screen);
    for (free_slots, expected) in [(16, "(16)"), (3, "(3)"), (0, "(0)")] {
        shared.insert(BagBarState {
            money: 0,
            free_slots,
        });
        screen.sync(&shared, &mut registry);
        assert_eq!(text_of(&registry, "MainMenuBarBackpackButtonCount"), expected);
    }
    let count = frame(&registry, "MainMenuBarBackpackButtonCount");
    assert_eq!(count.width, Dimension::Fixed(48.0));
    assert_eq!(count.height, Dimension::Fixed(14.0));
    // 48-high backpack: centre 24, +10 down, minus half the 14-high text.
    assert_eq!(count.position.top, Val::Px(27.0));
}
