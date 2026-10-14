//! Retail Mainline Journeys (Blizzard_Journeys.lua/.xml) over realm renown snapshots.
//! Renown cards and the reward track only; unsourced journey/encounter categories,
//! texture kits, claim state and Delves companions are not shown.
mod track;
use crate::adventure_guide::{scroll_in, text};
use crate::auction_mail::text as global_string;
use crate::bank_art::{WHITE, label, texture};
use crate::menu_primitives::{
    ContextMenu, ContextMenuItem, MenuElementKind, context_menu_with_rows, menu_element,
    menu_row_rect_with_heading,
};
use crate::quest_art::{DynName, NORMAL_FONT_COLOR, named_atlas_texture};
use shared::protocol::{MajorFactionSnapshot, RenownSnapshot, RenownUnlock};
use std::collections::BTreeSet;
use ui_toolkit::{rsx, screen::SharedContext, widget_def::Element};

/// Retail 12.1 client `LE_EXPANSION_LEVEL_CURRENT` (EXPANSION_NAME11 "Midnight").
pub const CURRENT_EXPANSION: u8 = 11;
/// Blizzard_Journeys.lua `MAX_REWARD_CARDS_TO_DISPLAY`.
const MAX_REWARD_CARDS: usize = 4;
const CARD: (f32, f32) = (374., 112.);
const REWARD_CARD: (f32, f32) = (317., 106.);

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Journeys {
    pub expansion: u8,
    pub expansion_open: bool,
    pub selected: Option<u32>,
    pub level: u32,
}
impl Default for Journeys {
    fn default() -> Self {
        Self {
            expansion: CURRENT_EXPANSION,
            expansion_open: false,
            selected: None,
            level: 0,
        }
    }
}

fn faction(snapshot: Option<&RenownSnapshot>, id: u32) -> Result<&MajorFactionSnapshot, String> {
    snapshot
        .and_then(|snapshot| snapshot.factions.iter().find(|faction| faction.id == id))
        .ok_or_else(|| format!("Unknown major faction {id}"))
}

pub fn apply(
    state: &mut Journeys,
    snapshot: Option<&RenownSnapshot>,
    key: &str,
    value: Option<&str>,
) -> Result<(), String> {
    let number = || {
        value
            .ok_or("Missing journey value")?
            .parse::<u32>()
            .map_err(|_| "Invalid journey value".to_string())
    };
    match key {
        "journey-expansion-menu" => state.expansion_open = !state.expansion_open,
        "journey-expansion" => {
            state.expansion = u8::try_from(number()?).map_err(|_| "Invalid expansion")?;
            state.expansion_open = false;
            state.selected = None;
        }
        "journey" => {
            let id = number()?;
            // JourneyProgressFrameMixin:GetLevels shows the current renown level.
            state.level = faction(snapshot, id)?.level.max(1);
            state.selected = Some(id);
        }
        "journey-back" => state.selected = None,
        "journey-select-level" => {
            let max = faction(snapshot, state.selected.ok_or("No journey selected")?)?.max_level;
            let level = number()?;
            if level == 0 || level > max {
                return Err(format!("Journey level {level} is outside 1..={max}"));
            }
            state.level = level;
        }
        "journey-level" => {
            let max = faction(snapshot, state.selected.ok_or("No journey selected")?)?.max_level;
            state.level = match value {
                Some("prev") => state.level.saturating_sub(1).max(1),
                Some("next") => (state.level + 1).min(max),
                Some("first") => 1,
                Some("last") => max,
                _ => return Err("Unknown journey level step".into()),
            };
        }
        _ => return Err(format!("Unknown journey action {key}")),
    }
    Ok(())
}

/// Replace successive `%d` and drop `|cn…:`/`|r` color markup.
fn format_numbers(template: &str, values: &[u32]) -> String {
    let mut out = template.to_string();
    for value in values {
        out = out.replacen("%d", &value.to_string(), 1);
    }
    while let Some(start) = out.find("|cn") {
        let Some(end) = out[start..].find(':') else {
            break;
        };
        out.replace_range(start..start + end + 1, "");
    }
    out.replace("|r", "")
}

pub fn journeys(
    ctx: &SharedContext,
    state: &Journeys,
    snapshot: Option<&RenownSnapshot>,
) -> Element {
    let Some(snapshot) = snapshot else {
        return text(
            "JournalJourneysLoading",
            "Loading renown…",
            (24., 100., 740., 40.),
        );
    };
    if let Some(error) = &snapshot.source_error {
        return text("JournalJourneysError", error, (24., 100., 740., 40.));
    }
    match state
        .selected
        .and_then(|id| snapshot.factions.iter().find(|f| f.id == id))
    {
        Some(faction) => progress(faction, state.level),
        None => journey_list(ctx, state, snapshot),
    }
}

fn expansion_name(expansion: u8) -> &'static str {
    global_string(&format!("EXPANSION_NAME{expansion}"))
}

fn journey_list(ctx: &SharedContext, state: &Journeys, snapshot: &RenownSnapshot) -> Element {
    let factions: Vec<_> = snapshot
        .factions
        .iter()
        .filter(|faction| faction.expansion == state.expansion)
        .collect();
    let contents = list_contents(ctx, &factions);
    let mut body = rsx! {r#frame {
        name:"EncounterJournalInstanceSelect",left:4.0,top:62.0,width:789.0,height:429.0,
        pos_type:"absolute",{contents}
    }};
    body.extend(crate::adventure_guide_art::dropdown(
        "JournalJourneysExpansion",
        expansion_name(state.expansion),
        "journal:journey-expansion-menu",
        (609., 72., 160., 25.),
        state.expansion_open,
    ));
    if state.expansion_open {
        body.extend(expansion_menu(state, snapshot));
    }
    body
}

/// JourneysList sequence: category20, padding15/10/14/10, horizontal/vertical gaps5.
fn list_contents(ctx: &SharedContext, factions: &[&MajorFactionSnapshot]) -> Element {
    let mut content = label(
        "JournalJourneysCategory".into(),
        global_string("JOURNEYS_RENOWN_LABEL"),
        (14., 15., 765., 20.),
        (16., WHITE, "LEFT"),
    );
    for (index, faction) in factions.iter().enumerate() {
        let left = 14. + (index % 2) as f32 * (CARD.0 + 5.);
        let top = 40. + (index / 2) as f32 * (CARD.1 + 5.);
        content.extend(renown_card(faction, left, top));
    }
    let rows_height = factions.len().div_ceil(2) as f32 * (CARD.1 + 5.);
    let height = 40. + rows_height - 5. + 10.;
    let mut body = named_atlas_texture(
        "EncounterJournalInstanceSelectBackground".into(),
        "ui-journeys-bg",
        (3., 1., 792., 432.),
    );
    body.extend(scroll_in(
        ctx,
        content,
        (0., 0., 789., 429.),
        height.max(429.),
        "EncounterJournalJourneysList",
    ));
    body
}

fn expansion_menu(state: &Journeys, snapshot: &RenownSnapshot) -> Element {
    let expansions: BTreeSet<u8> = snapshot.factions.iter().map(|f| f.expansion).collect();
    let entries: Vec<_> = expansions
        .iter()
        .rev()
        .map(|expansion| {
            (
                format!("JournalJourneysExpansion{expansion}"),
                expansion_name(*expansion),
                format!("journal:journey-expansion:{expansion}"),
                state.expansion == *expansion,
            )
        })
        .collect();
    let items: Vec<_> = entries
        .iter()
        .map(|(name, label, action, _)| ContextMenuItem {
            name,
            label,
            action,
        })
        .collect();
    let rows = entries
        .iter()
        .zip(&items)
        .enumerate()
        .flat_map(|(index, (entry, item))| {
            menu_element(
                *item,
                MenuElementKind::Radio { checked: entry.3 },
                true,
                menu_row_rect_with_heading(160., index, false),
            )
        })
        .collect();
    context_menu_with_rows(
        ContextMenu {
            frame_name: "JournalJourneysExpansionMenu",
            title_name: "JournalJourneysExpansionTitle",
            divider_name: "JournalJourneysExpansionDivider",
            hidden: false,
            title: "",
            width: 160.,
            x: 609.,
            y: 97.,
            items: &items,
        },
        rows,
    )
}

fn renown_card(faction: &MajorFactionSnapshot, left: f32, top: f32) -> Element {
    let prefix = format!("JourneyCard{}", faction.id);
    let mut parts = named_atlas_texture(
        format!("{prefix}IconBorder"),
        "ui-journeys-renown-radial-bar",
        (20., 26., 60., 60.),
    );
    parts.extend(label(
        format!("{prefix}Name"),
        &faction.name,
        (85., 41., 225., 20.),
        (14., WHITE, "LEFT"),
    ));
    parts.extend(label(
        format!("{prefix}Level"),
        &format_numbers(global_string("RENOWN_LEVEL_LABEL"), &[faction.level]),
        (85., 61., 220., 15.),
        (13., NORMAL_FONT_COLOR, "LEFT"),
    ));
    let action = format!("journal:journey:{}", faction.id);
    rsx! {button {
        name:{DynName(prefix)},left,top,width:{CARD.0},height:{CARD.1},pos_type:"absolute",
        onclick:{action.as_str()},button_default_skin:false,
        button_atlas_up:"ui-journeys-renown-button",button_atlas_pressed:"ui-journeys-renown-button-pressed",
        {parts}
    }}
}

fn progress(faction: &MajorFactionSnapshot, level: u32) -> Element {
    let mut body = label(
        "JourneyProgressName".into(),
        &faction.name,
        (144.5, 15., 500., 25.),
        (20., WHITE, "CENTER"),
    );
    body.extend(progress_details(faction));
    body.extend(track::track(faction, level));
    body.extend(named_atlas_texture(
        "JourneyProgressDivider".into(),
        "ui-journeys-renown-divider",
        (45., 177., 699., 16.),
    ));
    body.extend(reward_cards(faction, level));
    body.extend(unlock_notice(&faction.unlock));
    let background = named_atlas_texture(
        "EncounterJournalInstanceSelectBackground".into(),
        "ui-journeys-bg",
        (3., 1., 792., 432.),
    );
    rsx! {r#frame {
        name:"EncounterJournalInstanceSelect",left:4.0,top:62.0,width:789.0,height:429.0,
        pos_type:"absolute",{background} r#frame {
            name:"JourneyProgress",left:0.0,top:0.0,width:789.0,height:429.0,
            pos_type:"absolute",{body}
        }
    }}
}

fn progress_details(faction: &MajorFactionSnapshot) -> Element {
    let standing = if faction.level >= faction.max_level {
        faction.level_threshold
    } else {
        faction.standing
    };
    let current = format_numbers(
        global_string("JOURNEYS_CURRENT_PROGRESS"),
        &[standing, faction.level_threshold],
    );
    let mut contents = named_atlas_texture(
        "JourneyLevelBar".into(),
        "ui-journeys-delve-level-bar",
        (0., 5., 250., 25.),
    );
    contents.extend(named_atlas_texture(
        "JourneyLevelRing".into(),
        "ui-journeys-delve-level-ring",
        (-21., -1., 42., 37.),
    ));
    contents.extend(label(
        "JourneyProgressLevel".into(),
        &faction.level.to_string(),
        (-22., 0., 40., 35.),
        (13., NORMAL_FONT_COLOR, "CENTER"),
    ));
    contents.extend(label(
        "JourneyProgressCurrent".into(),
        &current,
        (0., 2.5, 250., 30.),
        (13., NORMAL_FONT_COLOR, "CENTER"),
    ));
    rsx! {r#frame {name:"JourneyProgressDetails",left:{259.5},top:45.0,width:270.0,height:35.0,
    pos_type:"absolute",{contents}}}
}

pub fn apply_postsetup(registry: &mut ui_toolkit::registry::FrameRegistry) {
    track::sync_arrow_art(registry);
}

/// SetRewards: one card centered +50; two stacked; >2 in two columns below divider.
fn reward_cards(faction: &MajorFactionSnapshot, level: u32) -> Element {
    let rewards: Vec<_> = faction
        .rewards
        .iter()
        .filter(|reward| reward.level == level)
        .take(MAX_REWARD_CARDS)
        .collect();
    rewards
        .iter()
        .enumerate()
        .flat_map(|(index, reward)| {
            let (left, top) = reward_card_position(rewards.len(), index);
            reward_card(reward, left, top)
        })
        .collect()
}

fn reward_card_position(count: usize, index: usize) -> (f32, f32) {
    // Renown track TOP90,height72 -> divider TOP177,height16 -> BOTTOM193.
    match count {
        1 => ((789. - REWARD_CARD.0) / 2., 243.),
        2 => (
            (789. - REWARD_CARD.0) / 2.,
            193. + index as f32 * REWARD_CARD.1,
        ),
        _ => (
            75. + (index % 2) as f32 * (REWARD_CARD.0 + 8.),
            193. + (index / 2) as f32 * REWARD_CARD.1,
        ),
    }
}

fn reward_card(reward: &shared::protocol::RenownReward, left: f32, top: f32) -> Element {
    let prefix = format!("JourneyReward{}", reward.id);
    let mut parts = named_atlas_texture(
        format!("{prefix}BG"),
        "ui-journeys-delve-card",
        (0., 0., 317., 106.),
    );
    parts.extend(texture(
        format!("{prefix}Icon"),
        reward.icon,
        (30., 30.5, 45., 45.),
        WHITE,
    ));
    parts.extend(named_atlas_texture(
        format!("{prefix}IconBorder"),
        "ui-journeys-delve-rewardicon-square-frame",
        (24.5, 25., 56., 56.),
    ));
    parts.extend(label(
        format!("{prefix}Name"),
        &reward.name,
        (85., 33., 200., 40.),
        (13., WHITE, "LEFT"),
    ));
    rsx! {r#frame {
        name:{DynName(format!("{prefix}Card"))},left,top,width:{REWARD_CARD.0},height:{REWARD_CARD.1},
        pos_type:"absolute",mouse_enabled:true,{parts}
    }}
}

/// Blizzard_Journeys.lua:SetRewards OnEnter, without unsourced collection/claim flags.
pub fn reward_tooltip(
    state: &Journeys,
    snapshot: Option<&RenownSnapshot>,
    frame: &str,
) -> Option<crate::game_tooltip::GameTooltip> {
    use crate::tooltip_presentation::{
        TOOLTIP_DESCRIPTION_COLOR, TOOLTIP_WHITE, TooltipLineState, TooltipPresentation,
    };
    let id = frame
        .strip_prefix("JourneyReward")?
        .strip_suffix("Card")?
        .parse::<u32>()
        .ok()?;
    let faction = faction(snapshot, state.selected?).ok()?;
    let reward = faction
        .rewards
        .iter()
        .filter(|reward| reward.level == state.level)
        .take(MAX_REWARD_CARDS)
        .find(|reward| reward.id == id)?;
    Some(crate::game_tooltip::GameTooltip::new(
        TooltipPresentation {
            title: reward.name.clone(),
            title_color: TOOLTIP_WHITE,
            lines: vec![
                TooltipLineState::new(""),
                TooltipLineState::colored(&reward.description, TOOLTIP_DESCRIPTION_COLOR),
            ],
            ..TooltipPresentation::hidden()
        },
        None,
    ))
}

fn unlock_notice(unlock: &RenownUnlock) -> Element {
    let RenownUnlock::Unsupported { description, .. } = unlock else {
        return vec![];
    };
    let notice = if description.is_empty() {
        "Unlock condition is not evaluated by this realm.".to_string()
    } else {
        format!("Unlock condition is not evaluated by this realm: {description}")
    };
    label(
        "JourneyProgressUnlock".into(),
        &notice,
        (24., 430., 752., 40.),
        (12., NORMAL_FONT_COLOR, "CENTER"),
    )
}
