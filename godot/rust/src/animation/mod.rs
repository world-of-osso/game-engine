//! Sequence-local M2 bone playback on Godot's native Skeleton3D palette.
//! Poses use local pivots matching the skeleton's rest and absolute inverse binds.
use game_engine_core::movement_animation_data::locomotion_playback_rate;
use game_engine_core::{asset::m2_format::m2_anim, m2};
use godot::{
    builtin::{Quaternion, Vector3},
    classes::{INode, MeshInstance3D, Node, Skeleton3D},
    prelude::*,
};

#[cfg(test)]
use godot::builtin::{Basis, Transform3D};

mod action;
mod billboard;
pub(crate) use action::ActionPriority;
pub(crate) mod lod;

const MIN_MOVEMENT_BLEND_MS: f32 = 150.0;
/// Death: a one-shot clip that holds its last frame.
const ANIM_DEATH: u16 = 1;
const ANIM_STAND: u16 = 0;

/// A stand state pose loop with its one-shot down and up clips (wowdev AnimationList;
/// AnimationData.db2 12.1.0.69933 falls the loop and up clip back to the down clip).
struct PoseClips {
    down: u16,
    pose: u16,
    up: u16,
}

const POSE_CLIPS: [PoseClips; 3] = [
    // SitGroundDown, SitGround, SitGroundUp.
    PoseClips {
        down: 96,
        pose: 97,
        up: 98,
    },
    // SleepDown, Sleep, SleepUp.
    PoseClips {
        down: 99,
        pose: 100,
        up: 101,
    },
    // KneelStart, KneelLoop, KneelEnd.
    PoseClips {
        down: 114,
        pose: 115,
        up: 116,
    },
];

fn pose_clips(matches: impl Fn(&PoseClips) -> bool) -> Option<&'static PoseClips> {
    POSE_CLIPS.iter().find(|clips| matches(clips))
}

fn wow_vec3(value: [f32; 3]) -> Vector3 {
    Vector3::new(value[0], value[2], -value[1])
}

#[derive(Clone, Copy)]
struct BonePose {
    position: Vector3,
    rotation: Quaternion,
    scale: Vector3,
}

impl BonePose {
    #[cfg(test)]
    fn transform(self) -> Transform3D {
        Transform3D::new(
            Basis::from_quaternion(self.rotation).scaled(self.scale),
            self.position,
        )
    }

    fn blend(self, target: Self, weight: f32) -> Self {
        Self {
            position: self.position.lerp(target.position, weight),
            rotation: blend_rotation(self.rotation, target.rotation, weight),
            scale: self.scale.lerp(target.scale, weight),
        }
    }
}

fn blend_rotation(start: Quaternion, target: Quaternion, weight: f32) -> Quaternion {
    let mut end = target;
    let mut dot = start.dot(end);
    if dot < 0.0 {
        end = -end;
        dot = -dot;
    }
    let (start_weight, end_weight) = if dot > 0.9995 {
        (1.0 - weight, weight)
    } else {
        let angle = dot.acos();
        let denominator = angle.sin();
        (
            ((1.0 - weight) * angle).sin() / denominator,
            (weight * angle).sin() / denominator,
        )
    };
    Quaternion::new(
        start.x * start_weight + end.x * end_weight,
        start.y * start_weight + end.y * end_weight,
        start.z * start_weight + end.z * end_weight,
        start.w * start_weight + end.w * end_weight,
    )
    .normalized()
}

enum Outgoing {
    Sequence { index: usize, time_ms: f64 },
    Snapshot(Vec<BonePose>),
}

struct Transition {
    outgoing: Outgoing,
    elapsed_ms: f32,
    duration_ms: f32,
}

fn sample_roll(state: &mut u64, upper: u32) -> u32 {
    let threshold = upper.wrapping_neg() % upper;
    loop {
        *state = state.wrapping_add(0x9e3779b97f4a7c15);
        let mut value = *state;
        value = (value ^ (value >> 30)).wrapping_mul(0xbf58476d1ce4e5b9);
        value = (value ^ (value >> 27)).wrapping_mul(0x94d049bb133111eb);
        let value = (value ^ (value >> 31)) as u32;
        if value >= threshold {
            return value % upper;
        }
    }
}

/// Global-sequence tracks keep their keys in timeline 0 for every sequence.
fn keyframed<T>(track: &m2_anim::AnimTrack<T>, sequence: usize) -> bool {
    let timeline = if track.global_sequence < 0 {
        sequence
    } else {
        0
    };
    track
        .sequences
        .get(timeline)
        .is_some_and(|(times, _)| times.len() > 1)
}

/// Deterministic M2 sequence controller, independent of Godot frame timing.
/// The caller owns explicit clip selection, time advancement and pause policy.
pub struct AnimationState {
    sequences: Vec<m2::Sequence>,
    tracks: std::sync::Arc<Vec<m2::BoneAnimTracks>>,
    /// Global sequence durations (ms) and the model's global clock, which runs
    /// whichever sequence plays.
    global_sequences: Vec<u32>,
    global_ms: f64,
    local_pivots: Vec<Vector3>,
    current: usize,
    time_ms: f64,
    looping: bool,
    transition: Option<Transition>,
    random_state: u64,
    /// Per sequence: whether any bone track has more than one keyframe.
    sequence_animated: Vec<bool>,
    /// Combat/spell clip layered over the selected sequence.
    action: Option<action::ActionLayer>,
    /// Per bone: in the upper-body set an action always drives.
    upper_body: Vec<bool>,
    /// Per sequence: its first missile release event (ms).
    release_ms: Vec<Option<u32>>,
    /// Per sequence: its reported events (ms, identifier), in time order.
    action_events: Vec<Vec<(u32, [u8; 4])>>,
    /// Reported events action clips passed, not yet taken.
    fired_events: Vec<[u8; 4]>,
    /// Locomotion is standing still, so an action also drives the legs.
    legs_free: bool,
    /// The unit's current ground speed for its movement clip, yd/s.
    locomotion_speed: Option<f32>,
    /// The next `update_locomotion` may enter or leave a stand state pose through its
    /// down or up clip.
    pose_transition: bool,
}

impl AnimationState {
    pub fn new(model: &m2::Model) -> Result<Self, String> {
        if model.sequences.is_empty() || model.bones.len() != model.bone_tracks.len() {
            return Err("M2 animation requires sequences and a track for each bone".into());
        }
        let globals = model.global_sequences.len();
        let unknown_global = |global: i16| usize::try_from(global).is_ok_and(|g| g >= globals);
        if model.bone_tracks.iter().any(|track| {
            unknown_global(track.translation.global_sequence)
                || unknown_global(track.rotation.global_sequence)
                || unknown_global(track.scale.global_sequence)
        }) {
            return Err(format!(
                "M2 bone track names a global sequence beyond the model's {globals}"
            ));
        }
        let local_pivots = model
            .bones
            .iter()
            .map(|bone| {
                let pivot = wow_vec3(bone.pivot);
                let parent = bone.parent_bone_id;
                if parent < 0 {
                    pivot
                } else {
                    pivot - wow_vec3(model.bones[parent as usize].pivot)
                }
            })
            .collect();
        let sequence_animated = (0..model.sequences.len())
            .map(|index| {
                model.bone_tracks.iter().any(|track| {
                    keyframed(&track.translation, index)
                        || keyframed(&track.rotation, index)
                        || keyframed(&track.scale, index)
                })
            })
            .collect();
        Ok(Self {
            sequences: model.sequences.clone(),
            tracks: model.bone_tracks.clone(),
            global_sequences: model.global_sequences.clone(),
            global_ms: 0.0,
            local_pivots,
            current: model
                .sequences
                .iter()
                .position(|sequence| sequence.id == 0)
                .unwrap_or(0),
            time_ms: 0.0,
            looping: true,
            transition: None,
            random_state: 0,
            sequence_animated,
            action: None,
            upper_body: action::upper_body_bones(model),
            release_ms: action::missile_release_times(model),
            action_events: action::reported_events(model),
            fired_events: Vec::new(),
            legs_free: true,
            locomotion_speed: None,
            pose_transition: false,
        })
    }

    /// Current selected clip and clock, not the outgoing crossfade clip.
    fn footstep_phase(&self) -> (usize, u16, f32, f32) {
        let sequence = &self.sequences[self.current];
        (
            self.current,
            sequence.id,
            sequence.duration as f32,
            self.time_ms as f32,
        )
    }

    /// Whether the sampled pose can change as time advances: a crossfade, or a
    /// current sequence with keyframed motion. Static props hold one pose.
    pub fn pose_varies(&self) -> bool {
        self.transition.is_some() || self.action.is_some() || self.sequence_animated[self.current]
    }

    /// Timeline and time a track samples: a global-sequence track reads timeline 0
    /// at the global clock wrapped to its global sequence (WebWowViewerCpp
    /// animate.h `animateTrack`), others the given sequence at its time.
    fn track_time<T>(
        &self,
        track: &m2_anim::AnimTrack<T>,
        index: usize,
        time_ms: f64,
    ) -> (usize, u32) {
        let Ok(global) = usize::try_from(track.global_sequence) else {
            return (index, time_ms as u32);
        };
        let duration = f64::from(self.global_sequences[global]);
        if duration == 0.0 {
            (0, 0)
        } else {
            (0, (self.global_ms % duration) as u32)
        }
    }

    fn sample_sequence(&self, index: usize, time_ms: f64) -> Vec<BonePose> {
        self.tracks
            .iter()
            .zip(&self.local_pivots)
            .map(|(track, &pivot)| {
                let (timeline, time) = self.track_time(&track.translation, index, time_ms);
                let translation = m2_anim::evaluate_vec3_track(&track.translation, timeline, time)
                    .map(wow_vec3)
                    .unwrap_or(Vector3::ZERO);
                let (timeline, time) = self.track_time(&track.rotation, index, time_ms);
                let rotation = m2_anim::evaluate_rotation_track(&track.rotation, timeline, time)
                    .map(|q| Quaternion::new(q[0], q[1], q[2], q[3]).normalized())
                    .unwrap_or(Quaternion::IDENTITY);
                let (timeline, time) = self.track_time(&track.scale, index, time_ms);
                let scale = m2_anim::evaluate_vec3_track(&track.scale, timeline, time)
                    .map(|s| Vector3::new(s[0], s[2], s[1]))
                    .unwrap_or(Vector3::ONE);
                BonePose {
                    position: pivot + translation,
                    rotation,
                    scale,
                }
            })
            .collect()
    }

    fn sampled_poses(&self) -> Vec<BonePose> {
        self.apply_action(self.sampled_base_poses())
    }

    fn sampled_base_poses(&self) -> Vec<BonePose> {
        let current = self.sample_sequence(self.current, self.time_ms);
        let Some(transition) = &self.transition else {
            return current;
        };
        let weight = (transition.elapsed_ms / transition.duration_ms).clamp(0.0, 1.0);
        let outgoing = match &transition.outgoing {
            Outgoing::Sequence { index, time_ms } => self.sample_sequence(*index, *time_ms),
            Outgoing::Snapshot(poses) => poses.clone(),
        };
        outgoing
            .into_iter()
            .zip(current)
            .map(|(from, to)| from.blend(to, weight))
            .collect()
    }

    /// Local Godot bone poses, including rest-relative pivot translations.
    #[cfg(test)]
    pub fn poses(&self) -> Vec<Transform3D> {
        self.sampled_poses()
            .into_iter()
            .map(BonePose::transform)
            .collect()
    }

    pub fn select(&mut self, index: usize, looping: bool) -> Result<(), String> {
        if self.sequences.get(index).is_none() {
            return Err(format!("M2 sequence index {index} is out of range"));
        }
        if self.current == index {
            self.looping = looping;
            return Ok(());
        }
        self.start_transition(index, looping);
        Ok(())
    }

    pub fn select_animation_id(&mut self, id: u16, looping: bool) -> Result<bool, String> {
        let index = self
            .sequences
            .iter()
            .position(|sequence| sequence.id == id && sequence.variation_id == 0)
            .ok_or_else(|| format!("M2 animation ID {id} has no base variation"))?;
        if self.sequences[self.current].id == id && self.looping == looping {
            return Ok(false);
        }
        self.start_transition(index, looping);
        Ok(true)
    }

    fn has_base_sequence(&self, id: u16) -> bool {
        self.sequences
            .iter()
            .any(|sequence| sequence.id == id && sequence.variation_id == 0)
    }

    /// Select local movement through JumpStart → Jump → landing, and into and out of a
    /// stand state pose through its down and up clips, holding each non-looping clip to
    /// completion; moving cuts a pose transition short. Call after advancing animation
    /// time.
    pub fn update_locomotion(
        &mut self,
        movement_id: u16,
        jumping: bool,
        running_forward: bool,
    ) -> Result<bool, String> {
        self.set_locomotion_stationary(movement_id, jumping);
        let current_id = self.sequences[self.current].id;
        let finished = self.time_ms >= f64::from(self.sequences[self.current].duration);
        let armed = std::mem::take(&mut self.pose_transition);
        if !jumping
            && let Some(selected) = self.update_pose(current_id, movement_id, finished, armed)
        {
            return selected;
        }
        match current_id {
            37 if finished => self.select_animation_id(38, true),
            37 => Ok(false),
            38 if !jumping => {
                let landing_id = if running_forward
                    && self
                        .sequences
                        .iter()
                        .any(|sequence| sequence.id == 187 && sequence.variation_id == 0)
                {
                    187
                } else {
                    39
                };
                self.select_animation_id(landing_id, false)
            }
            38 => Ok(false),
            39 | 187 if finished => self.select_animation_id(movement_id, true),
            39 | 187 => Ok(false),
            _ if jumping => self.select_animation_id(37, false),
            // A corpse pose whose Dead clip falls back to Death lies at Death's end.
            _ => self.select_animation_id(movement_id, movement_id != ANIM_DEATH),
        }
    }

    /// The pose transition `update_locomotion` selects, if any: a down clip held until
    /// its pose loop, an up clip held while the unit stands still, and, when `armed` by
    /// a stand state change, the up clip on leaving a pose for Stand or the down clip on
    /// entering a pose.
    fn update_pose(
        &mut self,
        current_id: u16,
        movement_id: u16,
        finished: bool,
        armed: bool,
    ) -> Option<Result<bool, String>> {
        if let Some(clips) = pose_clips(|clips| clips.down == current_id)
            && movement_id == clips.pose
        {
            return Some(if finished {
                self.select_animation_id(clips.pose, true)
            } else {
                Ok(false)
            });
        }
        if pose_clips(|clips| clips.up == current_id).is_some()
            && movement_id == ANIM_STAND
            && !finished
        {
            return Some(Ok(false));
        }
        if !armed {
            return None;
        }
        if let Some(clips) = pose_clips(|clips| clips.pose == current_id)
            && movement_id == ANIM_STAND
            && self.has_base_sequence(clips.up)
        {
            return Some(self.select_animation_id(clips.up, false));
        }
        if let Some(clips) = pose_clips(|clips| clips.pose == movement_id)
            && current_id != clips.pose
            && self.has_base_sequence(clips.down)
        {
            return Some(self.select_animation_id(clips.down, false));
        }
        None
    }

    fn play_death(&mut self) {
        self.action = None;
        if let Some(index) = self
            .sequences
            .iter()
            .position(|sequence| sequence.id == ANIM_DEATH)
        {
            self.start_transition(index, false);
        }
    }

    fn start_transition(&mut self, index: usize, looping: bool) {
        let outgoing = if self.transition.is_some() {
            Outgoing::Snapshot(self.sampled_base_poses())
        } else {
            Outgoing::Sequence {
                index: self.current,
                time_ms: self.time_ms,
            }
        };
        self.transition = Some(Transition {
            outgoing,
            elapsed_ms: 0.0,
            duration_ms: (self.sequences[index].blend_time as f32).max(MIN_MOVEMENT_BLEND_MS),
        });
        self.current = index;
        self.time_ms = 0.0;
        self.looping = looping;
    }

    /// The speed the unit moves at, which scales its movement clip's playback
    /// (`locomotion_playback_rate`); `None` plays clips at their authored rate.
    pub fn set_locomotion_speed(&mut self, speed: Option<f32>) {
        self.locomotion_speed = speed;
    }

    /// The current sequence's playback rate: 1 unless a paced movement clip.
    pub fn playback_rate(&self) -> f32 {
        locomotion_playback_rate(
            self.sequences[self.current].movespeed,
            self.locomotion_speed,
        )
    }

    /// Sample an explicitly forced sky phase without ordinary clip advancement.
    pub(crate) fn seek_fixed_time_ms(&mut self, time_ms: f64) -> Result<(), String> {
        if !time_ms.is_finite() || time_ms < 0.0 {
            return Err("Fixed sky animation time must be finite and nonnegative".into());
        }
        self.time_ms = time_ms;
        self.transition = None;
        Ok(())
    }

    pub fn advance(&mut self, delta_ms: f64) -> Result<(), String> {
        let mut state = self.random_state;
        let rate = self.playback_rate();
        let result = self.advance_with_roll(delta_ms * f64::from(rate), |upper| {
            sample_roll(&mut state, upper)
        });
        self.random_state = state;
        result?;
        // Global sequences run at real time, not the clip's playback rate
        // (WebWowViewerCpp animationManager.cpp updateSequencing `deltaTimeForGS`).
        self.global_ms += delta_ms;
        self.tick_action(delta_ms);
        Ok(())
    }

    fn advance_with_roll(
        &mut self,
        delta_ms: f64,
        mut roll: impl FnMut(u32) -> u32,
    ) -> Result<(), String> {
        if !delta_ms.is_finite() || delta_ms < 0.0 || delta_ms > f32::MAX as f64 {
            return Err(format!("Invalid M2 elapsed animation time {delta_ms}"));
        }
        let duration = f64::from(self.sequences[self.current].duration);
        let elapsed = self.time_ms + delta_ms;
        if !elapsed.is_finite() {
            return Err("M2 animation time overflow".into());
        }
        if !self.looping || duration == 0.0 || elapsed < duration {
            self.time_ms = if duration > 0.0 {
                elapsed.min(duration)
            } else {
                0.0
            };
            self.tick_transition(delta_ms);
            return Ok(());
        }
        let family = m2::VariationFamily::read(&self.sequences, self.current)?;
        if family.is_single() {
            self.time_ms = elapsed % duration;
            self.tick_transition(delta_ms);
            return Ok(());
        }
        family.validate_elapsed(elapsed)?;
        let mut remaining = delta_ms;
        loop {
            let duration = f64::from(self.sequences[self.current].duration);
            let until_boundary = (duration - self.time_ms).max(0.0);
            if remaining < until_boundary {
                self.time_ms += remaining;
                self.tick_transition(remaining);
                return Ok(());
            }
            remaining -= until_boundary;
            self.time_ms = duration;
            self.tick_transition(until_boundary);
            let next = family.choose(&mut roll)?;
            if next != self.current {
                self.select(next, true)?;
            }
            self.time_ms = 0.0;
            if remaining == 0.0 {
                return Ok(());
            }
        }
    }

    fn tick_transition(&mut self, delta_ms: f64) {
        let Some(transition) = &mut self.transition else {
            return;
        };
        transition.elapsed_ms += delta_ms as f32;
        if let Outgoing::Sequence { index, time_ms } = &mut transition.outgoing {
            let source_duration = f64::from(self.sequences[*index].duration);
            *time_ms = (*time_ms + delta_ms).min(source_duration);
        }
        if transition.elapsed_ms >= transition.duration_ms {
            self.transition = None;
        }
    }
}

/// Godot node bound to one model's native Skeleton3D, with explicit deterministic ticking.
#[derive(GodotClass)]
#[class(base = Node)]
pub struct WowAnimationPlayer {
    base: Base<Node>,
    animation: Option<AnimationState>,
    skeleton: Option<Gd<Skeleton3D>>,
    paused: bool,
    /// Whether advancing writes the sampled pose (NPC animation LOD).
    sampling: bool,
    /// A pose change was skipped while not sampling.
    stale: bool,
    /// Some bone track changes over time, or a billboard follows the camera; a static
    /// model keeps its first pose and never processes.
    animates: bool,
    billboards: Option<billboard::Billboards>,
    /// The sequence whose bounds the sibling skinned batches cull by.
    bounds_sequence: Option<usize>,
}

#[godot_api]
impl INode for WowAnimationPlayer {
    fn init(base: Base<Node>) -> Self {
        Self {
            base,
            animation: None,
            skeleton: None,
            paused: false,
            sampling: true,
            stale: false,
            animates: true,
            billboards: None,
            bounds_sequence: None,
        }
    }

    fn ready(&mut self) {
        if !self.animates {
            self.base_mut().set_process(false);
        }
    }

    fn process(&mut self, delta: f64) {
        self.advance_time_ms(delta * 1000.0);
    }
}

impl WowAnimationPlayer {
    pub fn from_model(model: &m2::Model, skeleton: Gd<Skeleton3D>) -> Result<Gd<Self>, String> {
        let animation = AnimationState::new(model)?;
        let billboards = billboard::Billboards::new(model);
        if skeleton.get_bone_count() as usize != model.bones.len() {
            return Err(format!(
                "M2 animation has {} bones but Skeleton3D has {}",
                model.bones.len(),
                skeleton.get_bone_count()
            ));
        }
        let mut player = Gd::<Self>::from_init_fn(|base| Self {
            base,
            animation: Some(animation),
            skeleton: Some(skeleton),
            paused: false,
            sampling: true,
            stale: false,
            animates: !m2::bones_are_static(model) || billboards.is_some(),
            billboards,
            bounds_sequence: None,
        });
        player.set_name("WowAnimationPlayer");
        player.bind_mut().write_poses();
        Ok(player)
    }

    pub fn animates(&self) -> bool {
        self.animates
    }

    fn validated_bone_count(&self, label: &str) -> Result<usize, String> {
        let bones = self
            .skeleton
            .as_ref()
            .ok_or_else(|| format!("{label} M2 animation has no bound Skeleton3D"))?
            .get_bone_count() as usize;
        let tracks = self
            .animation
            .as_ref()
            .ok_or_else(|| format!("{label} M2 animation has no bound model"))?
            .tracks
            .len();
        if bones != tracks {
            return Err(format!(
                "{label} M2 animation bone mismatch: Skeleton3D has {bones}, model has {tracks} tracks"
            ));
        }
        Ok(bones)
    }

    pub(crate) fn transfer_playback_from(&mut self, previous: &mut Self) -> Result<(), String> {
        let previous_bones = previous.validated_bone_count("Previous")?;
        let new_bones = self.validated_bone_count("New")?;
        if previous_bones != new_bones {
            return Err(format!(
                "M2 playback transfer bone mismatch: previous has {previous_bones}, new has {new_bones}"
            ));
        }
        self.animation = previous.animation.take();
        self.paused = previous.paused;
        self.write_poses();
        Ok(())
    }

    pub(crate) fn update_locomotion(
        &mut self,
        movement_id: u16,
        jumping: bool,
        running_forward: bool,
    ) -> Result<(), String> {
        let changed = self
            .animation
            .as_mut()
            .ok_or_else(|| "M2 animation has no bound model".to_string())?
            .update_locomotion(movement_id, jumping, running_forward)?;
        if changed {
            self.write_poses();
        }
        Ok(())
    }

    /// The next `update_locomotion` plays the down or up clip into or out of a stand
    /// state pose: the unit's stand state just changed (a unit first seen in a pose
    /// holds it without one).
    pub(crate) fn arm_pose_transition(&mut self) {
        if let Some(animation) = self.animation.as_mut() {
            animation.pose_transition = true;
        }
    }

    pub(crate) fn playback_rate(&self) -> Option<f32> {
        self.animation.as_ref().map(AnimationState::playback_rate)
    }

    pub(crate) fn set_locomotion_speed(&mut self, speed: Option<f32>) {
        if let Some(animation) = self.animation.as_mut() {
            animation.set_locomotion_speed(speed);
        }
    }

    /// Play combat/spell clip `id` (or its `AnimationData` fallback) over locomotion,
    /// once or held while `looping`. `Ok(None)`: the model has no such clip.
    pub(crate) fn play_action(
        &mut self,
        id: u16,
        looping: bool,
        priority: ActionPriority,
        fallbacks: &std::collections::HashMap<u16, u16>,
    ) -> Result<Option<u16>, String> {
        let animation = self
            .animation
            .as_mut()
            .ok_or_else(|| "M2 animation has no bound model".to_string())?;
        let Some(clip) = animation.resolve_clip(id, fallbacks) else {
            return Ok(None);
        };
        if !animation.play_action(clip, looping, priority)? {
            return Ok(None);
        }
        self.write_poses();
        Ok(Some(clip))
    }

    /// The playing sequence index and its time (ms), for keyframed particle emission.
    pub(crate) fn playback(&self) -> Option<(usize, u32)> {
        let animation = self.animation.as_ref()?;
        Some((animation.current, animation.time_ms as u32))
    }

    /// Select model clip `id` (base variation) as its own sequence, crossfading.
    pub(crate) fn play_clip(&mut self, id: u16, looping: bool) -> Result<(), String> {
        let changed = self
            .animation
            .as_mut()
            .ok_or_else(|| "M2 animation has no bound model".to_string())?
            .select_animation_id(id, looping)?;
        if changed {
            self.write_poses();
        }
        Ok(())
    }

    /// `id`, or the first clip of its `AnimationData.Fallback` chain the model has.
    pub(crate) fn resolve_clip(
        &self,
        id: u16,
        fallbacks: &std::collections::HashMap<u16, u16>,
    ) -> Option<u16> {
        self.animation.as_ref()?.resolve_clip(id, fallbacks)
    }

    /// The playing action clip has yet to fire its missile release event.
    pub(crate) fn awaits_missile_release(&self) -> bool {
        self.animation
            .as_ref()
            .is_some_and(AnimationState::awaits_missile_release)
    }

    /// Reported M2 events (`$SCD`) its action clips passed since the last call.
    pub(crate) fn take_fired_events(&mut self) -> Vec<[u8; 4]> {
        self.animation
            .as_mut()
            .map(AnimationState::take_fired_events)
            .unwrap_or_default()
    }

    /// Fade out held action clip `id`.
    pub(crate) fn stop_action(&mut self, id: u16) {
        if let Some(animation) = self.animation.as_mut() {
            animation.stop_action(id);
        }
    }

    /// Selected clip phase for the native local-player footstep observer.
    pub fn footstep_phase(&self) -> Option<(usize, u16, f32, f32)> {
        self.animation.as_ref().map(AnimationState::footstep_phase)
    }

    pub(crate) fn play_death(&mut self) -> Result<(), String> {
        self.animation
            .as_mut()
            .ok_or_else(|| "M2 animation has no bound model".to_string())?
            .play_death();
        self.write_poses();
        Ok(())
    }

    /// When off, advancing keeps the clock, sequence and crossfade running but writes
    /// no bone pose; the next sampled advance writes any change it skipped.
    pub(crate) fn set_sampling(&mut self, sampling: bool) {
        self.sampling = sampling;
    }

    pub(crate) fn seek_fixed_time_ms(&mut self, time_ms: f64) -> Result<(), String> {
        self.validated_bone_count("Fixed sky animation time")?;
        let animation = self
            .animation
            .as_mut()
            .ok_or("Fixed sky animation has no state")?;
        animation.seek_fixed_time_ms(time_ms)?;
        self.write_poses();
        self.stale = false;
        Ok(())
    }

    fn write_poses(&mut self) {
        let (Some(animation), Some(skeleton)) = (&self.animation, &mut self.skeleton) else {
            return;
        };
        let mut poses = animation.sampled_poses();
        if let Some(billboards) = &self.billboards {
            billboards.apply(&mut poses, skeleton);
        }
        for (index, pose) in poses.into_iter().enumerate() {
            skeleton.set_bone_pose_position(index as i32, pose.position);
            skeleton.set_bone_pose_rotation(index as i32, pose.rotation);
            skeleton.set_bone_pose_scale(index as i32, pose.scale);
        }
        self.apply_sequence_bounds();
    }

    /// Sibling skinned batches cull by the playing sequence's `M2Sequence.bounds`, set on
    /// every sequence change like WebWowViewerCpp `M2Object` (`isNeedUpdateBB`,
    /// `getAnimatinonBB`); an empty box keeps the previous one.
    pub(crate) fn apply_sequence_bounds(&mut self) {
        let Some(animation) = &self.animation else {
            return;
        };
        let current = animation.current;
        if self.bounds_sequence == Some(current) {
            return;
        }
        let bounds = sequence_bounds(&animation.sequences[current]);
        let Some(parent) = self.base().get_parent() else {
            return;
        };
        self.bounds_sequence = Some(current);
        let Some(bounds) = bounds else {
            return;
        };
        for child in parent.get_children().iter_shared() {
            if let Ok(mut batch) = child.try_cast::<MeshInstance3D>()
                && batch.get_skin().is_some()
            {
                batch.set_custom_aabb(bounds);
            }
        }
    }
}

/// A sequence's bounds in engine axes, unless empty (WebWowViewerCpp `m2Object.cpp`
/// `boundsValid`: min <= max and a diagonal over 0.001 squared).
fn sequence_bounds(sequence: &m2::Sequence) -> Option<Aabb> {
    let [min, max] = sequence.bounds;
    let ordered = (0..3).all(|axis| min[axis] <= max[axis]);
    let diagonal: f32 = (0..3).map(|axis| (max[axis] - min[axis]).powi(2)).sum();
    (ordered && diagonal > 0.001).then(|| crate::assets::wow_aabb(min, max))
}

#[godot_api]
impl WowAnimationPlayer {
    #[func]
    fn current_animation_id(&self) -> i32 {
        self.footstep_phase()
            .map(|(_, id, _, _)| i32::from(id))
            .unwrap_or(-1)
    }

    /// The combat/spell clip layered over locomotion, or -1.
    pub(crate) fn action_id(&self) -> i32 {
        self.current_action_id()
    }

    /// The combat/spell clip layered over locomotion, or -1.
    #[func]
    fn current_action_id(&self) -> i32 {
        self.animation
            .as_ref()
            .and_then(AnimationState::action_id)
            .map(i32::from)
            .unwrap_or(-1)
    }

    #[func]
    fn play_sequence(&mut self, index: i32, looping: bool) -> bool {
        let result = usize::try_from(index)
            .map_err(|_| format!("Invalid M2 sequence index {index}"))
            .and_then(|index| {
                self.animation
                    .as_mut()
                    .ok_or_else(|| "M2 animation has no bound model".to_string())?
                    .select(index, looping)
            });
        if let Err(error) = result {
            godot_error!("{error}");
            return false;
        }
        self.write_poses();
        true
    }

    /// Play model clip `id` (base variation), crossfading as gameplay does.
    #[func]
    fn play_animation(&mut self, id: i32, looping: bool) -> bool {
        let played = u16::try_from(id)
            .map_err(|_| format!("Invalid animation ID {id}"))
            .and_then(|id| self.play_clip(id, looping));
        if let Err(error) = played {
            godot_error!("{error}");
            return false;
        }
        true
    }

    #[func]
    pub(crate) fn advance_time_ms(&mut self, delta_ms: f64) -> bool {
        if self.paused {
            return true;
        }
        let result = self
            .animation
            .as_mut()
            .ok_or("M2 animation has no bound model".to_string())
            .and_then(|animation| {
                // A pose that varied before or after this step must be rewritten;
                // static props keep the pose already on the skeleton.
                let varied = animation.pose_varies();
                animation.advance(delta_ms)?;
                Ok(varied || animation.pose_varies() || self.billboards.is_some())
            });
        match result {
            Ok(changed) => {
                if pose_write_due(changed, self.sampling, &mut self.stale) {
                    self.write_poses();
                }
            }
            Err(error) => {
                godot_error!("{error}");
                return false;
            }
        }
        true
    }

    #[func]
    fn set_paused(&mut self, paused: bool) {
        self.paused = paused;
    }
}

/// Whether an advance whose pose `changed` writes it; a change skipped while not
/// `sampling` stays due until the next sampled advance.
fn pose_write_due(changed: bool, sampling: bool, stale: &mut bool) -> bool {
    let due = changed || *stale;
    *stale = due && !sampling;
    due && sampling
}

#[cfg(test)]
mod action_tests;

#[cfg(test)]
mod global_sequence_tests;

#[cfg(test)]
mod jump_tests;

#[cfg(test)]
mod npc_locomotion_tests;

#[cfg(test)]
mod npc_pose_tests;
#[cfg(test)]
mod stand_pose_tests;

#[cfg(test)]
mod sampling_tests {
    use super::pose_write_due;

    #[test]
    fn skipped_changes_are_written_on_the_next_sampled_advance() {
        let mut stale = false;
        assert!(pose_write_due(true, true, &mut stale));
        // Frozen: the pose moves, nothing is written.
        assert!(!pose_write_due(true, false, &mut stale));
        // The sequence settled while frozen; the missed final pose is still written.
        assert!(!pose_write_due(false, false, &mut stale));
        assert!(pose_write_due(false, true, &mut stale));
        assert!(!pose_write_due(false, true, &mut stale));
    }
}

#[cfg(test)]
mod tests {
    use super::AnimationState;
    use game_engine_core::m2;
    use godot::builtin::{Basis, Quaternion, Transform3D, Vector3};
    use std::{fs, path::PathBuf};

    fn model() -> m2::Model {
        let root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../data/models");
        let read = |name| fs::read(root.join(name)).expect("authored fixture");
        m2::parse_model_with_skeleton(
            &read("humanmale_hd.m2"),
            &read("humanmale_hd00.skin"),
            Some(&read("humanmale_hd.skel")),
            |fdid| fs::read(root.join(format!("{fdid}.anim"))).ok(),
        )
        .expect("HD model with authored tracks")
    }

    #[test]
    fn fixed_sky_zero_duration_keeps_requested_sampled_pose() {
        let mut model = model();
        model.sequences.truncate(1);
        model.sequences[0].id = 0;
        model.sequences[0].duration = 0;
        model.bones.truncate(1);
        model.bones[0].parent_bone_id = -1;
        model.bones[0].pivot = [0.0; 3];
        let mut tracks = model.bone_tracks.as_ref().clone();
        tracks.truncate(1);
        tracks[0].translation = m2::AnimTrack {
            interpolation_type: 1,
            global_sequence: -1,
            sequences: vec![(vec![0, 2000], vec![[0.0; 3], [20.0, 0.0, 0.0]])],
        };
        model.bone_tracks = std::sync::Arc::new(tracks);
        let mut player = AnimationState::new(&model).unwrap();
        player.seek_fixed_time_ms(1234.0).unwrap();
        let sampled = player.poses()[0].origin.x;
        assert!((sampled - 12.34).abs() < 0.00001, "sampled={sampled}");
    }

    #[test]
    fn fixed_sky_seek_rejects_invalid_time_without_pose_mutation() {
        let mut player = AnimationState::new(&model()).unwrap();
        player.seek_fixed_time_ms(1234.0).unwrap();
        let before = player.poses();
        for invalid in [f64::NAN, f64::INFINITY, -1.0] {
            assert!(player.seek_fixed_time_ms(invalid).is_err());
            assert_eq!(player.poses(), before);
        }
    }

    fn basis(v: [f32; 3]) -> Vector3 {
        Vector3::new(v[0], v[2], -v[1])
    }

    fn near(a: Vector3, b: Vector3) -> bool {
        a.distance_to(b) < 0.0001
    }

    fn near_pose(a: Transform3D, b: Transform3D) -> bool {
        near(a.origin, b.origin)
            && a.basis
                .rows
                .iter()
                .zip(b.basis.rows)
                .all(|(a, b)| near(*a, b))
    }

    #[test]
    fn active_phase_reports_selected_sequence_and_advanced_time() {
        let model = model();
        let mut player = AnimationState::new(&model).expect("animated model");
        player.select_animation_id(5, true).expect("Run ID");
        player.advance(75.0).expect("advance Run");
        assert_eq!(player.footstep_phase(), (2, 5, 667.0, 75.0));
    }

    #[test]
    fn authored_id_run_selects_base_sequence_and_repeated_request_keeps_time() {
        let model = model();
        assert_eq!(model.sequences[2].id, 5);
        assert_eq!(model.sequences[2].variation_id, 0);
        assert_eq!(model.sequences[2].duration, 667);
        let mut player = AnimationState::new(&model).expect("animated model");
        assert!(
            !player
                .select_animation_id(0, true)
                .expect("already standing")
        );
        assert!(player.select_animation_id(5, true).expect("Run ID"));
        assert_eq!(player.current, 2);
        player.advance(75.0).expect("advance Run crossfade");
        let before = player.poses();
        let blend_elapsed = player.transition.as_ref().expect("crossfade").elapsed_ms;
        assert!(!player.select_animation_id(5, true).expect("same Run ID"));
        assert_eq!(player.current, 2);
        assert_eq!(player.time_ms, 75.0);
        assert_eq!(
            player.transition.as_ref().expect("crossfade").elapsed_ms,
            blend_elapsed
        );
        assert!(
            before
                .iter()
                .zip(player.poses())
                .all(|(a, b)| near_pose(*a, b))
        );
    }

    #[test]
    fn authored_id_repeated_stand_preserves_selected_variation_and_crossfade() {
        let model = model();
        for index in [0, 26, 217, 376] {
            assert_eq!(model.sequences[index].id, 0);
        }
        let mut player = AnimationState::new(&model).expect("animated model");
        let duration = f64::from(model.sequences[0].duration);
        let next_variation_roll = model.sequences[0].frequency as u32;
        player
            .advance_with_roll(duration, |_| next_variation_roll)
            .expect("authored Stand variation boundary");
        assert_eq!(player.current, 26);
        player.advance(40.0).expect("advance variation crossfade");
        let before = player.poses();
        let blend_elapsed = player.transition.as_ref().expect("crossfade").elapsed_ms;
        assert!(
            !player
                .select_animation_id(0, true)
                .expect("same Stand family")
        );
        assert_eq!(player.current, 26);
        assert_eq!(player.time_ms, 40.0);
        assert_eq!(
            player.transition.as_ref().expect("crossfade").elapsed_ms,
            blend_elapsed
        );
        assert!(
            before
                .iter()
                .zip(player.poses())
                .all(|(a, b)| near_pose(*a, b))
        );
    }

    #[test]
    fn authored_id_missing_does_not_mutate_playback() {
        let model = model();
        assert!(
            !model
                .sequences
                .iter()
                .any(|sequence| sequence.id == u16::MAX)
        );
        let mut player = AnimationState::new(&model).expect("animated model");
        player.select_animation_id(5, true).expect("Run ID");
        player.advance(75.0).expect("advance crossfade");
        let before = player.poses();
        let elapsed = player.transition.as_ref().expect("crossfade").elapsed_ms;
        assert!(player.select_animation_id(u16::MAX, false).is_err());
        assert_eq!(player.current, 2);
        assert!(player.looping);
        assert_eq!(player.time_ms, 75.0);
        assert_eq!(
            player.transition.as_ref().expect("crossfade").elapsed_ms,
            elapsed
        );
        assert!(
            before
                .iter()
                .zip(player.poses())
                .all(|(a, b)| near_pose(*a, b))
        );
    }

    #[test]
    fn authored_id_loop_mode_change_and_interruption_keep_blended_pose() {
        let model = model();
        let mut player = AnimationState::new(&model).expect("animated model");
        player.advance(1000.0).expect("advance Stand");
        let before = player.poses();
        assert!(
            player
                .select_animation_id(0, false)
                .expect("change loop mode")
        );
        assert!(!player.looping);
        assert_eq!(player.time_ms, 0.0);
        assert!(player.transition.as_ref().expect("crossfade").duration_ms >= 150.0);
        assert!(
            before
                .iter()
                .zip(player.poses())
                .all(|(a, b)| near_pose(*a, b))
        );
        player.advance(75.0).expect("advance first crossfade");
        let midblend = player.poses();
        assert!(
            player
                .select_animation_id(5, true)
                .expect("interrupt with Run")
        );
        assert_eq!(player.current, 2);
        assert!(player.looping);
        assert_eq!(player.time_ms, 0.0);
        assert!(player.transition.as_ref().expect("crossfade").duration_ms >= 150.0);
        assert!(
            midblend
                .iter()
                .zip(player.poses())
                .all(|(a, b)| near_pose(*a, b))
        );
    }

    #[test]
    fn hd_stand_samples_authored_bone_and_local_pivot() {
        let model = model();
        let mut player = AnimationState::new(&model).expect("animated model");
        let initial = player.poses();
        player.advance(1000.0).expect("valid time");
        let sampled = player.poses();
        assert_eq!(sampled.len(), 216);
        assert_eq!(model.sequences[0].id, 0);
        assert_eq!(model.sequences[0].duration, 2667);
        let local_pivot = basis(model.bones[1].pivot) - basis(model.bones[0].pivot);
        let translated = Vector3::new(-0.011336661, -0.0016739104, -0.003007821);
        assert!(near(sampled[1].origin, local_pivot + translated));
        assert!(!near_pose(sampled[1], initial[1]));
        let authored_rotation =
            Quaternion::new(-0.024109622, 0.04464858, 0.12179937, 0.9912412).normalized();
        assert!(near(
            sampled[6].basis.rows[0],
            Basis::from_quaternion(authored_rotation).rows[0]
        ));
    }

    fn compose_godot_skin(index: usize, model: &m2::Model, poses: &[Transform3D]) -> Transform3D {
        let parent = model.bones[index].parent_bone_id;
        let parent_global = if parent < 0 {
            Transform3D::IDENTITY
        } else {
            compose_godot_global(parent as usize, model, poses)
        };
        let global = parent_global * poses[index];
        global * Transform3D::IDENTITY.translated(-basis(model.bones[index].pivot))
    }

    fn compose_godot_global(index: usize, model: &m2::Model, poses: &[Transform3D]) -> Transform3D {
        let parent = model.bones[index].parent_bone_id;
        if parent < 0 {
            poses[index]
        } else {
            compose_godot_global(parent as usize, model, poses) * poses[index]
        }
    }

    fn compose_bevy_skin(index: usize, model: &m2::Model, poses: &[Transform3D]) -> Transform3D {
        let bone = &model.bones[index];
        let absolute_pivot = basis(bone.pivot);
        let local_pivot = if bone.parent_bone_id < 0 {
            absolute_pivot
        } else {
            absolute_pivot - basis(model.bones[bone.parent_bone_id as usize].pivot)
        };
        let raw_translation = poses[index].origin - local_pivot;
        let bevy_origin = raw_translation + absolute_pivot - poses[index].basis * absolute_pivot;
        let local = Transform3D::new(poses[index].basis, bevy_origin);
        if bone.parent_bone_id < 0 {
            local
        } else {
            compose_bevy_skin(bone.parent_bone_id as usize, model, poses) * local
        }
    }

    #[test]
    fn authored_parent_child_rotations_match_bevy_pivot_skin_matrices() {
        let model = model();
        let mut player = AnimationState::new(&model).expect("animated model");
        player.advance(1000.0).expect("authored Stand at 1000ms");
        let poses = player.poses();
        let child = model
            .bones
            .iter()
            .enumerate()
            .find(|(index, bone)| {
                bone.parent_bone_id >= 0
                    && bone.pivot != [0.0; 3]
                    && !near(poses[*index].basis.rows[0], Vector3::RIGHT)
            })
            .map(|(index, _)| index)
            .expect("HD rotating child with a nonzero pivot");
        let parent = model.bones[child].parent_bone_id as usize;
        for index in [parent, child] {
            let native = compose_godot_skin(index, &model, &poses);
            let original = compose_bevy_skin(index, &model, &poses);
            assert!(
                near_pose(native, original),
                "bone {index} differs in skinning basis"
            );
        }
    }

    #[test]
    fn retransition_starts_at_blended_pose_and_rejects_invalid_time_and_index() {
        let model = model();
        let mut player = AnimationState::new(&model).expect("animated model");
        player.advance(1000.0).expect("idle playback");
        player.select(4, true).expect("authored clip");
        let at_start = player.poses();
        player.advance(75.0).expect("half of minimum blend");
        let midblend = player.poses();
        assert!(
            at_start
                .iter()
                .zip(&midblend)
                .any(|(a, b)| !near_pose(*a, *b))
        );
        player.select(1, true).expect("interrupt blend");
        assert!(
            midblend
                .iter()
                .zip(player.poses())
                .all(|(a, b)| near_pose(*a, b))
        );
        assert!(player.select(usize::MAX, true).is_err());
        assert!(player.advance(-1.0).is_err());
        assert!(
            midblend
                .iter()
                .zip(player.poses())
                .all(|(a, b)| near_pose(*a, b))
        );
    }

    #[test]
    fn death_restarts_from_zero_with_authored_crossfade_and_holds_last_pose() {
        let model = model();
        let death = model
            .sequences
            .iter()
            .position(|sequence| sequence.id == 1)
            .expect("authored Death");
        let mut player = AnimationState::new(&model).expect("animated model");
        player.select(death, true).expect("select Death initially");
        player.advance(300.0).expect("advance Death");
        let outgoing = player.poses();
        player.play_death();
        assert_eq!(player.current, death);
        assert_eq!(player.time_ms, 0.0);
        assert!(!player.looping);
        let transition = player.transition.as_ref().expect("Death crossfade");
        assert_eq!(
            transition.duration_ms,
            (model.sequences[death].blend_time as f32).max(150.0)
        );
        assert!(
            outgoing
                .iter()
                .zip(player.poses())
                .all(|(a, b)| near_pose(*a, b))
        );
        player
            .advance(model.sequences[death].duration as f64 + 500.0)
            .expect("finish Death");
        assert_eq!(player.time_ms, model.sequences[death].duration as f64);
        assert!(player.transition.is_none());
        let held = player.poses();
        player.advance(500.0).expect("hold corpse pose");
        assert!(
            held.iter()
                .zip(player.poses())
                .all(|(a, b)| near_pose(*a, b))
        );
    }

    #[test]
    fn wolf_loop_chooses_authored_weighted_variation_not_next_link_as_time() {
        let root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../data/models");
        let wolf = m2::parse_model(
            &fs::read(root.join("126487.m2")).expect("wolf model"),
            &fs::read(root.join("12648700.skin")).expect("wolf skin"),
        )
        .expect("authored wolf");
        let mut player = AnimationState::new(&wolf).expect("wolf animation");
        player.select(2, true).expect("Stand base variation");
        assert_eq!(wolf.sequences[2].variation_next, 9);
        assert_eq!(wolf.sequences[2].frequency, 30445);
        let duration = wolf.sequences[2].duration as f64;
        player
            .advance_with_roll(duration, |_| 30445)
            .expect("weighted boundary");
        assert_eq!(player.current, 9);
        assert_eq!(player.time_ms, 0.0);
        assert_eq!(wolf.sequences[player.current].id, wolf.sequences[2].id);
    }

    #[test]
    fn outgoing_sequence_advances_during_first_crossfade() {
        let model = model();
        let mut player = AnimationState::new(&model).expect("animated model");
        player.advance(1000.0).expect("idle playback");
        player.select(4, true).expect("authored clip");
        let half = player.transition.as_ref().expect("crossfade").duration_ms / 2.0;
        let source = player.sample_sequence(0, 1000.0 + f64::from(half));
        let target = player.sample_sequence(4, f64::from(half));
        player.advance(f64::from(half)).expect("crossfade playback");
        let actual = player.sampled_poses();
        let moving_bone = model
            .bone_tracks
            .iter()
            .enumerate()
            .find(|(index, _)| {
                !near(
                    source[*index].position,
                    player.sample_sequence(0, 1000.0)[*index].position,
                )
            })
            .map(|(index, _)| index)
            .expect("outgoing idle translates");
        let expected = source[moving_bone]
            .position
            .lerp(target[moving_bone].position, 0.5);
        assert!(near(actual[moving_bone].position, expected));
    }

    #[test]
    fn single_key_sequence_holds_its_pose_and_reports_static() {
        let mut model = model();
        let walk = 1;
        let authored = AnimationState::new(&model).expect("animated model");
        assert!(
            authored.sequence_animated[walk],
            "authored Walk moves bones"
        );
        // Global-sequence tracks (HD bones 186/187/210 scale over the 1518667 ms
        // global sequence 0) play in timeline 0 during Walk too.
        let timeline = |global: i16| if global < 0 { walk } else { 0 };
        for track in std::sync::Arc::make_mut(&mut model.bone_tracks) {
            let rotation = timeline(track.rotation.global_sequence);
            if let Some((times, values)) = track.rotation.sequences.get_mut(rotation) {
                times.truncate(1);
                values.truncate(1);
            }
            for vec3 in [&mut track.translation, &mut track.scale] {
                let index = timeline(vec3.global_sequence);
                if let Some((times, values)) = vec3.sequences.get_mut(index) {
                    times.truncate(1);
                    values.truncate(1);
                }
            }
        }
        let mut player = AnimationState::new(&model).expect("animated model");
        player.select(walk, true).expect("Walk clip");
        assert!(player.pose_varies(), "crossfade from Stand still moves");
        player.advance(1000.0).expect("finish crossfade");
        assert!(!player.pose_varies());
        let held = player.poses();
        player.advance(430.0).expect("advance static clip");
        assert!(
            held.iter()
                .zip(player.poses())
                .all(|(a, b)| near_pose(*a, b))
        );
    }

    #[test]
    fn looping_and_nonlooping_advance_are_deterministic() {
        let model = model();
        let mut looping = AnimationState::new(&model).expect("animated model");
        let mut direct = AnimationState::new(&model).expect("animated model");
        // HD Stand has linked weighted variations; use the authored unlinked Walk clip
        // to assert single-clip modulo independent of random variation selection.
        assert_eq!(model.sequences[1].duration, 1000);
        assert_eq!(model.sequences[1].variation_next, -1);
        looping.select(1, true).expect("Walk clip");
        direct.select(1, true).expect("Walk clip");
        looping.advance(2100.0).expect("two wraps");
        direct.advance(1100.0).expect("one wrap");
        assert!(
            looping
                .poses()
                .iter()
                .zip(direct.poses())
                .all(|(a, b)| near_pose(*a, b))
        );
        direct.select(2, false).expect("nonlooping clip");
        direct.advance(1_000_000.0).expect("clamped endpoint");
        let at_end = direct.poses();
        direct.advance(100.0).expect("hold endpoint");
        assert!(
            at_end
                .iter()
                .zip(direct.poses())
                .all(|(a, b)| near_pose(*a, b))
        );
    }

    /// FDID 588287 pa_redbird_stand.m2: Stand variation 1 is weighted 30583 of
    /// 32767 but lasts 0 ms, so it plays for no time and Stand variation 0 loops.
    #[test]
    fn redbird_zero_duration_stand_variation_keeps_the_bird_animating() {
        let root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../data/models");
        let read = |name| fs::read(root.join(name)).expect("authored fixture");
        let model =
            m2::parse_model(&read("588287.m2"), &read("58828700.skin")).expect("redbird model");
        let stand = &model.sequences;
        assert_eq!(
            [
                (stand[0].duration, stand[0].frequency),
                (stand[1].duration, stand[1].frequency)
            ],
            [(3333, 2184), (0, 30583)]
        );
        let mut player = AnimationState::new(&model).expect("animated model");
        let start = player.poses();
        for roll in [30000, 0, 32766] {
            player
                .advance_with_roll(3333.0, |_| roll)
                .expect("Stand loops past its zero-duration variation");
            assert_eq!(player.current, 0);
            assert_eq!(player.time_ms, 0.0);
        }
        player.advance(1000.0).expect("advance within Stand");
        assert_eq!(player.time_ms, 1000.0);
        assert!(player.pose_varies());
        assert!(
            !start
                .iter()
                .zip(player.poses())
                .all(|(a, b)| near_pose(*a, b))
        );
    }
}

#[cfg(test)]
mod remote_player_tests;
