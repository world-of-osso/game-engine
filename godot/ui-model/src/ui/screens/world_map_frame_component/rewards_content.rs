//! MapQuestInfoRewardsFrame uses SmallQuestRewardItemButtonTemplate, not giver rewards.
//! QuestInfo.xml:609-724; SmallItemButtonTemplate: 134x30, 30px icon, name x36.
use crate::quest_art::{DynName, named_atlas_texture, wrapped_text_height};
use crate::quest_frame_component::{RewardItemView, RewardView};
use ui_toolkit::{rsx, widget_def::Element, widgets::font_string::GameFont};

const WIDTH: f32 = 289.0;
const ITEM_WIDTH: f32 = 134.0;
const ITEM_HEIGHT: f32 = 30.0;
const COLUMN_STRIDE: f32 = ITEM_WIDTH + 8.0;
const SECTION_GAP: f32 = 5.0;
const ROW_GAP: f32 = 2.0;
const REWARD_TEXT: &str = "0.902,0.788,0.671,1.0";
const UNKNOWN_ITEM_ICON: u32 = 134_400;
const COIN_ICON: u32 = 133_784; // interface/icons/inv_misc_coin_01.blp

pub(super) struct Content {
    pub elements: Element,
    pub height: f32,
    pub rows: usize,
}

pub(super) fn build(rewards: &RewardView) -> Content {
    let mut content = Content {
        elements: Vec::new(),
        height: 1.0,
        rows: 0,
    };
    let choices = if rewards.choices.len() == 1 {
        "You will receive:"
    } else {
        "You will be able to choose one of these rewards:"
    };
    append_items(&mut content, &rewards.choices, 0, choices);
    let receive = if rewards.choices.is_empty() {
        "You will receive:"
    } else {
        "You will also receive:"
    };
    append_items(&mut content, &rewards.items, rewards.choices.len(), receive);
    if rewards.money > 0 {
        content.height += SECTION_GAP;
        let money = RewardItemView {
            name: crate::quest_runtime::money_text(rewards.money),
            count: 1,
            icon_fdid: Some(COIN_ICON),
        };
        content.elements.extend(small_item(
            "QuestInfoMoney",
            "QuestInfoMoneyText",
            &money,
            [0.0, content.height],
        ));
        content.height += ITEM_HEIGHT;
        content.rows += 1;
    }
    content
}

fn append_items(content: &mut Content, items: &[RewardItemView], first: usize, heading: &str) {
    if items.is_empty() {
        return;
    }
    content.height += SECTION_GAP;
    let height = wrapped_text_height(heading, WIDTH, 12.0);
    content.elements.extend(rsx! {
        fontstring { width: WIDTH, height, text: heading, font: GameFont::FrizQuadrata,
            font_size: 12.0, font_color: REWARD_TEXT, justify_h: "LEFT",
            pos_type: "absolute", left: 0.0, top: {content.height} }
    });
    content.height += height + SECTION_GAP;
    for (index, item) in items.iter().enumerate() {
        let name = format!("QuestInfoRewardsFrameQuestInfoItem{}", first + index + 1);
        let x = (index % 2) as f32 * COLUMN_STRIDE;
        let y = content.height + (index / 2) as f32 * (ITEM_HEIGHT + ROW_GAP);
        content
            .elements
            .extend(small_item(&name, &format!("{name}Name"), item, [x, y]));
    }
    let rows = items.len().div_ceil(2);
    content.height += rows as f32 * ITEM_HEIGHT + rows.saturating_sub(1) as f32 * ROW_GAP;
    content.rows += rows;
}

fn small_item(name: &str, label_name: &str, item: &RewardItemView, [x, y]: [f32; 2]) -> Element {
    let text = &item.name;
    let icon = item.icon_fdid.unwrap_or(UNKNOWN_ITEM_ICON);
    let count = item.count;
    let border =
        ui_toolkit::atlas::resolve_region("QuestItemBorder", ui_toolkit::atlas::thread_skin())
            .expect("small reward name border");
    let art = named_atlas_texture(
        format!("{name}NameFrame"),
        "QuestItemBorder",
        (
            32.0,
            (ITEM_HEIGHT - border.height) / 2.0,
            border.width,
            border.height,
        ),
    );
    let hide_count = count <= 1;
    let count_text = count.to_string();
    rsx! {
        r#frame { name: {DynName(name.into())}, width: ITEM_WIDTH, height: ITEM_HEIGHT,
            mouse_enabled: true, pos_type: "absolute", left: x, top: y,
            {art}
            texture { name: {DynName(format!("{name}IconTexture"))}, width: ITEM_HEIGHT, height: ITEM_HEIGHT,
                texture_fdid: icon, pos_type: "absolute", left: 0.0, top: 0.0 }
            fontstring { name: {DynName(label_name.into())}, width: 92.0, height: 36.0, text,
                font: GameFont::FrizQuadrata, font_size: 12.0, font_color: "1,1,1,1", justify_h: "LEFT",
                pos_type: "absolute", left: 36.0, top: -3.0 }
            fontstring { name: {DynName(format!("{name}Count"))}, width: ITEM_HEIGHT, height: 12.0,
                text: count_text, hidden: hide_count, font: GameFont::FrizQuadrata, font_size: 12.0,
                font_color: "1,1,1,1", justify_h: "RIGHT", pos_type: "absolute", left: 0.0, top: 17.0 }
        }
    }
}
