//! EncounterChannel lifecycle and the Retail center-screen warning host.
use game_engine_network::{ProtocolMessage, replica::Replica};
use game_engine_session::SessionScreen;
use game_engine_ui_model::{
    encounter_timeline::EncounterTimeline, encounter_timeline_component::EncounterTimelineScreen,
};
use game_engine_ui_model::{
    inworld_unit_frames_component::MAX_BOSS_FRAMES, raid_warning::RaidWarnings,
};
use godot::prelude::*;
use shared::protocol::{
    EncounterDisengageUnit, EncounterEnd, EncounterEngageUnit, EncounterStart,
    EncounterTimelineEvent,
};

use crate::{
    GameClient, account::decode, frame_error::FrameError, replicated::is_unit, ui::RegistryUi,
};

pub(crate) enum EncounterMessage {
    Start(EncounterStart),
    End(EncounterEnd),
    Engage(EncounterEngageUnit),
    Disengage(EncounterDisengageUnit),
    Timeline(EncounterTimelineEvent),
}

pub(crate) fn is_encounter_message(message: &ProtocolMessage) -> bool {
    message.is::<EncounterTimelineEvent>()
        || message.is::<EncounterStart>()
        || message.is::<EncounterEnd>()
        || message.is::<EncounterEngageUnit>()
        || message.is::<EncounterDisengageUnit>()
}

pub(crate) fn decode_message(message: ProtocolMessage) -> Result<EncounterMessage, String> {
    if message.is::<EncounterTimelineEvent>() {
        return decode(message).map(EncounterMessage::Timeline);
    }
    if message.is::<EncounterStart>() {
        return decode(message).map(EncounterMessage::Start);
    }
    if message.is::<EncounterEnd>() {
        return decode(message).map(EncounterMessage::End);
    }
    if message.is::<EncounterEngageUnit>() {
        return decode(message).map(EncounterMessage::Engage);
    }
    decode(message).map(EncounterMessage::Disengage)
}

#[derive(Default)]
pub(crate) struct EncounterFrames {
    pub active: Option<EncounterStart>,
    engaged: Vec<EncounterEngageUnit>,
}

impl EncounterFrames {
    pub fn receive(&mut self, message: EncounterMessage) {
        match message {
            EncounterMessage::Start(start) => {
                self.clear();
                self.active = Some(start);
            }
            EncounterMessage::End(_) => self.clear(),
            EncounterMessage::Engage(unit) => self.engage(unit),
            EncounterMessage::Disengage(unit) => self.engaged.retain(|boss| boss.unit != unit.unit),
            EncounterMessage::Timeline(_) => {}
        }
    }

    fn engage(&mut self, unit: EncounterEngageUnit) {
        if self.engaged.iter().any(|boss| boss.unit == unit.unit) {
            return;
        }
        self.engaged.push(unit);
        // Stable priority order: equal-priority bosses retain engagement order.
        self.engaged.sort_by_key(|boss| boss.target_frame_priority);
    }

    pub fn clear(&mut self) {
        self.active = None;
        self.engaged.clear();
    }

    #[cfg(test)]
    pub fn units(&self) -> Vec<u64> {
        self.engaged.iter().map(|boss| boss.unit).collect()
    }

    pub fn visible_units(&self, replica: &Replica) -> Vec<u64> {
        self.engaged
            .iter()
            .map(|boss| boss.unit)
            .filter(|&id| replica.unit(id).is_some_and(is_unit))
            .take(MAX_BOSS_FRAMES)
            .collect()
    }
}

/// Resolve the same ordered unit vector that supplied the displayed frames.
pub(crate) fn boss_frame_target(name: &str, units: &[u64]) -> Option<u64> {
    let index = name
        .strip_prefix("Boss")?
        .strip_suffix("TargetFrame")?
        .parse::<usize>()
        .ok()?
        .checked_sub(1)?;
    units.get(index).copied()
}

#[derive(Default)]
pub(crate) struct EncounterHud {
    pub frames: EncounterFrames,
    pub warnings: RaidWarnings,
    pub timeline: EncounterTimeline,
    pub timeline_ui: Option<Gd<RegistryUi>>,
    pub ui: Option<Gd<RegistryUi>>,
}

impl EncounterHud {
    pub fn clear(&mut self) {
        self.frames.clear();
        self.warnings.clear();
        self.timeline.clear();
    }
}

impl GameClient {
    pub(crate) fn receive_encounter(&mut self, message: EncounterMessage) {
        match &message {
            EncounterMessage::Start(start) => {
                godot_print!("ENCOUNTER_START {}", start.encounter_id);
                self.encounter.warnings.clear();
                self.encounter.timeline.clear();
            }
            EncounterMessage::End(end) => {
                godot_print!("ENCOUNTER_END {} success={}", end.encounter_id, end.success);
                self.encounter.warnings.clear();
                self.encounter.timeline.clear();
            }
            _ => {}
        }
        if let EncounterMessage::Timeline(event) = message {
            let play_warning_sound = matches!(
                event,
                EncounterTimelineEvent::Cast { .. } | EncounterTimelineEvent::Phase { .. }
            );
            let Some(catalog) = self.spells.catalog() else {
                godot_error!("Encounter timeline arrived before spell catalog readiness");
                return;
            };
            let spell_ids: Vec<u32> = match &event {
                EncounterTimelineEvent::Timers { timers, .. } => {
                    timers.iter().map(|t| t.spell_id).collect()
                }
                EncounterTimelineEvent::Cast { spell_id, .. } => vec![*spell_id],
                EncounterTimelineEvent::Phase { .. } => Vec::new(),
            };
            if let Some(id) = spell_ids.iter().find(|id| catalog.get(**id).is_none()) {
                godot_error!("Encounter spell {id} absent from shipped catalog");
                return;
            }
            self.encounter.timeline.receive(
                event,
                shared::protocol::read_encounter_unix_ms(),
                |id| {
                    let spell = catalog.get(id).expect("validated encounter spell");
                    (spell.name.to_string(), spell.icon_fdid)
                },
            );
            if play_warning_sound
                && self.client_options.hud.encounter_warnings
                && self.client_options.hud.encounter_sounds
                && let Err(error) =
                    self.play_ui_sound_kit(game_engine_core::ui_sound_kits::RAID_BOSS_EMOTE_WARNING)
            {
                godot_error!("Encounter warning sound: {error}");
            }
            return;
        }
        self.encounter.frames.receive(message);
    }

    pub(crate) fn update_encounter(&mut self, delta: f32) -> Result<(), FrameError> {
        let in_world = self.account.session.screen == SessionScreen::InWorld;
        if !in_world {
            self.encounter.clear();
        } else {
            self.encounter.warnings.advance(delta);
            self.encounter.timeline.advance(delta);
        }
        if self.encounter.ui.is_none() && !self.encounter.warnings.lines.is_empty() {
            self.attach_encounter_warnings()?;
        }
        if let Some(ui) = self.encounter.ui.as_mut() {
            ui.set_visible(in_world);
            ui.bind_mut().set_state(self.encounter.warnings.clone())?;
        }
        let state = EncounterTimelineScreen {
            timeline: self.encounter.timeline.clone(),
            enabled: self.client_options.hud.encounter_timeline,
            bars: self.client_options.hud.encounter_bars,
            warnings: self.client_options.hud.encounter_warnings,
            preview: self.hud_editor.draft.active,
        };
        if in_world && self.encounter.timeline_ui.is_none() {
            let mut ui = RegistryUi::new_alloc();
            ui.set_name("EncounterTimelineUI");
            self.base_mut().add_child(&ui);
            ui.bind_mut().show_encounter_timeline(state.clone())?;
            self.encounter.timeline_ui = Some(ui);
        }
        if let Some(ui) = &mut self.encounter.timeline_ui {
            ui.set_visible(in_world);
            ui.bind_mut().set_state(state)?;
        }
        Ok(())
    }

    fn attach_encounter_warnings(&mut self) -> Result<(), String> {
        let mut ui = RegistryUi::new_alloc();
        ui.set_name("RaidWarningsUI");
        self.base_mut().add_child(&ui);
        let result = ui
            .bind_mut()
            .show_raid_warnings(self.encounter.warnings.clone());
        if let Err(error) = result {
            ui.free();
            return Err(error);
        }
        self.encounter.ui = Some(ui);
        Ok(())
    }
}

#[cfg(test)]
#[path = "encounter_tests.rs"]
mod tests;
