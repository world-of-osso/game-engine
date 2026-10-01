//! Native CharacterFrame / paperdoll (docs/specs/character-frame.md) over the real item
//! catalog: Worn Shortsword 25 (one-hand, item level 1), Battleworn Bludgeon 2361
//! (two-hand, 1), Tarnished Chain Vest 2379 (chest, 2), Footpad's Shirt 49 (shirt, 1).

use std::path::PathBuf;

use game_engine_ui_model::bag_data::InventoryState;
use game_engine_ui_model::character_frame::{
    ACTION_FRAME, CharacterFrameView, MIN_LEVEL_FOR_ITEM_LEVEL, PAPERDOLL_BUTTONS,
    apply_character_frame_postsetup, average_equipped_item_level, character_frame_screen,
    level_line, paperdoll_button, paperdoll_slots, parse_equipment_slot_action,
};
use game_engine_ui_model::item_catalog::item_catalog_entry;
use game_engine_ui_model::micro_menu::{ACTION_CHARACTER, micro_menu_screen};
use shared::protocol::{
    EquipmentSlot, EquipmentSnapshot, EquippedItem, InventoryDelta, InventorySlotChange,
    ItemLocation, ItemStack,
};
use ui_toolkit::frame::WidgetData;
use ui_toolkit::registry::FrameRegistry;
use ui_toolkit::screen::{Screen, SharedContext};
use ui_toolkit::widgets::texture::TextureSource;

fn data_root() {
    game_engine_ui_model::paths::set_data_root(
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../data"),
    )
    .unwrap();
}

fn stack(guid: u64, item_id: u32) -> ItemStack {
    ItemStack {
        item_guid: guid,
        item_id,
        count: 1,
        durability: None,
        soulbound: false,
    }
}

fn equipped(items: &[(EquipmentSlot, u32)]) -> InventoryState {
    data_root();
    let mut inventory = InventoryState::default();
    inventory.apply_equipment_snapshot(&EquipmentSnapshot {
        items: items
            .iter()
            .enumerate()
            .map(|(index, &(slot, item_id))| EquippedItem {
                slot,
                item: stack(9_000 + index as u64, item_id),
            })
            .collect(),
    });
    inventory
}

fn catalog_level(item_id: u32) -> Option<(u16, u8)> {
    item_catalog_entry(item_id).map(|entry| (entry.item_level, entry.inventory_type))
}

fn build(view: CharacterFrameView) -> FrameRegistry {
    data_root();
    let mut registry = FrameRegistry::new(1920.0, 1080.0);
    let mut shared = SharedContext::new();
    shared.insert(view);
    Screen::new(character_frame_screen).sync(&shared, &mut registry);
    apply_character_frame_postsetup(&mut registry);
    registry
}

fn view(inventory: &InventoryState, cursor: Option<ItemLocation>) -> CharacterFrameView {
    CharacterFrameView {
        visible: true,
        title: "Theron".into(),
        level: level_line(12, Some("Protection"), "Warrior", [0.78, 0.61, 0.43]),
        slots: paperdoll_slots(inventory, cursor),
        item_level: Some("1".into()),
        race_id: 1,
        class_id: 1,
    }
}

fn texture_fdid(registry: &FrameRegistry, name: &str) -> Option<u32> {
    let frame = registry.get(registry.get_by_name(name)?)?;
    match frame.widget_data.as_ref()? {
        WidgetData::Texture(texture) => match texture.source {
            TextureSource::FileDataId(fdid) => Some(fdid),
            _ => None,
        },
        _ => None,
    }
}

fn text(registry: &FrameRegistry, name: &str) -> String {
    let frame = registry
        .get(registry.get_by_name(name).expect(name))
        .unwrap();
    match frame.widget_data.as_ref() {
        Some(WidgetData::FontString(text)) => text.text.clone(),
        _ => panic!("{name} is not a font string"),
    }
}

fn onclick(registry: &FrameRegistry, name: &str) -> Option<String> {
    registry
        .get(registry.get_by_name(name).expect(name))
        .unwrap()
        .onclick
        .clone()
}

#[test]
fn every_retail_paperdoll_button_clicks_with_its_own_equipment_slot() {
    let registry = build(view(&InventoryState::default(), None));
    assert_eq!(PAPERDOLL_BUTTONS.len(), 18);
    for button in &PAPERDOLL_BUTTONS {
        let action = onclick(&registry, button.name).expect(button.name);
        assert_eq!(parse_equipment_slot_action(&action), Some(button.slot));
    }
    assert!(paperdoll_button(EquipmentSlot::Ranged).is_none());
}

#[test]
fn empty_slots_show_their_paperdoll_slot_textures_and_equipped_slots_their_item_icon() {
    let inventory = equipped(&[(EquipmentSlot::MainHand, 25), (EquipmentSlot::Chest, 2379)]);
    let registry = build(view(&inventory, None));
    let sword_icon = item_catalog_entry(25).unwrap().icon_fdid;
    assert_eq!(
        texture_fdid(&registry, "CharacterMainHandSlotIconTexture"),
        Some(sword_icon)
    );
    // UI-PaperDoll-Slot-Head / -SecondaryHand / -Finger.
    assert_eq!(
        texture_fdid(&registry, "CharacterHeadSlotIconTexture"),
        Some(136_516)
    );
    assert_eq!(
        texture_fdid(&registry, "CharacterSecondaryHandSlotIconTexture"),
        Some(136_524)
    );
    assert_eq!(
        texture_fdid(&registry, "CharacterFinger1SlotIconTexture"),
        Some(136_514)
    );
    // Common quality: the grey WhiteIconFrame border; empty slots have none.
    assert_eq!(
        texture_fdid(&registry, "CharacterChestSlotIconBorder"),
        Some(651_080)
    );
    assert_eq!(texture_fdid(&registry, "CharacterHeadSlotIconBorder"), None);
}

#[test]
fn a_delta_unequipping_the_weapon_restores_the_empty_slot() {
    let mut inventory = equipped(&[(EquipmentSlot::MainHand, 25)]);
    inventory.apply_delta(&InventoryDelta {
        changes: vec![InventorySlotChange {
            location: ItemLocation::Equipment(EquipmentSlot::MainHand),
            item: None,
        }],
    });
    let registry = build(view(&inventory, None));
    assert_eq!(
        texture_fdid(&registry, "CharacterMainHandSlotIconTexture"),
        Some(136_518)
    );
}

#[test]
fn the_slot_on_the_cursor_is_dimmed() {
    let inventory = equipped(&[(EquipmentSlot::MainHand, 25), (EquipmentSlot::Chest, 2379)]);
    let slots = paperdoll_slots(
        &inventory,
        Some(ItemLocation::Equipment(EquipmentSlot::MainHand)),
    );
    let index = |slot| {
        PAPERDOLL_BUTTONS
            .iter()
            .position(|button| button.slot == slot)
            .unwrap()
    };
    assert!(slots[index(EquipmentSlot::MainHand)].locked);
    assert!(!slots[index(EquipmentSlot::Chest)].locked);
}

#[test]
fn buttons_sit_at_the_retail_anchors() {
    let origin = |slot| paperdoll_button(slot).unwrap().origin();
    // Head TOPLEFT Inset(4,-60) +4,-2; each next -4 below a 37-unit button.
    assert_eq!(origin(EquipmentSlot::Head), (8.0, 62.0));
    assert_eq!(origin(EquipmentSlot::Wrist), (8.0, 62.0 + 7.0 * 41.0));
    // Hands TOPRIGHT Inset(332) -4,-2.
    assert_eq!(origin(EquipmentSlot::Hands), (291.0, 62.0));
    assert_eq!(origin(EquipmentSlot::Trinket2), (291.0, 62.0 + 7.0 * 41.0));
    // Main hand BOTTOMLEFT 130,16 of the 424-high frame; off hand +5 to its right.
    assert_eq!(origin(EquipmentSlot::MainHand), (130.0, 371.0));
    assert_eq!(origin(EquipmentSlot::OffHand), (172.0, 371.0));
}

#[test]
fn average_item_level_counts_sixteen_slots_and_a_two_hander_twice() {
    // Vest 2 + one-hand 1 + shirt (not counted) over 16.
    let one_hand = equipped(&[
        (EquipmentSlot::Chest, 2379),
        (EquipmentSlot::MainHand, 25),
        (EquipmentSlot::Shirt, 49),
    ]);
    assert_eq!(
        average_equipped_item_level(&one_hand, catalog_level),
        3.0 / 16.0
    );
    let two_hand = equipped(&[
        (EquipmentSlot::Chest, 2379),
        (EquipmentSlot::MainHand, 2361),
    ]);
    assert_eq!(
        average_equipped_item_level(&two_hand, catalog_level),
        4.0 / 16.0
    );
}

#[test]
fn level_line_uses_player_level_with_and_without_a_spec() {
    let with_spec = level_line(12, Some("Protection"), "Warrior", [0.78, 0.61, 0.43]);
    assert_eq!(with_spec.level, "Level 12 ");
    assert_eq!(with_spec.class_text, "Protection Warrior");
    assert_eq!(with_spec.class_color, "0.78,0.61,0.43,1.0");
    assert_eq!(level_line(1, None, "Mage", [1.0; 3]).class_text, "Mage");
    let registry = build(view(&InventoryState::default(), None));
    assert_eq!(text(&registry, "CharacterLevelText"), "Level 12 ");
    assert_eq!(
        text(&registry, "CharacterLevelTextClass"),
        "Protection Warrior"
    );
}

#[test]
fn item_level_shows_from_level_ten_and_is_hidden_below() {
    assert_eq!(MIN_LEVEL_FOR_ITEM_LEVEL, 10);
    let registry = build(view(&InventoryState::default(), None));
    assert_eq!(
        text(&registry, "CharacterStatsPaneItemLevelCategoryTitle"),
        "Item Level"
    );
    assert_eq!(
        text(&registry, "CharacterStatsPaneItemLevelFrameValue"),
        "1"
    );
    let mut low = view(&InventoryState::default(), None);
    low.item_level = None;
    let registry = build(low);
    assert!(
        registry
            .get_by_name("CharacterStatsPaneItemLevelFrameValue")
            .is_none()
    );
}

#[test]
fn title_is_white_and_the_race_backdrop_desaturated() {
    let registry = build(view(&InventoryState::default(), None));
    let title = registry
        .get(registry.get_by_name("CharacterFrameTitleText").unwrap())
        .unwrap();
    let Some(WidgetData::FontString(title)) = &title.widget_data else {
        panic!("title is a font string");
    };
    assert_eq!(title.text, "Theron");
    assert_eq!(title.color, [1.0; 4]);
    // DressUpBackground-Human1..4.
    assert_eq!(
        texture_fdid(&registry, "CharacterModelFrameBackgroundTopLeft"),
        Some(131_093)
    );
    assert_eq!(
        texture_fdid(&registry, "CharacterModelFrameBackgroundBotRight"),
        Some(131_096)
    );
    let quarter = registry
        .get(
            registry
                .get_by_name("CharacterModelFrameBackgroundTopLeft")
                .unwrap(),
        )
        .unwrap();
    let Some(WidgetData::Texture(quarter)) = &quarter.widget_data else {
        panic!("quarter is a texture");
    };
    assert!(quarter.desaturated);
}

#[test]
fn hidden_view_hides_the_frame_and_the_frame_itself_blocks_clicks() {
    let mut hidden = view(&InventoryState::default(), None);
    hidden.visible = false;
    let registry = build(hidden);
    let frame = registry
        .get(registry.get_by_name("CharacterFrame").unwrap())
        .unwrap();
    assert!(!frame.visible);
    assert_eq!(
        onclick(&registry, "CharacterFrame").as_deref(),
        Some(ACTION_FRAME)
    );
}

#[test]
fn micro_menu_character_button_toggles_the_character_frame() {
    data_root();
    let mut registry = FrameRegistry::new(1920.0, 1080.0);
    let mut shared = SharedContext::new();
    shared.insert(());
    Screen::new(micro_menu_screen).sync(&shared, &mut registry);
    assert_eq!(
        onclick(&registry, "CharacterMicroButton").as_deref(),
        Some(ACTION_CHARACTER)
    );
    assert_eq!(
        onclick(&registry, "AchievementMicroButton").as_deref(),
        Some("")
    );
}
