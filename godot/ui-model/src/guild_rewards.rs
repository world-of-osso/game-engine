//! Mainline GuildRewards.xml/.lua: the Benefits tab's rewards list and guild reputation bar.
//! Rewards come from the server (`RequestGuildRewards` on show); standing and money are the
//! player's own replicated values.
use crate::{
    bank_art::{cropped, label},
    guild_ranks::{CommunitiesTab, GuildRanksSession},
    merchant_frame_component::{MoneyAlign, money_colored, money_width},
    minimal_scroll_bar::{MinimalScrollBar, Unscrollable, pixel_geometry, scroll_list_attr},
    quest_art::DynName,
};
use shared::protocol::{GuildRewardItem, GuildRewardsRequest, GuildRewardsState};
use shared::reputation::{Standing, standing_for_value, tier_progress};
use ui_toolkit::{rsx, screen::SharedContext, widget_def::Element};

pub const SCROLL_BOX: &str = "CommunitiesGuildRewardsScrollBox";
pub const ROW_PREFIX: &str = "CommunitiesGuildReward";
pub const GUILD_FACTION: u32 = 1168;
const ROW_HEIGHT: f32 = 47.0; // COMMUNITIES_GUILD_REWARDS_BUTTON_HEIGHT
const ROW_WIDTH: f32 = 293.0; // ScrollBox TOPLEFT 5,-6 / BOTTOMRIGHT -4,0 in 302.
const VIEW_HEIGHT: f32 = 327.0;
const TRAINER_TEXTURES: u32 = 404984;
const GUILD_FRAME: u32 = 410251;
const ACHIEVEMENT_GUILD: u32 = 452110;
const STATUS_BAR: u32 = 137012;
// SmallMoneyFrame's CopperButton RIGHT is 13px inside the frame (MoneyFrame.lua:378-380).
const MONEY_RIGHT_INSET: f32 = 13.0;
const NORMAL: &str = "1,0.82,0,1";
const GREY: &str = "0.5,0.5,0.5,1";
const WHITE: &str = "1,1,1,1";
const RED: &str = "1,0.1254902,0.1254902,1";
const YELLOW: &str = "1,1,0,1";

/// `FACTION_STANDING_LABEL1`..`8` (GlobalStrings), by `Standing` order.
pub const STANDING_LABELS: [&str; 8] = [
    "Hated",
    "Hostile",
    "Unfriendly",
    "Neutral",
    "Friendly",
    "Honored",
    "Revered",
    "Exalted",
];

#[derive(Clone, Debug, Default, PartialEq)]
pub struct GuildRewardsSession {
    pub state: Option<GuildRewardsState>,
    pub waiting: bool,
    /// Faction 1168 value from the player's reputation snapshot (`GetGuildFactionData`).
    pub reputation: Option<i32>,
    /// `GetMoney`.
    pub money: u64,
}

impl GuildRewardsSession {
    pub fn query(&mut self) -> Option<GuildRewardsRequest> {
        if self.waiting {
            return None;
        }
        self.waiting = true;
        Some(GuildRewardsRequest)
    }

    pub fn apply(&mut self, state: GuildRewardsState) {
        self.waiting = false;
        self.state = (state.error.is_none()).then_some(state);
    }

    pub fn standing(&self) -> Standing {
        standing_for_value(self.reputation.unwrap_or(0))
    }

    pub fn reward(&self, item_id: u32) -> Option<&GuildRewardItem> {
        self.state
            .as_ref()?
            .rewards
            .iter()
            .find(|reward| reward.item_id == item_id)
    }
}

pub fn standing_label(standing: Standing) -> &'static str {
    STANDING_LABELS[standing as usize]
}

pub fn frame(session: &GuildRanksSession, shared: &SharedContext) -> Element {
    if session.communities_tab != CommunitiesTab::Benefits || session.profile.is_none() {
        return vec![];
    }
    let mut out = rewards_frame(&session.rewards, shared);
    out.extend(faction_frame(&session.rewards));
    out
}

/// GuildBenefitsFrameTemplate: Rewards TOPRIGHT -15 in the 608-wide frame at 194/63.
fn rewards_frame(rewards: &GuildRewardsSession, shared: &SharedContext) -> Element {
    let mut children = cropped(
        "CommunitiesGuildRewardsBackground".into(),
        TRAINER_TEXTURES,
        "0,0.587890625,0,0.65625",
        (1.0, 1.0, 300.0, 333.0),
    );
    children.extend(label(
        "CommunitiesGuildRewardsTitle".into(),
        "Guild Rewards",
        (0.0, -35.0, 232.0, 38.0),
        (14.0, NORMAL, "LEFT"),
    ));
    let items = rewards
        .state
        .as_ref()
        .map_or(&[][..], |state| state.rewards.as_slice());
    children.extend(reward_list(items, rewards, shared));
    rsx! { r#frame { name: "CommunitiesGuildRewardsFrame", left: 485.0, top: 63.0,
    width: 302.0, height: 333.0, pos_type: "absolute", {children} } }
}

fn reward_list(
    items: &[GuildRewardItem],
    rewards: &GuildRewardsSession,
    shared: &SharedContext,
) -> Element {
    const BAR_HEIGHT: f32 = 327.0; // ScrollBar TOPRIGHT+3,-5 / BOTTOMRIGHT+3,+1 in 333.
    let height = items.len() as f32 * ROW_HEIGHT;
    let geometry = pixel_geometry(VIEW_HEIGHT, height, BAR_HEIGHT);
    let offset = geometry.clamp(shared.scroll_first_row(SCROLL_BOX));
    let config = scroll_list_attr(&geometry);
    let rows: Element = items
        .iter()
        .enumerate()
        .flat_map(|(index, item)| reward_row(item, rewards, index as f32 * ROW_HEIGHT))
        .collect();
    let mut out = rsx! { r#frame { name: {DynName(SCROLL_BOX.into())}, left: 5.0, top: 6.0,
    width: ROW_WIDTH, height: VIEW_HEIGHT, pos_type: "absolute", scroll_list: config, mouse_enabled: true,
        r#frame { name: "CommunitiesGuildRewardsScrollChild", left: 0.0, top: {-(offset as f32)},
        width: ROW_WIDTH, height, pos_type: "absolute", {rows} } } };
    out.extend(
        MinimalScrollBar {
            list: SCROLL_BOX,
            left: 305.0,
            top: 5.0,
            height: BAR_HEIGHT,
            geometry,
            offset,
            unscrollable: Unscrollable::HideBar,
        }
        .element(),
    );
    out
}

/// CommunitiesGuildRewardsButtonMixin:Init.
fn reward_row(item: &GuildRewardItem, rewards: &GuildRewardsSession, top: f32) -> Element {
    let name = format!("{ROW_PREFIX}{}", item.item_id);
    let catalog = crate::item_catalog::item_catalog_entry(item.item_id);
    let item_name = catalog.map_or("Unknown", |entry| entry.name.as_str());
    let icon = catalog.map_or(0, |entry| entry.icon_fdid);
    let achievement = item.achievements.first();
    let standing_short = rewards.standing() < item.min_standing;
    let mut art = cropped(
        format!("{name}Normal"),
        TRAINER_TEXTURES,
        "0.00195313,0.57421875,0.65820313,0.75",
        (0.0, 0.0, ROW_WIDTH, ROW_HEIGHT),
    );
    if achievement.is_some() {
        // Black alpha .45 yields the displayed RGB of Retail's DisabledBG MOD(.55).
        art.extend(
            rsx! { r#frame { name: {DynName(format!("{name}DisabledBG"))},
            width: {ROW_WIDTH - 4.0}, height: {ROW_HEIGHT - 4.0}, left: 2.0, top: 2.0,
            pos_type: "absolute", background_color: "0,0,0,0.45" } },
        );
    }
    let icon_color = if achievement.is_none() && standing_short {
        "1,0,0,1"
    } else {
        WHITE
    };
    let desaturated = achievement.is_some();
    art.extend(
        rsx! { texture { name: {DynName(format!("{name}Icon"))}, texture_fdid: icon,
        vertex_color: icon_color, desaturated, left: 6.0, top: 6.0, width: 36.0, height: 36.0,
        pos_type: "absolute" } },
    );
    if achievement.is_some() {
        art.extend(cropped(
            format!("{name}Lock"),
            GUILD_FRAME,
            "0.51660156,0.53320313,0.92578125,0.96679688",
            (25.0, 21.0, 17.0, 21.0),
        ));
    }
    let cost_width = if item.cost > 0 {
        money_width(item.cost) + MONEY_RIGHT_INSET
    } else {
        0.0
    };
    let money_left = ROW_WIDTH + 5.0 - cost_width;
    art.extend(label(
        format!("{name}Name"),
        item_name,
        (48.0, 7.0, money_left - 2.0 - 48.0, 12.0),
        (
            12.0,
            if achievement.is_some() { GREY } else { NORMAL },
            "LEFT",
        ),
    ));
    art.extend(subtext(&name, item, achievement, standing_short));
    if item.cost > 0 {
        let color = if rewards.money >= item.cost {
            WHITE
        } else {
            RED
        };
        let coins = money_colored(
            &format!("{name}Money"),
            item.cost,
            (cost_width - MONEY_RIGHT_INSET, 14.0),
            MoneyAlign::Right,
            color,
        );
        art.extend(rsx! { r#frame { name: {DynName(format!("{name}MoneyFrame"))},
        width: cost_width, height: 13.0, left: money_left, top: 7.0, pos_type: "absolute", {coins} } });
    }
    rsx! { button { name: {DynName(name)}, left: 0.0, top, width: ROW_WIDTH, height: ROW_HEIGHT,
    pos_type: "absolute", button_default_skin: false, tooltip_provider: "guild_reward",
    button_fdid_highlight: TRAINER_TEXTURES,
    button_highlight_tex_coords: "0.00195313,0.57421875,0.75390625,0.84570313", {art} } }
}

/// SubText: SystemFont_Shadow_Small, LEFT 48,-8, width 240.
fn subtext(
    name: &str,
    item: &GuildRewardItem,
    achievement: Option<&shared::protocol::GuildRewardAchievement>,
    standing_short: bool,
) -> Element {
    const TOP: f32 = 25.5;
    const SIZE: f32 = 10.0;
    if let Some(achievement) = achievement {
        // REQUIRES_LABEL .. COMMUNITIES_GUILD_REWARDS_ACHIEVEMENT_ICON (18x16) .. yellow name.
        let prefix = "Requires: ";
        let (prefix_width, _) = ui_toolkit::text_measure::measure_text(
            prefix,
            ui_toolkit::widgets::font_string::GameFont::FrizQuadrata,
            SIZE,
        )
        .expect("source reward font");
        let mut out = label(
            format!("{name}SubText"),
            prefix,
            (48.0, TOP, prefix_width, 12.0),
            (SIZE, WHITE, "LEFT"),
        );
        out.extend(cropped(
            format!("{name}SubTextIcon"),
            ACHIEVEMENT_GUILD,
            "0.6328125,0.671875,0.130859375,0.166015625",
            (48.0 + prefix_width, TOP - 2.0, 18.0, 16.0),
        ));
        out.extend(label(
            format!("{name}SubTextAchievement"),
            &achievement.name,
            (
                48.0 + prefix_width + 18.0,
                TOP,
                240.0 - prefix_width - 18.0,
                12.0,
            ),
            (SIZE, YELLOW, "LEFT"),
        ));
        return out;
    }
    if !standing_short {
        return vec![];
    }
    // REQUIRES_GUILD_FACTION "Requires: |cffff0000%s|r".
    let prefix = "Requires: ";
    let (prefix_width, _) = ui_toolkit::text_measure::measure_text(
        prefix,
        ui_toolkit::widgets::font_string::GameFont::FrizQuadrata,
        SIZE,
    )
    .expect("source reward font");
    let mut out = label(
        format!("{name}SubText"),
        prefix,
        (48.0, TOP, prefix_width, 12.0),
        (SIZE, WHITE, "LEFT"),
    );
    out.extend(label(
        format!("{name}SubTextStanding"),
        standing_label(item.min_standing),
        (48.0 + prefix_width, TOP, 240.0 - prefix_width, 12.0),
        (SIZE, "1,0,0,1", "LEFT"),
    ));
    out
}

/// CommunitiesGuildRewardsButton_OnEnter: the item, then its unmet requirements in red.
pub fn reward_tooltip(
    reward: &GuildRewardItem,
    standing: Standing,
) -> Option<crate::game_tooltip::GameTooltip> {
    use crate::tooltip_presentation::TooltipLineState;
    const RED_LINE: [f32; 4] = [1.0, 0.0, 0.0, 1.0];
    let item = crate::item_catalog::item_catalog_entry(reward.item_id)?;
    let slot = crate::game_tooltip::item::named_item(reward.item_id, &item.name, item.quality, 1);
    let mut tooltip = crate::game_tooltip::item::item_game_tooltip(&slot, None);
    let lines = &mut tooltip.content.lines;
    let colored = |text: &str, color| TooltipLineState {
        left_color: color,
        ..TooltipLineState::new(text)
    };
    if let Some(achievement) = reward.achievements.first() {
        lines.push(colored(" ", RED_LINE));
        lines.push(colored("Requires guild achievement", RED_LINE));
        lines.push(colored(&achievement.name, [1.0, 1.0, 0.0, 1.0]));
    }
    if reward.min_standing > standing {
        lines.push(colored(" ", RED_LINE));
        lines.push(colored(
            &format!(
                "Requires {} with guild",
                standing_label(reward.min_standing)
            ),
            RED_LINE,
        ));
    }
    if !reward.achievements.is_empty() {
        lines.push(colored(" ", RED_LINE));
        lines.push(colored(
            "<Ctrl-Click to view requirements>",
            [0.0, 1.0, 0.0, 1.0],
        ));
    }
    Some(tooltip)
}

/// CommunitiesGuildFactionBarMixin:OnEnter: GUILD_REPUTATION, the faction description and
/// GUILD_EXPERIENCE_CURRENT within the current standing.
pub fn reputation_tooltip(value: i32, description: &str) -> crate::game_tooltip::GameTooltip {
    use crate::tooltip_presentation::{TooltipLineState, TooltipPresentation, description_lines};
    let (current, maximum) = if standing_for_value(value) == Standing::Exalted {
        (1, 1)
    } else {
        tier_progress(value)
    };
    let percent = (current as f64 / maximum.max(1) as f64 * 100.0).ceil() as i64;
    let mut lines = description_lines(description, [1.0; 4]);
    lines.push(TooltipLineState::new(format!(
        "Current: {}/{} ({percent}%)",
        crate::character_frame::break_up_large_numbers(current as i64),
        crate::character_frame::break_up_large_numbers(maximum as i64),
    )));
    crate::game_tooltip::GameTooltip::new(
        TooltipPresentation {
            title: "Guild Reputation".into(),
            lines,
            ..Default::default()
        },
        None,
    )
}

/// GuildBenefitsFrameTemplate FactionFrame (BOTTOMLEFT 0,-25) and its 184x20 Bar
/// (CommunitiesGuildProgressBarTemplate, CommunitiesGuildFactionBarMixin:UpdateFaction).
fn faction_frame(rewards: &GuildRewardsSession) -> Element {
    const LEFT: f32 = 194.0;
    const LABEL_TOP: f32 = 406.5;
    const SIZE: f32 = 10.0;
    let caption = "Guild Reputation:";
    let (caption_width, _) = ui_toolkit::text_measure::measure_text(
        caption,
        ui_toolkit::widgets::font_string::GameFont::FrizQuadrata,
        SIZE,
    )
    .expect("source reputation font");
    let mut out = label(
        "CommunitiesGuildFactionLabel".into(),
        caption,
        (LEFT, LABEL_TOP, caption_width, 10.0),
        (SIZE, NORMAL, "LEFT"),
    );
    let value = rewards.reputation.unwrap_or(0);
    let standing = standing_for_value(value);
    let (current, maximum) = if standing == Standing::Exalted {
        (1, 1)
    } else {
        tier_progress(value)
    };
    const BAR_WIDTH: f32 = 184.0;
    let max_bar = BAR_WIDTH - 4.0;
    let progress = (max_bar * current as f32 / maximum.max(1) as f32).min(max_bar);
    let mut bar = cropped(
        "CommunitiesGuildFactionBarBG".into(),
        GUILD_FRAME,
        "0.39843750,0.41210938,0.96875000,0.99609375",
        (0.0, 4.0, BAR_WIDTH, 14.0),
    );
    bar.extend(rsx! { texture { name: "CommunitiesGuildFactionBarProgress", texture_fdid: STATUS_BAR,
    vertex_color: "0.224,0.396,0.839,1", left: 1.0, top: 4.0, width: {progress + 1.0}, height: 14.0,
    pos_type: "absolute" } });
    if progress <= max_bar - 4.0 {
        bar.extend(cropped(
            "CommunitiesGuildFactionBarShadow".into(),
            GUILD_FRAME,
            "0.41601563,0.42480469,0.88671875,0.91406250",
            (progress + 2.0, 4.0, 9.0, 14.0),
        ));
    }
    bar.extend(cropped(
        "CommunitiesGuildFactionBarLeft".into(),
        GUILD_FRAME,
        "0.60742188,0.62500000,0.78710938,0.82226563",
        (0.0, 2.0, 18.0, 18.0),
    ));
    bar.extend(cropped(
        "CommunitiesGuildFactionBarMiddle".into(),
        GUILD_FRAME,
        "0.60742188,0.62500000,0.74804688,0.78320313",
        (18.0, 2.0, BAR_WIDTH - 36.0, 18.0),
    ));
    bar.extend(cropped(
        "CommunitiesGuildFactionBarRight".into(),
        GUILD_FRAME,
        "0.60742188,0.62500000,0.82617188,0.86132813",
        (BAR_WIDTH - 18.0, 2.0, 18.0, 18.0),
    ));
    bar.extend(label(
        "CommunitiesGuildFactionBarLabel".into(),
        standing_label(standing),
        (0.0, 6.0, BAR_WIDTH, 10.0),
        (SIZE, WHITE, "CENTER"),
    ));
    out.extend(
        rsx! { r#frame { name: "CommunitiesGuildFactionBar", left: {LEFT + caption_width + 4.0},
        top: {LABEL_TOP - 6.5}, width: BAR_WIDTH, height: 20.0, pos_type: "absolute",
        mouse_enabled: true, tooltip_provider: "guild_reputation", {bar} } },
    );
    out
}
