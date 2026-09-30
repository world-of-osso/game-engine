//! Combat and spell clips layered over locomotion: melee swings, hit reactions, cast
//! releases (played once) and precast/channel loops (held until stopped).
//!
//! The layer crossfades in and out with the clip's M2 `blend_time` (150 ms minimum,
//! AGENTS.md). Upper-body bones (M2 key bone SpineLow's subtree, `AnimKitBoneSet` 1)
//! always take the action; lower-body bones take it only while the unit stands, so a
//! unit swinging or casting on the move keeps its legs on the run cycle (Retail
//! `AnimKitConfig` upper-body segments). Replacing an action mid-play crossfades from
//! the pose it had reached and keeps the layer weight, so neither pops.
//!
//! A cast clip releases the caster's pending spell missiles at its first `$CSL`, `$CSR`
//! or `$CST` M2 event (wowdev.wiki/M2 Events: "release_missiles_on_next_update if
//! has_pending_missiles"); SpellCastDirected (53) of HumanMale/HumanFemale HD fires
//! `$CSL` at 200 ms, as the arm thrusts.

use std::collections::HashMap;

use game_engine_core::m2;

use super::{AnimationState, BonePose, MIN_MOVEMENT_BLEND_MS};

/// M2 key bone id of SpineLow, the root of `AnimKitBoneSet` 1 (upper body).
const UPPER_BODY_KEY_BONE: i32 = 4;

/// Locomotion clips that leave the legs free for an action: Stand, the Ready stances
/// (Unarmed/1H/2H/2HL) and SwimIdle.
const STATIONARY_ANIMS: [u16; 6] = [0, 25, 26, 27, 28, 41];

/// M2 events that release pending spell missiles (left hand, right hand, generic).
const MISSILE_RELEASE_EVENTS: [&[u8; 4]; 3] = [b"$CSL", b"$CSR", b"$CST"];

/// M2 events an action clip reports as it passes them (wowdev.wiki/M2 Events): `$SCD`
/// plays the unit's spell-cast-directed voice, `$CSS` its weapon swoosh and `$CAH`
/// lands its melee swing.
const REPORTED_EVENTS: [&[u8; 4]; 3] = [b"$SCD", b"$CSS", b"$CAH"];

/// Which actions may replace a playing one: a spell's kit animation is not cut short
/// by a melee swing or hit reaction arriving mid-cast, nor a swing by a hit reaction
/// (the swing must reach its `$CSS`/`$CAH` events); equal or higher priority replaces.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub(crate) enum ActionPriority {
    /// Hit reactions: wound, crit, dodge, parry, block.
    Reaction,
    /// Melee swings.
    Combat,
    /// Spell visual kit animations (cast releases, precast and channel loops).
    Spell,
}

pub(super) struct ActionLayer {
    index: usize,
    priority: ActionPriority,
    time_ms: f64,
    looping: bool,
    /// Pose of the action this one replaced, faded out over `fade_ms`.
    outgoing: Option<(Vec<BonePose>, f32)>,
    fade_ms: f32,
    pub(super) upper: f32,
    pub(super) lower: f32,
    releasing: bool,
    /// The clip's missile release event has yet to fire.
    awaits_release: bool,
    /// Index of its next reported event.
    next_event: usize,
}

/// Per sequence: its reported events (time ms, identifier), in time order.
pub(super) fn reported_events(model: &m2::Model) -> Vec<Vec<(u32, [u8; 4])>> {
    (0..model.sequences.len())
        .map(|sequence| {
            let mut events: Vec<(u32, [u8; 4])> = model
                .events
                .iter()
                .filter(|event| REPORTED_EVENTS.contains(&&event.identifier))
                .flat_map(|event| {
                    event
                        .timestamps
                        .get(sequence)
                        .into_iter()
                        .flatten()
                        .map(|&time| (time, event.identifier))
                })
                .collect();
            events.sort();
            events
        })
        .collect()
}

/// Per sequence: when its first missile release event fires (ms), if it has one.
pub(super) fn missile_release_times(model: &m2::Model) -> Vec<Option<u32>> {
    (0..model.sequences.len())
        .map(|sequence| {
            model
                .events
                .iter()
                .filter(|event| MISSILE_RELEASE_EVENTS.contains(&&event.identifier))
                .filter_map(|event| event.timestamps.get(sequence)?.iter().min().copied())
                .min()
        })
        .collect()
}

/// Per bone: in the subtree of key bone SpineLow (all `false` when the model has none).
pub(super) fn upper_body_bones(model: &m2::Model) -> Vec<bool> {
    let Some(root) = model
        .bones
        .iter()
        .position(|bone| bone.key_bone_id == UPPER_BODY_KEY_BONE)
    else {
        return vec![false; model.bones.len()];
    };
    (0..model.bones.len())
        .map(|index| {
            let mut bone = index as i32;
            while bone >= 0 {
                if bone as usize == root {
                    return true;
                }
                bone = i32::from(model.bones[bone as usize].parent_bone_id);
            }
            false
        })
        .collect()
}

fn approach(value: f32, target: f32, step: f32) -> f32 {
    if value < target {
        (value + step).min(target)
    } else {
        (value - step).max(target)
    }
}

impl AnimationState {
    /// `id`, or its `AnimationData.Fallback` chain's first clip the model has.
    pub fn resolve_clip(&self, id: u16, fallbacks: &HashMap<u16, u16>) -> Option<u16> {
        let mut clip = id;
        for _ in 0..16 {
            if self.base_sequence(clip).is_some() {
                return Some(clip);
            }
            clip = *fallbacks.get(&clip)?;
            // A chain ending in Stand has no action to show.
            if clip == 0 {
                return None;
            }
        }
        None
    }

    fn base_sequence(&self, id: u16) -> Option<usize> {
        self.sequences
            .iter()
            .position(|sequence| sequence.id == id && sequence.variation_id == 0)
    }

    /// Play clip `id` over locomotion: once, or held while `looping` until
    /// [`Self::stop_action`]. Requesting the held loop again keeps it and its blend.
    /// `false`: a higher-priority action is playing and keeps playing.
    pub fn play_action(
        &mut self,
        id: u16,
        looping: bool,
        priority: ActionPriority,
    ) -> Result<bool, String> {
        let index = self
            .base_sequence(id)
            .ok_or_else(|| format!("M2 animation ID {id} has no base variation"))?;
        let fade_ms = (self.sequences[index].blend_time as f32).max(MIN_MOVEMENT_BLEND_MS);
        if let Some(action) = &mut self.action
            && !action.releasing
        {
            if action.priority > priority {
                return Ok(false);
            }
            if action.index == index && action.looping && looping {
                return Ok(true);
            }
        }
        let (outgoing, upper, lower) = match self.action.take() {
            Some(previous) => {
                let pose = self.sample_action(&previous);
                (Some((pose, 0.0)), previous.upper, previous.lower)
            }
            None => (None, 0.0, 0.0),
        };
        self.action = Some(ActionLayer {
            index,
            priority,
            time_ms: 0.0,
            looping,
            outgoing,
            fade_ms,
            upper,
            lower,
            releasing: false,
            awaits_release: self.release_ms[index].is_some(),
            next_event: 0,
        });
        Ok(true)
    }

    /// Fade out the held action `id`; another clip or no action is left alone.
    pub fn stop_action(&mut self, id: u16) {
        if let Some(action) = &mut self.action
            && self.sequences[action.index].id == id
        {
            action.releasing = true;
        }
    }

    /// The playing action clip has a missile release event that has not fired yet.
    pub fn awaits_missile_release(&self) -> bool {
        self.action
            .as_ref()
            .is_some_and(|action| action.awaits_release && !action.releasing)
    }

    /// Reported M2 events (`$SCD`) the action clips passed since the last call.
    pub fn take_fired_events(&mut self) -> Vec<[u8; 4]> {
        std::mem::take(&mut self.fired_events)
    }

    /// The action clip playing or fading, if any.
    pub fn action_id(&self) -> Option<u16> {
        let action = self.action.as_ref()?;
        (!action.releasing).then(|| self.sequences[action.index].id)
    }

    pub(super) fn set_locomotion_stationary(&mut self, movement_id: u16, jumping: bool) {
        self.legs_free = !jumping && STATIONARY_ANIMS.contains(&movement_id);
    }

    pub(super) fn tick_action(&mut self, delta_ms: f64) {
        let legs_free = self.legs_free;
        let Some(action) = &mut self.action else {
            return;
        };
        let duration = f64::from(self.sequences[action.index].duration);
        action.time_ms += delta_ms;
        if self.release_ms[action.index].is_some_and(|release| action.time_ms >= f64::from(release))
        {
            action.awaits_release = false;
        }
        let events = &self.action_events[action.index];
        while let Some(&(time, identifier)) = events.get(action.next_event) {
            if f64::from(time) > action.time_ms {
                break;
            }
            self.fired_events.push(identifier);
            action.next_event += 1;
        }
        if action.time_ms >= duration {
            if action.looping && duration > 0.0 {
                action.time_ms %= duration;
                action.next_event = 0;
            } else {
                action.time_ms = duration;
                action.releasing = true;
            }
        }
        let step = delta_ms as f32 / action.fade_ms;
        if let Some((_, elapsed)) = &mut action.outgoing {
            *elapsed += delta_ms as f32;
            if *elapsed >= action.fade_ms {
                action.outgoing = None;
            }
        }
        let upper_target = if action.releasing { 0.0 } else { 1.0 };
        let lower_target = if action.releasing || !legs_free {
            0.0
        } else {
            1.0
        };
        action.upper = approach(action.upper, upper_target, step);
        action.lower = approach(action.lower, lower_target, step);
        if action.releasing && action.upper == 0.0 && action.lower == 0.0 {
            self.action = None;
        }
    }

    fn sample_action(&self, action: &ActionLayer) -> Vec<BonePose> {
        let current = self.sample_sequence(action.index, action.time_ms);
        let Some((outgoing, elapsed)) = &action.outgoing else {
            return current;
        };
        let weight = (elapsed / action.fade_ms).clamp(0.0, 1.0);
        outgoing
            .iter()
            .zip(current)
            .map(|(from, to)| from.blend(to, weight))
            .collect()
    }

    /// `base` with the action layer blended per bone.
    pub(super) fn apply_action(&self, base: Vec<BonePose>) -> Vec<BonePose> {
        let Some(action) = &self.action else {
            return base;
        };
        let poses = self.sample_action(action);
        base.into_iter()
            .zip(poses)
            .zip(&self.upper_body)
            .map(|((base, pose), &upper)| {
                let weight = if upper { action.upper } else { action.lower };
                base.blend(pose, weight)
            })
            .collect()
    }
}
