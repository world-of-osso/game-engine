//! Retail Mainline Journeys (Blizzard_Journeys.lua/.xml) over realm renown snapshots.
//! Renown cards and the reward track only; unsourced journey/encounter categories,
//! texture kits, claim state and Delves companions are not shown.
use crate::adventure_guide::{button, scroll, text};
use crate::auction_mail::text as global_string;
use crate::bank_art::{WHITE, label, texture};
use crate::quest_art::{DynName, NORMAL_FONT_COLOR};
use crate::selection_list::MenuTick;
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
        "journey-level" => {
            let max = faction(snapshot, state.selected.ok_or("No journey selected")?)?.max_level;
            state.level = match value {
                Some("prev") => state.level.saturating_sub(1).max(1),
                Some("next") => (state.level + 1).min(max),
                _ => return Err("Unknown journey level step".into()),
            };
        }
        _ => return Err(format!("Unknown journey action {key}")),
    }
    Ok(())
}

fn fill(name: String, rect: (f32, f32, f32, f32), color: &str) -> Element {
    let (left, top, width, height) = rect;
    rsx! {r#frame {name:{DynName(name)},left,top,width,height,pos_type:"absolute",background_color:color}}
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
    let mut content = label(
        "JournalJourneysCategory".into(),
        global_string("JOURNEYS_RENOWN_LABEL"),
        (0., 0., 400., 20.),
        (16., WHITE, "LEFT"),
    );
    for (index, faction) in factions.iter().enumerate() {
        let left = (index % 2) as f32 * (CARD.0 + 4.);
        let top = 26. + (index / 2) as f32 * (CARD.1 + 6.);
        content.extend(renown_card(faction, left, top));
    }
    let height = 26. + factions.len().div_ceil(2) as f32 * (CARD.1 + 6.);
    let mut body = button(
        "JournalJourneysExpansion".into(),
        expansion_name(state.expansion),
        "journal:journey-expansion-menu",
        (15., 65., 175., 24.),
    );
    body.extend(scroll(
        ctx,
        content,
        (15., 96., 770., 387.),
        height.max(387.),
    ));
    if state.expansion_open {
        body.extend(expansion_menu(state, snapshot));
    }
    body
}

fn expansion_menu(state: &Journeys, snapshot: &RenownSnapshot) -> Element {
    let expansions: BTreeSet<u8> = snapshot.factions.iter().map(|f| f.expansion).collect();
    expansions
        .iter()
        .rev()
        .enumerate()
        .flat_map(|(index, expansion)| {
            crate::selection_list::menu_choice(
                format!("JournalJourneysExpansion{expansion}"),
                expansion_name(*expansion),
                &format!("journal:journey-expansion:{expansion}"),
                (MenuTick::Radio, state.expansion == *expansion),
                (15., 90. + index as f32 * 24., 175., 23.),
            )
        })
        .collect()
}

fn renown_card(faction: &MajorFactionSnapshot, left: f32, top: f32) -> Element {
    let prefix = format!("JourneyCard{}", faction.id);
    let mut card = button(
        prefix.clone(),
        "",
        &format!("journal:journey:{}", faction.id),
        (left, top, CARD.0, CARD.1),
    );
    card.extend(label(
        format!("{prefix}Name"),
        &faction.name,
        (left + 90., top + 30., 225., 20.),
        (14., WHITE, "LEFT"),
    ));
    card.extend(label(
        format!("{prefix}Level"),
        &format_numbers(global_string("RENOWN_LEVEL_LABEL"), &[faction.level]),
        (left + 90., top + 54., 220., 15.),
        (13., NORMAL_FONT_COLOR, "LEFT"),
    ));
    card
}

fn progress(faction: &MajorFactionSnapshot, level: u32) -> Element {
    let maxed = faction.level >= faction.max_level;
    let mut body = button(
        "JourneyProgressBack".into(),
        global_string("BACK"),
        "journal:journey-back",
        (15., 65., 90., 24.),
    );
    body.extend(label(
        "JourneyProgressName".into(),
        &faction.name,
        (150., 66., 500., 25.),
        (20., WHITE, "CENTER"),
    ));
    body.extend(label(
        "JourneyProgressLevel".into(),
        &format_numbers(
            global_string("JOURNEYS_LEVEL_LABEL"),
            &[faction.level, faction.max_level],
        ),
        (300., 96., 200., 20.),
        (13., NORMAL_FONT_COLOR, "CENTER"),
    ));
    body.extend(progress_bar(faction, maxed));
    body.extend(level_selector(level));
    body.extend(reward_cards(faction, level));
    body.extend(unlock_notice(&faction.unlock));
    body
}

fn progress_bar(faction: &MajorFactionSnapshot, maxed: bool) -> Element {
    // JourneysProgressBarMixin:RefreshBar fills completely at maximum renown.
    let ratio = if maxed {
        1.
    } else {
        faction.standing as f32 / faction.level_threshold.max(1) as f32
    };
    let mut bar = fill(
        "JourneyProgressBarBG".into(),
        (120., 122., 560., 14.),
        "0.1,0.1,0.1,1.0",
    );
    bar.extend(fill(
        "JourneyProgressBarFill".into(),
        (120., 122., 560. * ratio, 14.),
        "0.2,0.55,0.95,1.0",
    ));
    let current = if maxed {
        global_string("JOURNEYS_MAX_LEVEL_LABEL").to_string()
    } else {
        format_numbers(
            global_string("JOURNEYS_CURRENT_PROGRESS"),
            &[faction.standing, faction.level_threshold],
        )
    };
    bar.extend(label(
        "JourneyProgressCurrent".into(),
        &current,
        (120., 140., 560., 18.),
        (12., WHITE, "CENTER"),
    ));
    bar
}

fn level_selector(level: u32) -> Element {
    let mut row = button(
        "JourneyLevelPrev".into(),
        "<",
        "journal:journey-level:prev",
        (300., 166., 30., 24.),
    );
    row.extend(label(
        "JourneyLevelSelected".into(),
        &format_numbers(global_string("JOURNEYS_HIGHLIGHT_LEVEL_LABEL"), &[level]),
        (335., 168., 130., 20.),
        (14., WHITE, "CENTER"),
    ));
    row.extend(button(
        "JourneyLevelNext".into(),
        ">",
        "journal:journey-level:next",
        (470., 166., 30., 24.),
    ));
    row
}

/// JourneyProgressFrameMixin:SetRewards layout: one card centered, otherwise two columns.
fn reward_cards(faction: &MajorFactionSnapshot, level: u32) -> Element {
    let rewards: Vec<_> = faction
        .rewards
        .iter()
        .filter(|reward| reward.level == level)
        .take(MAX_REWARD_CARDS)
        .collect();
    let single = rewards.len() == 1;
    rewards
        .iter()
        .enumerate()
        .flat_map(|(index, reward)| {
            let left = if single {
                (800. - REWARD_CARD.0) / 2.
            } else {
                75. + (index % 2) as f32 * (REWARD_CARD.0 + 8.)
            };
            let top = 200. + (index / 2) as f32 * REWARD_CARD.1;
            let prefix = format!("JourneyReward{}", reward.id);
            let mut card = fill(
                format!("{prefix}Card"),
                (left, top, REWARD_CARD.0, REWARD_CARD.1 - 6.),
                "0.12,0.10,0.08,1.0",
            );
            card.extend(texture(
                format!("{prefix}Icon"),
                reward.icon,
                (left + 20., top + 28., 45., 45.),
                WHITE,
            ));
            card.extend(label(
                format!("{prefix}Name"),
                &reward.name,
                (left + 80., top + 30., 200., 40.),
                (13., WHITE, "LEFT"),
            ));
            card
        })
        .collect()
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
