use super::*;
use ui_toolkit::frame::Dimension;
use ui_toolkit::layout::{LayoutRect, recompute_layouts};
use ui_toolkit::registry::FrameRegistry;
use ui_toolkit::screen::{Screen, SharedContext};

fn action_bar_registry() -> FrameRegistry {
    let mut reg = FrameRegistry::new(1920.0, 1080.0);
    let shared = SharedContext::new();
    Screen::new(action_bar_screen).sync(&shared, &mut reg);
    reg
}

fn layout_registry() -> FrameRegistry {
    let mut reg = action_bar_registry();
    recompute_layouts(&mut reg);
    reg
}

fn rect(reg: &FrameRegistry, name: &str) -> LayoutRect {
    reg.get(reg.get_by_name(name).expect(name))
        .and_then(|f| f.layout_rect.clone())
        .unwrap_or_else(|| panic!("{name} has no layout_rect"))
}

#[test]
fn main_action_bar_builds_root_frame() {
    let reg = action_bar_registry();
    assert!(reg.get_by_name("MainActionBar").is_some());
}

#[test]
fn main_action_bar_builds_12_slots() {
    let reg = action_bar_registry();
    for i in 1..=SLOT_COUNT {
        assert!(
            reg.get_by_name(&format!("ActionButton{i}")).is_some(),
            "ActionButton{i} missing"
        );
    }
}

#[test]
fn main_action_bar_slots_have_hotkey_labels() {
    let reg = action_bar_registry();
    for i in 1..=SLOT_COUNT {
        assert!(
            reg.get_by_name(&format!("ActionButton{i}HotKey")).is_some(),
            "ActionButton{i}HotKey missing"
        );
    }
}

#[test]
fn main_action_bar_slots_have_count_labels() {
    let reg = action_bar_registry();
    for i in 1..=SLOT_COUNT {
        assert!(
            reg.get_by_name(&format!("ActionButton{i}Count")).is_some(),
            "ActionButton{i}Count missing"
        );
    }
}

#[test]
fn secondary_bars_build_root_frames() {
    let reg = action_bar_registry();
    assert!(reg.get_by_name("MultiBarBottomLeft").is_some());
    assert!(reg.get_by_name("MultiBarBottomRight").is_some());
    assert!(reg.get_by_name("MultiBarRight").is_some());
    assert!(reg.get_by_name("MultiBarLeft").is_some());
}

#[test]
fn bottom_left_bar_builds_12_slots() {
    let reg = action_bar_registry();
    for i in 1..=SLOT_COUNT {
        assert!(
            reg.get_by_name(&format!("MultiBarBottomLeftButton{i}"))
                .is_some(),
            "MultiBarBottomLeftButton{i} missing"
        );
    }
}

#[test]
fn bottom_right_bar_builds_12_slots() {
    let reg = action_bar_registry();
    for i in 1..=SLOT_COUNT {
        assert!(
            reg.get_by_name(&format!("MultiBarBottomRightButton{i}"))
                .is_some(),
            "MultiBarBottomRightButton{i} missing"
        );
    }
}

#[test]
fn right_bar_builds_12_slots() {
    let reg = action_bar_registry();
    for i in 1..=SLOT_COUNT {
        assert!(
            reg.get_by_name(&format!("MultiBarRightButton{i}"))
                .is_some(),
            "MultiBarRightButton{i} missing"
        );
    }
}

#[test]
fn left_bar_builds_12_slots() {
    let reg = action_bar_registry();
    for i in 1..=SLOT_COUNT {
        assert!(
            reg.get_by_name(&format!("MultiBarLeftButton{i}")).is_some(),
            "MultiBarLeftButton{i} missing"
        );
    }
}

#[test]
fn micro_menu_builds_all_buttons() {
    let reg = action_bar_registry();
    assert!(reg.get_by_name("MicroMenuContainer").is_some());
    for name in MICRO_BUTTONS {
        assert!(reg.get_by_name(name).is_some(), "{name} missing");
    }
}

#[test]
fn bag_bar_builds_backpack_and_slots() {
    let reg = action_bar_registry();
    assert!(reg.get_by_name("BagsBar").is_some());
    assert!(
        reg.get_by_name("MainMenuBarBackpackButton").is_some(),
        "backpack missing"
    );
    for i in 0..BAG_COUNT {
        assert!(
            reg.get_by_name(&format!("CharacterBag{i}Slot")).is_some(),
            "CharacterBag{i}Slot missing"
        );
    }
}

#[test]
fn bag_buttons_expose_toggle_actions() {
    let reg = action_bar_registry();
    let backpack = reg
        .get(
            reg.get_by_name("MainMenuBarBackpackButton")
                .expect("backpack"),
        )
        .expect("backpack frame");
    assert_eq!(backpack.onclick.as_deref(), Some("bag_toggle:0"));

    let bag0 = reg
        .get(reg.get_by_name("CharacterBag0Slot").expect("bag0"))
        .expect("bag0 frame");
    assert_eq!(bag0.onclick.as_deref(), Some("bag_toggle:1"));
}

#[test]
fn bag_bar_has_money_display() {
    let reg = action_bar_registry();
    assert!(
        reg.get_by_name("BagsBarMoneyDisplay").is_some(),
        "money display missing"
    );
}

// --- Coord validation ---

#[test]
fn coord_main_action_bar_slot_dimensions() {
    let reg = action_bar_registry();
    let id = reg.get_by_name("ActionButton1").expect("ActionButton1");
    let frame = reg.get(id).expect("frame data");
    assert_eq!(frame.width, Dimension::Fixed(SLOT_W));
    assert_eq!(frame.height, Dimension::Fixed(SLOT_H));
}

#[test]
fn coord_micro_menu_button_spacing() {
    let reg = layout_registry();
    let btn0 = rect(&reg, "CharacterMicroButton");
    let btn1 = rect(&reg, "SpellbookMicroButton");
    let spacing = btn1.x - btn0.x;
    let expected = MICRO_BTN_W + MICRO_BTN_GAP;
    assert!(
        (spacing - expected).abs() < 1.0,
        "micro button spacing: expected {expected}, got {spacing}"
    );
}

#[test]
fn coord_micro_menu_button_dimensions() {
    let reg = layout_registry();
    let btn = rect(&reg, "CharacterMicroButton");
    assert!(
        (btn.width - MICRO_BTN_W).abs() < 1.0,
        "width: expected {MICRO_BTN_W}, got {}",
        btn.width
    );
    assert!(
        (btn.height - MICRO_BTN_H).abs() < 1.0,
        "height: expected {MICRO_BTN_H}, got {}",
        btn.height
    );
}

#[test]
fn coord_bag_slot_spacing() {
    let reg = layout_registry();
    let backpack = rect(&reg, "MainMenuBarBackpackButton");
    let bag0 = rect(&reg, "CharacterBag0Slot");
    let spacing = bag0.x - backpack.x;
    let expected = BAG_SLOT_SIZE + BAG_SLOT_GAP;
    assert!(
        (spacing - expected).abs() < 1.0,
        "bag slot spacing: expected {expected}, got {spacing}"
    );
}

#[test]
fn coord_bag_slot_dimensions() {
    let reg = layout_registry();
    let slot = rect(&reg, "MainMenuBarBackpackButton");
    assert!(
        (slot.width - BAG_SLOT_SIZE).abs() < 1.0,
        "width: expected {BAG_SLOT_SIZE}, got {}",
        slot.width
    );
    assert!(
        (slot.height - BAG_SLOT_SIZE).abs() < 1.0,
        "height: expected {BAG_SLOT_SIZE}, got {}",
        slot.height
    );
}

// --- Minimap tests ---

fn minimap_registry() -> FrameRegistry {
    let mut reg = FrameRegistry::new(1920.0, 1080.0);
    let shared = SharedContext::new();
    Screen::new(minimap_screen).sync(&shared, &mut reg);
    reg
}

#[test]
fn minimap_builds_cluster_and_elements() {
    let reg = minimap_registry();
    assert!(reg.get_by_name("MinimapCluster").is_some());
    assert!(reg.get_by_name("MinimapDisplay").is_some());
    assert!(reg.get_by_name("MinimapBorder").is_some());
    assert!(reg.get_by_name("MinimapArrow").is_some());
    assert!(reg.get_by_name("MinimapZoneName").is_some());
    assert!(reg.get_by_name("MinimapCoords").is_some());
    assert!(reg.get_by_name("MinimapHeader").is_some());
}

#[test]
fn minimap_display_dimensions() {
    let reg = minimap_registry();
    let id = reg.get_by_name("MinimapDisplay").expect("display");
    let frame = reg.get(id).expect("data");
    assert_eq!(frame.width, Dimension::Fixed(MINIMAP_DISPLAY_SIZE));
    assert_eq!(frame.height, Dimension::Fixed(MINIMAP_DISPLAY_SIZE));
}

#[test]
fn minimap_builds_buttons_ring() {
    let reg = minimap_registry();
    for name in [
        "MinimapZoomIn",
        "MinimapZoomOut",
        "MinimapCalendarButton",
        "MinimapMailButton",
        "MinimapLFGButton",
    ] {
        assert!(reg.get_by_name(name).is_some(), "{name} missing");
    }
    let frame = reg
        .get(
            reg.get_by_name("MinimapCalendarButton")
                .expect("MinimapCalendarButton"),
        )
        .expect("calendar frame");
    assert_eq!(frame.onclick.as_deref(), Some("calendar_toggle"));

    let guild = reg
        .get(
            reg.get_by_name("GuildMicroButton")
                .expect("GuildMicroButton"),
        )
        .expect("guild button");
    assert_eq!(guild.onclick.as_deref(), Some("guild_toggle"));
}
