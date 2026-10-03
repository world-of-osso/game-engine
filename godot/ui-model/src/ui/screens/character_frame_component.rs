use std::fmt;

use shared::protocol::EquipmentSlot;
use ui_toolkit::rsx;
use ui_toolkit::screen::SharedContext;
use ui_toolkit::widget_def::Element;

use crate::ui::strata::FrameStrata;

struct DynName(String);

impl fmt::Display for DynName {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

pub const FRAME_W: f32 = 336.0;
pub const FRAME_H: f32 = 424.0;
const SLOT_SIZE: f32 = 36.0;
const SLOT_GAP: f32 = 4.0;
const COLUMN_INSET: f32 = 12.0;
const HEADER_H: f32 = 30.0;
const STATS_H: f32 = 60.0;

const FRAME_BG: &str = "0.06,0.05,0.04,0.92";
const TITLE_COLOR: &str = "1.0,0.82,0.0,1.0";
const INFO_COLOR: &str = "1.0,1.0,1.0,1.0";
const SLOT_LABEL_COLOR: &str = "0.7,0.7,0.7,1.0";
const ITEM_COLOR: &str = "1.0,0.82,0.0,0.9";
const SLOT_BG: &str = "0.0,0.0,0.0,0.6";
const STAT_LABEL_COLOR: &str = "0.8,0.8,0.8,1.0";
const STAT_VALUE_COLOR: &str = "1.0,1.0,1.0,1.0";

/// Left column WoW slot names (8 slots).
pub const LEFT_SLOT_LABELS: [&str; 8] = [
    "Head",
    "Neck",
    "Shoulders",
    "Back",
    "Chest",
    "Shirt",
    "Tabard",
    "Wrists",
];

/// Right column WoW slot names (8 slots).
pub const RIGHT_SLOT_LABELS: [&str; 8] = [
    "Hands", "Waist", "Legs", "Feet", "Finger", "Finger", "Trinket", "Trinket",
];

/// Bottom row WoW slot names.
pub const BOTTOM_SLOT_LABELS: [&str; 2] = ["Main Hand", "Off Hand"];

/// The equipment slot of each label, column by column.
pub const LEFT_SLOTS: [EquipmentSlot; 8] = [
    EquipmentSlot::Head,
    EquipmentSlot::Neck,
    EquipmentSlot::Shoulder,
    EquipmentSlot::Back,
    EquipmentSlot::Chest,
    EquipmentSlot::Shirt,
    EquipmentSlot::Tabard,
    EquipmentSlot::Wrist,
];
pub const RIGHT_SLOTS: [EquipmentSlot; 8] = [
    EquipmentSlot::Hands,
    EquipmentSlot::Waist,
    EquipmentSlot::Legs,
    EquipmentSlot::Feet,
    EquipmentSlot::Finger1,
    EquipmentSlot::Finger2,
    EquipmentSlot::Trinket1,
    EquipmentSlot::Trinket2,
];
pub const BOTTOM_SLOTS: [EquipmentSlot; 2] = [EquipmentSlot::MainHand, EquipmentSlot::OffHand];

/// `equipment_slot:<n>` on each paperdoll slot, `n` its index in
/// [`EquipmentSlot::ALL`] (`INVSLOT_*` order).
pub const ACTION_EQUIPMENT_SLOT_PREFIX: &str = "equipment_slot:";

pub fn equipment_slot_action(slot: EquipmentSlot) -> String {
    let index = EquipmentSlot::ALL
        .iter()
        .position(|candidate| *candidate == slot)
        .expect("EquipmentSlot::ALL lists every slot");
    format!("{ACTION_EQUIPMENT_SLOT_PREFIX}{index}")
}

pub fn parse_equipment_slot_action(action: &str) -> Option<EquipmentSlot> {
    let index: usize = action
        .strip_prefix(ACTION_EQUIPMENT_SLOT_PREFIX)?
        .parse()
        .ok()?;
    EquipmentSlot::ALL.get(index).copied()
}

#[derive(Clone, Debug, PartialEq)]
pub struct EquipmentSlotState {
    pub slot_name: String,
    pub item_name: String,
    /// The equipped item's icon, 0 when the slot is empty.
    pub icon_fdid: u32,
    /// Click action (`equipment_slot:<n>`); empty for a slot that takes no clicks.
    pub action: String,
    /// On the cursor: the icon is dimmed.
    pub locked: bool,
}

impl EquipmentSlotState {
    pub fn empty(slot_name: &str) -> Self {
        Self {
            slot_name: slot_name.to_string(),
            item_name: String::new(),
            icon_fdid: 0,
            action: String::new(),
            locked: false,
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct CharacterFrameState {
    pub visible: bool,
    pub character_name: String,
    pub level: u16,
    pub class_name: String,
    pub health: String,
    pub mana: String,
    pub speed: String,
    /// Head, Neck, Shoulders, Back, Chest, Shirt, Tabard, Wrists
    pub left_slots: Vec<EquipmentSlotState>,
    /// Hands, Waist, Legs, Feet, Finger, Finger, Trinket, Trinket
    pub right_slots: Vec<EquipmentSlotState>,
    /// Main Hand, Off Hand
    pub bottom_slots: Vec<EquipmentSlotState>,
}

impl Default for CharacterFrameState {
    fn default() -> Self {
        Self {
            visible: false,
            character_name: String::new(),
            level: 0,
            class_name: String::new(),
            health: String::new(),
            mana: String::new(),
            speed: String::new(),
            left_slots: LEFT_SLOT_LABELS
                .iter()
                .map(|s| EquipmentSlotState::empty(s))
                .collect(),
            right_slots: RIGHT_SLOT_LABELS
                .iter()
                .map(|s| EquipmentSlotState::empty(s))
                .collect(),
            bottom_slots: BOTTOM_SLOT_LABELS
                .iter()
                .map(|s| EquipmentSlotState::empty(s))
                .collect(),
        }
    }
}

pub fn character_frame_screen(ctx: &SharedContext) -> Element {
    let state = ctx
        .get::<CharacterFrameState>()
        .expect("CharacterFrameState must be in SharedContext");
    let hide = !state.visible;
    rsx! {
        r#frame {
            name: "CharacterFrame",
            width: {FRAME_W},
            height: {FRAME_H},
            strata: FrameStrata::Dialog,
            hidden: hide,
            background_color: FRAME_BG,
            pos_type: "absolute",
            left: 20.0,
            top: 80.0,
            {character_title_bar()}
            {character_center_info(state)}
            {left_slot_column(&state.left_slots)}
            {right_slot_column(&state.right_slots)}
            {bottom_slot_row(&state.bottom_slots)}
            {stats_area(state)}
        }
    }
}

fn character_title_bar() -> Element {
    rsx! {
        fontstring {
            name: "CharacterFrameTitle",
            width: {FRAME_W},
            height: {HEADER_H},
            text: "Character",
            font_size: 16.0,
            font_color: TITLE_COLOR,
            justify_h: "CENTER",
            pos_type: "absolute",
            left: "50%",
            translate_x: "-50%",
            top: -0.0,
        }
    }
}

fn character_center_info(state: &CharacterFrameState) -> Element {
    let level_class = if state.level > 0 {
        format!("Level {} {}", state.level, state.class_name)
    } else {
        state.class_name.clone()
    };
    let center_x = COLUMN_INSET + SLOT_SIZE + SLOT_GAP;
    let center_w = FRAME_W - 2.0 * (COLUMN_INSET + SLOT_SIZE + SLOT_GAP);
    rsx! {
        r#frame {
            name: "CharacterFrameInfo",
            width: {center_w},
            height: 60.0,
            pos_type: "absolute",
            left: {center_x},
            top: {-(-(HEADER_H + SLOT_GAP))},
            {center_name_label(&state.character_name, center_w)}
            {center_level_class_label(&level_class, center_w)}
        }
    }
}

fn center_name_label(name: &str, w: f32) -> Element {
    rsx! {
        fontstring {
            name: "CharacterFrameName",
            width: {w},
            height: 18.0,
            text: name,
            font_size: 14.0,
            font_color: INFO_COLOR,
            justify_h: "CENTER",
            pos_type: "absolute",
            left: "50%",
            translate_x: "-50%",
            top: -0.0,
        }
    }
}

fn center_level_class_label(text: &str, w: f32) -> Element {
    rsx! {
        fontstring {
            name: "CharacterFrameLevelClass",
            width: {w},
            height: 14.0,
            text: text,
            font_size: 11.0,
            font_color: SLOT_LABEL_COLOR,
            justify_h: "CENTER",
            pos_type: "absolute",
            left: "50%",
            translate_x: "-50%",
            top: 20.0,
        }
    }
}

fn equipment_slot(slot_id: DynName, slot: &EquipmentSlotState, x: f32, y: f32) -> Element {
    let label_id = DynName(format!("{}Label", slot_id.0));
    let item_id = DynName(format!("{}Item", slot_id.0));
    let icon = slot_icon(DynName(format!("{}Icon", slot_id.0)), slot);
    rsx! {
        r#frame {
            name: slot_id,
            width: {SLOT_SIZE},
            height: {SLOT_SIZE},
            background_color: SLOT_BG,
            onclick: {slot.action.as_str()},
            mouse_enabled: true,
            pos_type: "absolute",
            left: {x},
            top: {-(y)},
            {icon}
            {slot_name_label(label_id, &slot.slot_name)}
            {slot_item_label(item_id, &slot.item_name)}
        }
    }
}

fn slot_icon(id: DynName, slot: &EquipmentSlotState) -> Element {
    if slot.icon_fdid == 0 {
        return Element::default();
    }
    let color = if slot.locked {
        crate::ui::screens::bag_frame_component::LOCKED_ICON
    } else {
        "1.0,1.0,1.0,1.0"
    };
    rsx! {
        texture {
            name: id,
            width: {SLOT_SIZE},
            height: {SLOT_SIZE},
            texture_fdid: {slot.icon_fdid},
            vertex_color: color,
            pos_type: "absolute",
            left: 0.0,
            top: 0.0,
        }
    }
}

fn slot_name_label(id: DynName, text: &str) -> Element {
    rsx! {
        fontstring {
            name: id,
            width: {SLOT_SIZE},
            height: 11.0,
            text: text,
            font_size: 8.0,
            font_color: SLOT_LABEL_COLOR,
            justify_h: "CENTER",
            pos_type: "absolute",
            left: "50%",
            translate_x: "-50%",
            top: 2.0,
        }
    }
}

fn slot_item_label(id: DynName, text: &str) -> Element {
    rsx! {
        fontstring {
            name: id,
            width: {SLOT_SIZE},
            height: 11.0,
            text: text,
            font_size: 7.0,
            font_color: ITEM_COLOR,
            justify_h: "CENTER",
            pos_type: "absolute",
            left: "50%",
            translate_x: "-50%",
            bottom: 2.0,
        }
    }
}

fn left_slot_column(slots: &[EquipmentSlotState]) -> Element {
    slots
        .iter()
        .enumerate()
        .flat_map(|(i, slot)| {
            let slot_id = DynName(format!("CharacterSlotLeft{i}"));
            let x = COLUMN_INSET;
            let y = -(HEADER_H + SLOT_GAP + i as f32 * (SLOT_SIZE + SLOT_GAP));
            equipment_slot(slot_id, slot, x, y)
        })
        .collect()
}

fn right_slot_column(slots: &[EquipmentSlotState]) -> Element {
    let col_x = FRAME_W - COLUMN_INSET - SLOT_SIZE;
    slots
        .iter()
        .enumerate()
        .flat_map(|(i, slot)| {
            let slot_id = DynName(format!("CharacterSlotRight{i}"));
            let x = col_x;
            let y = -(HEADER_H + SLOT_GAP + i as f32 * (SLOT_SIZE + SLOT_GAP));
            equipment_slot(slot_id, slot, x, y)
        })
        .collect()
}

fn bottom_slot_row(slots: &[EquipmentSlotState]) -> Element {
    let row_y = -(FRAME_H - STATS_H - SLOT_SIZE - SLOT_GAP);
    slots
        .iter()
        .enumerate()
        .flat_map(|(i, slot)| {
            let slot_id = DynName(format!("CharacterSlotBottom{i}"));
            let x = if i == 0 {
                COLUMN_INSET
            } else {
                FRAME_W - COLUMN_INSET - SLOT_SIZE
            };
            equipment_slot(slot_id, slot, x, row_y)
        })
        .collect()
}

fn stats_area(state: &CharacterFrameState) -> Element {
    let stats_y = -(FRAME_H - STATS_H);
    rsx! {
        r#frame {
            name: "CharacterFrameStats",
            width: {FRAME_W - 2.0 * COLUMN_INSET},
            height: {STATS_H},
            pos_type: "absolute",
            left: {COLUMN_INSET},
            top: {-(stats_y)},
            {stat_row("CharacterStatHealth", "Health:", &state.health, 0.0)}
            {stat_row("CharacterStatMana", "Mana:", &state.mana, 18.0)}
            {stat_row("CharacterStatSpeed", "Speed:", &state.speed, 36.0)}
        }
    }
}

fn stat_row(id: &str, label: &str, value: &str, y_offset: f32) -> Element {
    let stat_w = FRAME_W - 2.0 * COLUMN_INSET;
    let half = stat_w / 2.0;
    rsx! {
        {stat_label(DynName(format!("{id}Label")), label, half, y_offset, STAT_LABEL_COLOR, "LEFT", 0.0)}
        {stat_label(DynName(format!("{id}Value")), value, half, y_offset, STAT_VALUE_COLOR, "RIGHT", half)}
    }
}

fn stat_label(
    id: DynName,
    text: &str,
    w: f32,
    y_offset: f32,
    color: &str,
    justify: &str,
    x: f32,
) -> Element {
    rsx! {
        fontstring {
            name: id,
            width: {w},
            height: 14.0,
            text: text,
            font_size: 10.0,
            font_color: color,
            justify_h: justify,
            pos_type: "absolute",
            left: x,
            top: y_offset,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ui::screens::screen_test_helpers::fontstring_text;
    use ui_toolkit::registry::FrameRegistry;
    use ui_toolkit::screen::{Screen, SharedContext};

    fn make_test_state() -> CharacterFrameState {
        CharacterFrameState {
            visible: true,
            character_name: "Azerothia".to_string(),
            level: 60,
            class_name: "Paladin".to_string(),
            health: "5000 / 5000".to_string(),
            mana: "3000 / 3000".to_string(),
            speed: "100%".to_string(),
            left_slots: LEFT_SLOT_LABELS
                .iter()
                .map(|s| EquipmentSlotState::empty(s))
                .collect(),
            right_slots: RIGHT_SLOT_LABELS
                .iter()
                .map(|s| EquipmentSlotState::empty(s))
                .collect(),
            bottom_slots: BOTTOM_SLOT_LABELS
                .iter()
                .map(|s| EquipmentSlotState::empty(s))
                .collect(),
        }
    }

    #[test]
    fn character_frame_screen_builds_expected_frames() {
        let mut registry = FrameRegistry::new(1920.0, 1080.0);
        let mut shared = SharedContext::new();
        shared.insert(make_test_state());
        let mut screen = Screen::new(character_frame_screen);
        screen.sync(&shared, &mut registry);

        assert!(registry.get_by_name("CharacterFrame").is_some());
        assert!(registry.get_by_name("CharacterFrameTitle").is_some());
        assert!(registry.get_by_name("CharacterFrameInfo").is_some());
        assert!(registry.get_by_name("CharacterFrameName").is_some());
        assert!(registry.get_by_name("CharacterFrameLevelClass").is_some());
        assert!(registry.get_by_name("CharacterFrameStats").is_some());
    }

    #[test]
    fn character_frame_builds_all_slot_frames() {
        let mut registry = FrameRegistry::new(1920.0, 1080.0);
        let mut shared = SharedContext::new();
        shared.insert(make_test_state());
        Screen::new(character_frame_screen).sync(&shared, &mut registry);

        for i in 0..8 {
            assert!(
                registry
                    .get_by_name(&format!("CharacterSlotLeft{i}"))
                    .is_some(),
                "CharacterSlotLeft{i} missing"
            );
            assert!(
                registry
                    .get_by_name(&format!("CharacterSlotRight{i}"))
                    .is_some(),
                "CharacterSlotRight{i} missing"
            );
        }
        assert!(registry.get_by_name("CharacterSlotBottom0").is_some());
        assert!(registry.get_by_name("CharacterSlotBottom1").is_some());
    }

    #[test]
    fn character_frame_hidden_when_not_visible() {
        let mut registry = FrameRegistry::new(1920.0, 1080.0);
        let mut shared = SharedContext::new();
        let mut state = make_test_state();
        state.visible = false;
        shared.insert(state);
        Screen::new(character_frame_screen).sync(&shared, &mut registry);

        let frame_id = registry
            .get_by_name("CharacterFrame")
            .expect("CharacterFrame");
        let frame = registry.get(frame_id).expect("frame data");
        assert!(frame.hidden, "frame should be hidden when visible=false");
    }

    #[test]
    fn equipment_slots_show_item_names() {
        let mut registry = FrameRegistry::new(1920.0, 1080.0);
        let mut shared = SharedContext::new();
        let mut state = make_test_state();
        state.left_slots[0] = EquipmentSlotState {
            slot_name: "Head".to_string(),
            item_name: "Helm of Valor".to_string(),
            ..EquipmentSlotState::empty("")
        };
        state.right_slots[2] = EquipmentSlotState {
            slot_name: "Legs".to_string(),
            item_name: "Legplates of Might".to_string(),
            ..EquipmentSlotState::empty("")
        };
        state.bottom_slots[0] = EquipmentSlotState {
            slot_name: "Main Hand".to_string(),
            item_name: "Ashbringer".to_string(),
            ..EquipmentSlotState::empty("")
        };
        shared.insert(state);
        Screen::new(character_frame_screen).sync(&shared, &mut registry);

        assert_eq!(
            fontstring_text(&registry, "CharacterSlotLeft0Item"),
            "Helm of Valor"
        );
        assert_eq!(
            fontstring_text(&registry, "CharacterSlotRight2Item"),
            "Legplates of Might"
        );
        assert_eq!(
            fontstring_text(&registry, "CharacterSlotBottom0Item"),
            "Ashbringer"
        );
    }

    #[test]
    fn slot_labels_show_slot_names() {
        let mut registry = FrameRegistry::new(1920.0, 1080.0);
        let mut shared = SharedContext::new();
        shared.insert(make_test_state());
        Screen::new(character_frame_screen).sync(&shared, &mut registry);

        assert_eq!(
            fontstring_text(&registry, "CharacterSlotLeft0Label"),
            "Head"
        );
        assert_eq!(
            fontstring_text(&registry, "CharacterSlotLeft7Label"),
            "Wrists"
        );
        assert_eq!(
            fontstring_text(&registry, "CharacterSlotRight0Label"),
            "Hands"
        );
        assert_eq!(
            fontstring_text(&registry, "CharacterSlotBottom1Label"),
            "Off Hand"
        );
    }

    #[test]
    fn stat_rows_render_values() {
        let mut registry = FrameRegistry::new(1920.0, 1080.0);
        let mut shared = SharedContext::new();
        shared.insert(make_test_state());
        Screen::new(character_frame_screen).sync(&shared, &mut registry);

        assert_eq!(
            fontstring_text(&registry, "CharacterStatHealthLabel"),
            "Health:"
        );
        assert_eq!(
            fontstring_text(&registry, "CharacterStatHealthValue"),
            "5000 / 5000"
        );
        assert_eq!(
            fontstring_text(&registry, "CharacterStatManaLabel"),
            "Mana:"
        );
        assert_eq!(
            fontstring_text(&registry, "CharacterStatManaValue"),
            "3000 / 3000"
        );
        assert_eq!(
            fontstring_text(&registry, "CharacterStatSpeedLabel"),
            "Speed:"
        );
        assert_eq!(
            fontstring_text(&registry, "CharacterStatSpeedValue"),
            "100%"
        );
    }

    #[test]
    fn title_bar_and_character_info_text() {
        let mut registry = FrameRegistry::new(1920.0, 1080.0);
        let mut shared = SharedContext::new();
        shared.insert(make_test_state());
        Screen::new(character_frame_screen).sync(&shared, &mut registry);

        assert_eq!(
            fontstring_text(&registry, "CharacterFrameTitle"),
            "Character"
        );
        assert_eq!(
            fontstring_text(&registry, "CharacterFrameName"),
            "Azerothia"
        );
        assert_eq!(
            fontstring_text(&registry, "CharacterFrameLevelClass"),
            "Level 60 Paladin"
        );
    }

    #[test]
    fn paperdoll_slots_click_with_their_equipment_slot_and_show_the_item_icon() {
        let mut state = make_test_state();
        state.bottom_slots[0] = EquipmentSlotState {
            slot_name: "Main Hand".into(),
            item_name: "Worn Shortsword".into(),
            icon_fdid: 135_274,
            action: equipment_slot_action(EquipmentSlot::MainHand),
            locked: false,
        };
        let mut registry = FrameRegistry::new(1920.0, 1080.0);
        let mut shared = SharedContext::new();
        shared.insert(state);
        Screen::new(character_frame_screen).sync(&shared, &mut registry);

        let slot = registry
            .get(registry.get_by_name("CharacterSlotBottom0").unwrap())
            .unwrap();
        assert_eq!(
            slot.onclick
                .as_deref()
                .and_then(parse_equipment_slot_action),
            Some(EquipmentSlot::MainHand)
        );
        assert!(registry.get_by_name("CharacterSlotBottom0Icon").is_some());
        assert!(registry.get_by_name("CharacterSlotBottom1Icon").is_none());
        assert_eq!(parse_equipment_slot_action("equipment_slot:19"), None);
        assert_eq!(
            fontstring_text(&registry, "CharacterSlotBottom0Item"),
            "Worn Shortsword"
        );
    }
}
