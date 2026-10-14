//! QuestMapFrame.xml:720-763 and QuestMapFrame.lua:953-990: bottom reward clip,
//! with a same-height story spacer and a reveal driven by the story scroll range.
use ui_toolkit::widgets::font_string::GameFont;
use ui_toolkit::{rsx, screen::SharedContext, widget_def::Element};

use crate::quest_art::named_atlas_texture;
use crate::quest_frame_component::{Column, RewardView, reward_contents};
use crate::quest_log_frame_component::QUEST_LOG_DETAILS_SCROLL;
use crate::quest_scroll::QuestScrollExtent;

use super::DETAILS_H;

const WIDTH: f32 = 307.0;
const BOTTOM: f32 = 23.0;
const CONTENT_LEFT: f32 = 12.0;
const CONTENT_TOP: f32 = 52.0;
const CONTENT_WIDTH: f32 = 289.0;
const HEIGHT_PADDING: f32 = 62.0;
const EMPTY_HEIGHT: f32 = 59.0;
const MIN_REVEAL: f32 = 124.0;
const SCROLL_HEIGHT: f32 = 430.0;

pub(super) struct Rewards {
    pub height: f32,
    rows: usize,
    content: Element,
}

impl Rewards {
    pub fn new(rewards: Option<&RewardView>) -> Self {
        let Some(rewards) = rewards.filter(|rewards| !rewards.is_empty()) else {
            return Self {
                height: EMPTY_HEIGHT,
                rows: 0,
                content: Vec::new(),
            };
        };
        let mut content_height = 0.0;
        let content = reward_contents(
            rewards,
            false,
            Column {
                x: 0.0,
                width: CONTENT_WIDTH,
            },
            &mut content_height,
        );
        Self {
            height: content_height + HEIGHT_PADDING,
            rows: rewards.choices.len().div_ceil(2)
                + rewards.items.len().div_ceil(2)
                + usize::from(rewards.money > 0),
            content,
        }
    }

    fn visible_height(&self, ctx: &SharedContext) -> f32 {
        let content_height = ctx
            .get::<QuestScrollExtent>()
            .filter(|extent| extent.list == QUEST_LOG_DETAILS_SCROLL)
            .map_or(0.0, |extent| extent.height);
        let range = (content_height - SCROLL_HEIGHT).max(0.0);
        if range == 0.0 || self.rows <= 1 {
            return self.height;
        }
        let offset = (ctx.scroll_first_row(QUEST_LOG_DETAILS_SCROLL) as f32).min(range);
        (self.height - (range - offset)).max(MIN_REVEAL)
    }

    pub fn element(self, left: f32, top: f32, ctx: &SharedContext) -> Element {
        let visible_height = self.visible_height(ctx);
        let chrome = reward_chrome(self.height, self.rows > 0);
        let content = self.content;
        rsx! {
            r#frame { name: "QuestMapRewardsFrameContainer", width: WIDTH, height: visible_height,
                frame_level: 50.0, pos_type: "absolute", left,
                top: {top + DETAILS_H - BOTTOM - visible_height},
                r#frame { name: "QuestMapRewardsFrame", width: WIDTH, height: {self.height},
                    pos_type: "absolute", left: 0.0, top: 0.0,
                    {chrome}
                    r#frame { name: "QuestMapRewardContents", width: CONTENT_WIDTH,
                        height: {self.height - HEIGHT_PADDING}, pos_type: "absolute",
                        left: CONTENT_LEFT, top: CONTENT_TOP, {content} }
                }
            }
        }
    }
}

fn reward_chrome(height: f32, show_label: bool) -> Element {
    let skin = ui_toolkit::atlas::thread_skin();
    let region =
        |atlas| ui_toolkit::atlas::resolve_region(atlas, skin).expect("quest reward atlas");
    let top = region("questlog-reward-header-top");
    let bottom = region("questlog-reward-bottom");
    let mut art = named_atlas_texture(
        "QuestMapRewardTile".into(),
        "questlog-reward-tile-vertical",
        (0.0, top.height, WIDTH, (height - top.height).max(0.0)),
    );
    art.extend(named_atlas_texture(
        "QuestMapRewardBottom".into(),
        "questlog-reward-bottom",
        (0.0, height - bottom.height, bottom.width, bottom.height),
    ));
    art.extend(named_atlas_texture(
        "QuestMapRewardTop".into(),
        "questlog-reward-header-top",
        (0.0, 0.0, top.width, top.height),
    ));
    let hide_label = !show_label;
    art.extend(rsx! {
        fontstring { name: "QuestMapRewardsLabel", width: WIDTH, height: 18.0,
            text: "Rewards", font: GameFont::FrizQuadrata, font_size: 18.0,
            font_color: "0.902,0.788,0.671,1.0", justify_h: "CENTER",
            hidden: hide_label, pos_type: "absolute", left: 0.0, top: 22.0 }
    });
    art
}

pub(super) fn spacer(height: f32) -> Element {
    rsx! { r#frame { name: "QuestMapRewardSpacer", width: CONTENT_WIDTH, height } }
}
