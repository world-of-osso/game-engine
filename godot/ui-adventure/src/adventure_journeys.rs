//! Retail Mainline Journeys (Blizzard_Journeys.lua/.xml) over realm renown snapshots.
//! Renown cards and the reward track only; unsourced journey/encounter categories,
//! claim state and Delves companions are not shown. Texture kits/priorities are local source data.
mod sliding;
mod track;
use crate::adventure_guide::{scroll_in, text};
use game_engine_core::adventure_guide::JourneyPresentation;
use game_engine_ui_model::auction_mail::text as global_string;
use game_engine_ui_model::bank_art::{WHITE, label, texture};
use game_engine_ui_model::menu_primitives::{
    ContextMenu, ContextMenuItem, MenuElementKind, context_menu_with_rows, menu_element,
    menu_row_rect_with_heading,
};
use game_engine_ui_model::quest_art::{DynName, NORMAL_FONT_COLOR, named_atlas_texture};
use shared::protocol::{MajorFactionSnapshot, RenownSnapshot, RenownUnlock};
use std::collections::{BTreeMap, BTreeSet};
use ui_toolkit::{rsx, screen::SharedContext, widget_def::Element};

/// Retail 12.1 client `LE_EXPANSION_LEVEL_CURRENT` (EXPANSION_NAME11 "Midnight").
pub const CURRENT_EXPANSION: u8 = 11;
/// Blizzard_Journeys.lua `MAX_REWARD_CARDS_TO_DISPLAY`.
const MAX_REWARD_CARDS: usize = 4;
const CARD: (f32, f32) = (374., 112.);
const REWARD_CARD: (f32, f32) = (317., 106.);

#[derive(Clone, Debug, PartialEq)]
pub struct Journeys {
    pub expansion: u8,
    pub expansion_open: bool,
    pub selected: Option<u32>,
    pub hovered: Option<u32>,
    pub level: u32,
    pub scroll: Option<sliding::Scroll>,
}
impl Default for Journeys {
    fn default() -> Self {
        Self {
            expansion: CURRENT_EXPANSION,
            expansion_open: false,
            selected: None,
            hovered: None,
            level: 0,
            scroll: None,
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
    if state.is_scrolling() && matches!(key, "journey-select-level" | "journey-level") {
        return Ok(());
    }
    state.scroll = None;
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
    presentation: &BTreeMap<u32, JourneyPresentation>,
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
    if let Some(error) = presentation_error(snapshot, presentation) {
        return error;
    }
    match state
        .selected
        .and_then(|id| snapshot.factions.iter().find(|f| f.id == id))
    {
        Some(faction) => progress(faction, state),
        None => journey_list(ctx, state, snapshot, presentation),
    }
}

fn expansion_name(expansion: u8) -> &'static str {
    global_string(&format!("EXPANSION_NAME{expansion}"))
}

fn presentation_error(
    snapshot: &RenownSnapshot,
    presentation: &BTreeMap<u32, JourneyPresentation>,
) -> Option<Element> {
    let missing = snapshot
        .factions
        .iter()
        .find(|faction| !presentation.contains_key(&faction.id))?;
    Some(text(
        "JournalJourneysPresentationError",
        &format!("Missing Covenant {} display metadata", missing.id),
        (24., 100., 740., 40.),
    ))
}

/// Blizzard_Journeys.lua JourneysMajorFactionSort: priority descending, then UTF-8 name.
fn sorted_factions_for_expansion<'a>(
    snapshot: &'a RenownSnapshot,
    expansion: u8,
    presentation: &BTreeMap<u32, JourneyPresentation>,
) -> Vec<&'a MajorFactionSnapshot> {
    let mut factions: Vec<_> = snapshot
        .factions
        .iter()
        .filter(|faction| faction.expansion == expansion)
        .collect();
    factions.sort_by(|left, right| {
        let priority = presentation[&right.id]
            .priority
            .cmp(&presentation[&left.id].priority);
        priority.then_with(|| left.name.to_lowercase().cmp(&right.name.to_lowercase()))
    });
    factions
}

fn journey_list(
    ctx: &SharedContext,
    state: &Journeys,
    snapshot: &RenownSnapshot,
    presentation: &BTreeMap<u32, JourneyPresentation>,
) -> Element {
    let factions = sorted_factions_for_expansion(snapshot, state.expansion, presentation);
    let contents = list_contents(ctx, &factions, state, snapshot, presentation);
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
fn list_contents(
    ctx: &SharedContext,
    factions: &[&MajorFactionSnapshot],
    state: &Journeys,
    snapshot: &RenownSnapshot,
    presentation: &BTreeMap<u32, JourneyPresentation>,
) -> Element {
    let mut content = label(
        "JournalJourneysCategory".into(),
        global_string("JOURNEYS_RENOWN_LABEL"),
        (14., 15., 765., 20.),
        (16., WHITE, "LEFT"),
    );
    for (index, faction) in factions.iter().enumerate() {
        let left = 14. + (index % 2) as f32 * (CARD.0 + 5.);
        let top = 40. + (index / 2) as f32 * (CARD.1 + 5.);
        content.extend(renown_card(
            faction,
            &presentation[&faction.id],
            (left, top),
            state,
            snapshot,
        ));
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

fn renown_card(
    faction: &MajorFactionSnapshot,
    presentation: &JourneyPresentation,
    position: (f32, f32),
    state: &Journeys,
    snapshot: &RenownSnapshot,
) -> Element {
    let (left, top) = position;
    let prefix = format!("JourneyCard{}", faction.id);
    let mut parts = card_art(faction, &prefix, &presentation.texture_kit);
    parts.extend(watch_toggle(faction, state, snapshot));
    let action = format!("journal:journey:{}", faction.id);
    rsx! {button {
        name:{DynName(prefix)},left,top,width:{CARD.0},height:{CARD.1},pos_type:"absolute",
        onclick:{action.as_str()},button_default_skin:false,
        button_atlas_up:"ui-journeys-renown-button",button_atlas_pressed:"ui-journeys-renown-button-pressed",
        {parts}
    }}
}

fn card_art(faction: &MajorFactionSnapshot, prefix: &str, texture_kit: &str) -> Element {
    let mut parts = named_atlas_texture(
        format!("{prefix}Icon"),
        &format!("majorfactions_icons_{texture_kit}512"),
        (30., 36., 40., 40.),
    );
    parts.extend(named_atlas_texture(
        format!("{prefix}IconBorder"),
        "ui-journeys-renown-radial-bar",
        (20., 26., 60., 60.),
    ));
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
    parts
}

/// RenownCard OnEnter/OnLeave show the source toggle only for known unlocked factions.
fn watch_toggle(
    faction: &MajorFactionSnapshot,
    state: &Journeys,
    snapshot: &RenownSnapshot,
) -> Element {
    let eligible = matches!(faction.unlock, RenownUnlock::Ungated);
    if !eligible || state.hovered != Some(faction.id) {
        return Vec::new();
    }
    let name = format!("JourneyCard{}Watch", faction.id);
    let checked = snapshot.watched_faction == Some(faction.faction_id);
    let target = if checked { 0 } else { faction.faction_id };
    let action = format!("journal:journey-watch:{}:{target}", faction.id);
    let contents = game_engine_ui_model::bank_art::checkbox_sized(
        &format!("{name}Checkbox"),
        global_string("SHOW_FACTION_ON_MAINSCREEN"),
        checked,
        &action,
        (0., -3., 26., -2.),
    );
    rsx! {r#frame {name:{DynName(name)},left:204.0,top:76.0,width:200.0,height:20.0,
    pos_type:"absolute",{contents}}}
}

/// WatchedFactionToggleFrameMixin:OnClick plays the cue for the requested checked state.
pub fn watch_click_sound(faction_id: u32) -> u32 {
    use game_engine_core::ui_sound_kits::{
        IG_MAINMENU_OPTION_CHECKBOX_OFF, IG_MAINMENU_OPTION_CHECKBOX_ON,
    };
    if faction_id == 0 {
        IG_MAINMENU_OPTION_CHECKBOX_OFF
    } else {
        IG_MAINMENU_OPTION_CHECKBOX_ON
    }
}

/// Native hit testing supplies the card or its watch child action, including clipped rows.
pub fn hovered_faction(action: &str) -> Option<u32> {
    action
        .strip_prefix("journal:journey:")
        .or_else(|| action.strip_prefix("journal:journey-watch:"))?
        .split(':')
        .next()?
        .parse()
        .ok()
}

fn progress(faction: &MajorFactionSnapshot, state: &Journeys) -> Element {
    let level = state.level;
    let mut body = label(
        "JourneyProgressName".into(),
        &faction.name,
        (144.5, 15., 500., 25.),
        (20., WHITE, "CENTER"),
    );
    body.extend(progress_details(faction));
    body.extend(track::track(faction, state));
    body.extend(named_atlas_texture(
        "JourneyProgressDivider".into(),
        "ui-journeys-renown-divider",
        (45., 177., 699., 16.),
    ));
    body.extend(reward_cards(faction, level));
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
        pos_type:"absolute",mouse_enabled:true,tooltip_provider:"journal_journey",tooltip_anchor:"ANCHOR_RIGHT",{parts}
    }}
}

/// Blizzard_Journeys.lua:SetRewards OnEnter, without unsourced collection/claim flags.
pub fn reward_tooltip(
    state: &Journeys,
    snapshot: Option<&RenownSnapshot>,
    frame: &str,
) -> Option<game_engine_ui_model::game_tooltip::GameTooltip> {
    use game_engine_ui_model::tooltip_presentation::{
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
    Some(game_engine_ui_model::game_tooltip::GameTooltip::new(
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
