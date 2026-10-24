//! Saved instance state backing Retail boss defeated overlays.
use game_engine_core::adventure_guide::Encounter;
use shared::protocol::InstanceLockInfo;
use ui_toolkit::{rsx, widget_def::Element};

use crate::adventure_guide::View;
use crate::adventure_guide_menus::{JOURNAL_DIFFICULTIES, base_difficulty, difficulty_name};
use game_engine_ui_model::game_tooltip::GameTooltip;
use game_engine_ui_model::quest_art::DynName;
use game_engine_ui_model::tooltip_presentation::{
    TOOLTIP_DESCRIPTION_COLOR, TooltipLineState, TooltipPresentation,
};

impl View {
    /// Replace the received raid-lock view, subtracting only local snapshot age.
    pub fn replace_saved_instances(&mut self, locks: &[InstanceLockInfo], elapsed_secs: u64) {
        self.saved_instances = locks
            .iter()
            .filter(|lock| lock.locked && u64::from(lock.time_remaining_secs) > elapsed_secs)
            .map(|lock| {
                let remaining = u64::from(lock.time_remaining_secs) - elapsed_secs;
                InstanceLockInfo {
                    time_remaining_secs: remaining as u32,
                    ..*lock
                }
            })
            .collect();
        self.lockouts = self
            .saved_instances
            .iter()
            .map(|lock| (lock.map_id, saved_summary(lock)))
            .collect();
    }
}

/// Only a matching active, received save can create a positive defeated marker.
pub(crate) fn is_defeated(view: &View, encounter: &Encounter) -> bool {
    let difficulty = base_difficulty(view.difficulty);
    if !JOURNAL_DIFFICULTIES.contains(&difficulty) {
        return false;
    }
    let Some(metadata) = view
        .catalog
        .encounter_locks
        .get(&encounter.dungeon_encounter)
    else {
        return false;
    };
    let Some(instance) = view.catalog.instance(encounter.instance) else {
        return false;
    };
    let matches_difficulty =
        metadata.difficulty == 0 || base_difficulty(metadata.difficulty) == difficulty;
    if metadata.map != instance.map || !matches_difficulty {
        return false;
    }
    let Some(bit) = 1_u32.checked_shl(metadata.bit) else {
        return false;
    };
    view.saved_instances.iter().any(|lock| {
        let active = lock.locked && lock.time_remaining_secs > 0;
        let key = (lock.map_id, base_difficulty(lock.difficulty_id));
        let completed = lock.completed_mask & bit != 0;
        active && key == (metadata.map, difficulty) && completed
    })
}

pub(crate) fn overlay(view: &View, encounter: &Encounter) -> Element {
    if !is_defeated(view, encounter) {
        return vec![];
    }
    let icon = game_engine_ui_model::inworld_unit_frames_component::inworld_unit_frames_art::sized_atlas_texture(
        format!("JournalBoss{}DefeatedIcon", encounter.id),
        "Map-MarkedDefeated",
        ui_toolkit::atlas::thread_skin(),
        |(width, height)| ((16. - width) / 2., (16. - height) / 2., width, height),
        "1,1,1,1",
        false,
    );
    rsx! {
        button {
            name: {DynName(format!("JournalBoss{}DefeatedOverlay", encounter.id))},
            left: 4., top: 39., width: 16., height: 16., frame_level: 150.,
            pos_type: "absolute", button_default_skin: false,
            tooltip_provider: "journal_defeated", tooltip_anchor: "ANCHOR_RIGHT", {icon}
        }
    }
}

pub fn tooltip(view: &View, frame: &str) -> Option<GameTooltip> {
    let id = frame
        .strip_prefix("JournalBoss")?
        .strip_suffix("DefeatedOverlay")?
        .parse()
        .ok()?;
    let encounter = view.catalog.encounter(id)?;
    if !is_defeated(view, encounter) {
        return None;
    }
    let difficulty = difficulty_name(base_difficulty(view.difficulty));
    let line = game_engine_ui_model::auction_mail::text(
        "ENCOUNTER_JOURNAL_ENCOUNTER_STATUS_DEFEATED_TOOLTIP",
    )
    .replacen("%s", difficulty, 1);
    Some(GameTooltip::new(
        TooltipPresentation {
            lines: vec![TooltipLineState::colored(line, TOOLTIP_DESCRIPTION_COLOR)],
            ..Default::default()
        },
        None,
    ))
}

fn saved_summary(lock: &InstanceLockInfo) -> String {
    format!(
        "Saved · difficulty {} · {} bosses defeated · resets in {} minutes",
        lock.difficulty_id,
        lock.completed_mask.count_ones(),
        u64::from(lock.time_remaining_secs).div_ceil(60),
    )
}
