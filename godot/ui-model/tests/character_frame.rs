//! Native CharacterFrame / paperdoll (docs/specs/character-frame.md) over the real item
//! catalog: Worn Shortsword 25 (one-hand, item level 1), Battleworn Bludgeon 2361
//! (two-hand, 1), Tarnished Chain Vest 2379 (chest, 2), Footpad's Shirt 49 (shirt, 1).

use std::path::PathBuf;

use game_engine_core::spell_catalog::PrimaryStat;
use game_engine_ui_model::bag_data::InventoryState;
use game_engine_ui_model::character_frame::{
    ACTION_FRAME, CharacterFrameView, MIN_LEVEL_FOR_ITEM_LEVEL, PAPERDOLL_BUTTONS,
    apply_character_frame_postsetup, attribute_lines, average_equipped_item_level,
    break_up_large_numbers, character_frame_screen, enhancement_lines, level_line,
    paperdoll_button, paperdoll_slots, parse_equipment_slot_action,
};
use game_engine_ui_model::item_catalog::item_catalog_entry;
use shared::components::{CombatRatings, DerivedStats, UnitStats};
use shared::protocol::{
    EquipmentSlot, EquipmentSnapshot, EquippedItem, InventoryDelta, InventorySlotChange,
    ItemLocation, ItemStack,
};
use ui_toolkit::frame::WidgetData;
use ui_toolkit::layout_values::Val;
use ui_toolkit::registry::FrameRegistry;
use ui_toolkit::screen::{Screen, SharedContext};
use ui_toolkit::widgets::texture::TextureSource;

fn data_root() {
    game_engine_ui_model::paths::set_data_root(
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../data"),
    )
    .unwrap();
    game_engine_ui_model::item_catalog::wait_for_item_catalog();
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
    let tab = view.tab;
    shared.insert(view);
    Screen::new(character_frame_screen).sync(&shared, &mut registry);
    apply_character_frame_postsetup(&mut registry, tab);
    registry
}

fn view(inventory: &InventoryState, cursor: Option<ItemLocation>) -> CharacterFrameView {
    CharacterFrameView {
        visible: true,
        title: "Theron".into(),
        level: level_line(12, Some("Protection"), "Warrior", [0.78, 0.61, 0.43]),
        slots: paperdoll_slots(inventory, cursor),
        item_level: Some("1".into()),
        attributes: Vec::new(),
        enhancements: Vec::new(),
        race_id: 1,
        class_id: 1,
        ..CharacterFrameView::default()
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

/// Level 5 human warrior with a Notched Shortsword (+1 Agility, +1 Stamina) and some
/// armor, as the server replicates it.
fn warrior_sheet() -> (UnitStats, CombatRatings) {
    (
        UnitStats {
            stamina: 22.6,
            strength: 25.0,
            agility: 18.0,
            intellect: 10.0,
            spirit: 0.0,
        },
        CombatRatings {
            armor: 1234.0,
            ..CombatRatings::default()
        },
    )
}

#[test]
fn attributes_list_primaries_stamina_and_armor_as_retail_integers() {
    let (stats, ratings) = warrior_sheet();
    let lines = attribute_lines(Some((&stats, &ratings)), None);
    let shown: Vec<_> = lines
        .iter()
        .map(|line| (line.label, line.value.as_str()))
        .collect();
    assert_eq!(
        shown,
        [
            ("Strength:", "25"),
            ("Agility:", "18"),
            ("Intellect:", "10"),
            ("Stamina:", "22"),
            ("Armor:", "1,234"),
        ]
    );
    assert!(attribute_lines(None, None).is_empty());
    assert_eq!(break_up_large_numbers(1_234_567), "1,234,567");
    assert_eq!(break_up_large_numbers(999), "999");
    assert_eq!(break_up_large_numbers(-1_000), "-1,000");
}

#[test]
fn attributes_category_sits_under_the_item_level_and_hides_without_stats() {
    let (stats, ratings) = warrior_sheet();
    let mut sheet = view(&InventoryState::default(), None);
    sheet.attributes = attribute_lines(Some((&stats, &ratings)), None);
    let registry = build(sheet.clone());
    assert_eq!(
        text(&registry, "CharacterStatsPaneAttributesCategoryTitle"),
        "Attributes"
    );
    assert_eq!(text(&registry, "CharacterStatsPaneStat1Label"), "Strength:");
    assert_eq!(text(&registry, "CharacterStatsPaneStat5Value"), "1,234");
    // Every second line has the bounce band (`numStatInCat % 2 == 0`).
    assert!(
        registry
            .get_by_name("CharacterStatsPaneStat1Background")
            .is_none()
    );
    assert!(
        registry
            .get_by_name("CharacterStatsPaneStat2Background")
            .is_some()
    );
    let top = |registry: &FrameRegistry, name: &str| {
        let frame = registry.get(registry.get_by_name(name).unwrap()).unwrap();
        let Val::Px(top) = frame.position.top else {
            panic!("{name} has no absolute top");
        };
        top
    };
    let category = top(&registry, "CharacterStatsPaneAttributesCategoryBackground");
    let item_level = top(&registry, "CharacterStatsPaneItemLevelCategoryBackground");
    // ItemLevelCategory 40 + ItemLevelFrame 29, then the category's 40 and -2.
    assert_eq!(category - item_level, 69.0);
    let first = top(&registry, "CharacterStatsPaneStat1Label");
    assert_eq!(first - category, 42.0);
    assert_eq!(top(&registry, "CharacterStatsPaneStat2Label") - first, 15.0);

    sheet.item_level = None;
    let low = build(sheet.clone());
    let first = top(&low, "CharacterStatsPaneStat1Label");
    // Below level 10 the stats are 5 further apart (`statYOffset = -5`).
    assert_eq!(top(&low, "CharacterStatsPaneStat2Label") - first, 20.0);

    sheet.attributes.clear();
    let empty = build(sheet);
    assert!(
        empty
            .get_by_name("CharacterStatsPaneAttributesCategoryTitle")
            .is_none()
    );
}

#[test]
fn a_specialization_shows_only_its_primary_stat() {
    let (stats, ratings) = warrior_sheet();
    let labels = |primary| {
        attribute_lines(Some((&stats, &ratings)), Some(primary))
            .iter()
            .map(|line| line.label)
            .collect::<Vec<_>>()
    };
    assert_eq!(
        labels(PrimaryStat::Strength),
        ["Strength:", "Stamina:", "Armor:"]
    );
    assert_eq!(
        labels(PrimaryStat::Agility),
        ["Agility:", "Stamina:", "Armor:"]
    );
    assert_eq!(
        labels(PrimaryStat::Intellect),
        ["Intellect:", "Stamina:", "Armor:"]
    );
}

/// The server's `DerivedStats` of a level 20 human Arcane mage in three caster cloth
/// pieces (game-server `level_20_arcane_mage_in_caster_cloth_derives_sheet_stats`).
fn arcane_mage_derived() -> DerivedStats {
    DerivedStats {
        spell_power: 66.0,
        attack_power: 0.0,
        crit_pct: 5.0 + 3.0 / 4.431_969_6,
        haste_pct: 8.0 / 4.239_275_5,
        mastery_pct: 10.56,
        versatility_pct: 6.0 / 5.202_747,
        ..DerivedStats::default()
    }
}

#[test]
fn enhancements_round_retail_percentages_and_hide_zero_stats() {
    let lines = enhancement_lines(Some(&arcane_mage_derived()));
    let shown: Vec<_> = lines
        .iter()
        .map(|line| (line.label, line.value.as_str()))
        .collect();
    // format("%d%%", value + 0.5): 5.68 -> 6, 1.89 -> 2, 10.56 -> 11, 1.15 -> 1.
    assert_eq!(
        shown,
        [
            ("Critical Strike:", "6%"),
            ("Haste:", "2%"),
            ("Mastery:", "11%"),
            ("Versatility:", "1%"),
        ]
    );
    // hideAt = 0: a level 1 character without gear has only its 5% base crit.
    let bare = DerivedStats {
        crit_pct: 5.0,
        ..DerivedStats::default()
    };
    let labels: Vec<_> = enhancement_lines(Some(&bare))
        .iter()
        .map(|line| line.label)
        .collect();
    assert_eq!(labels, ["Critical Strike:"]);
    assert!(enhancement_lines(None).is_empty());
}

/// Leech, Avoidance and Speed follow Versatility in `PAPERDOLL_STATCATEGORIES` order once
/// non-zero; the server's level 20 values for 12% raw leech, 5% avoidance and 30% raw
/// speed (game-server `tertiary_ratings_follow_curve_21025`).
#[test]
fn enhancements_show_nonzero_tertiary_stats_after_versatility() {
    let derived = DerivedStats {
        leech_pct: 11.6,
        avoidance_pct: 5.0,
        speed_pct: 21.0,
        ..arcane_mage_derived()
    };
    let shown: Vec<_> = enhancement_lines(Some(&derived))
        .iter()
        .map(|line| (line.label, line.value.clone()))
        .collect();
    assert_eq!(
        shown[3..],
        [
            ("Versatility:", "1%".to_string()),
            ("Leech:", "12%".to_string()),
            ("Avoidance:", "5%".to_string()),
            ("Speed:", "21%".to_string()),
        ]
    );
}

#[test]
fn enhancements_category_follows_the_last_attribute_and_numbers_on() {
    let (stats, ratings) = warrior_sheet();
    let mut sheet = view(&InventoryState::default(), None);
    sheet.attributes = attribute_lines(Some((&stats, &ratings)), Some(PrimaryStat::Intellect));
    sheet.enhancements = enhancement_lines(Some(&arcane_mage_derived()));
    let registry = build(sheet.clone());
    assert_eq!(
        text(&registry, "CharacterStatsPaneEnhancementsCategoryTitle"),
        "Enhancements"
    );
    // Intellect, Stamina, Armor, then the four enhancements from Stat4.
    assert_eq!(text(&registry, "CharacterStatsPaneStat3Label"), "Armor:");
    assert_eq!(
        text(&registry, "CharacterStatsPaneStat4Label"),
        "Critical Strike:"
    );
    assert_eq!(text(&registry, "CharacterStatsPaneStat4Value"), "6%");
    assert_eq!(
        text(&registry, "CharacterStatsPaneStat7Label"),
        "Versatility:"
    );
    let top = |registry: &FrameRegistry, name: &str| {
        let frame = registry.get(registry.get_by_name(name).unwrap()).unwrap();
        let Val::Px(top) = frame.position.top else {
            panic!("{name} has no absolute top");
        };
        top
    };
    // The category sits right under the last attribute (categoryYOffset 0), its first
    // stat 2 below its 40-tall title.
    let armor = top(&registry, "CharacterStatsPaneStat3Label");
    let category = top(
        &registry,
        "CharacterStatsPaneEnhancementsCategoryBackground",
    );
    assert_eq!(category - armor, 15.0);
    assert_eq!(
        top(&registry, "CharacterStatsPaneStat4Label") - category,
        42.0
    );

    // Below level 10: categoryYOffset -11 and statYOffset -5.
    sheet.item_level = None;
    let low = build(sheet.clone());
    let armor = top(&low, "CharacterStatsPaneStat3Label");
    let category = top(&low, "CharacterStatsPaneEnhancementsCategoryBackground");
    assert_eq!(category - armor, 15.0 + 11.0);

    sheet.enhancements.clear();
    assert!(
        build(sheet)
            .get_by_name("CharacterStatsPaneEnhancementsCategoryTitle")
            .is_none()
    );
}
