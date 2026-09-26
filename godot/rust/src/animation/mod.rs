//! Sequence-local M2 bone playback on Godot's native Skeleton3D palette.
//! Poses use local pivots matching the skeleton's rest and absolute inverse binds.
use game_engine_core::{asset::m2_format::m2_anim, m2};
use godot::{
    builtin::{Basis, Quaternion, Transform3D, Vector3},
    classes::{INode, Node, Skeleton3D},
    prelude::*,
};

const MIN_MOVEMENT_BLEND_MS: f32 = 150.0;

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
    fn transform(self) -> Transform3D {
        Transform3D::new(
            Basis::from_quaternion(self.rotation).scaled(self.scale),
            self.position,
        )
    }

    fn blend(self, target: Self, weight: f32) -> Self {
        Self {
            position: self.position.lerp(target.position, weight),
            rotation: self.rotation.slerp(target.rotation, weight),
            scale: self.scale.lerp(target.scale, weight),
        }
    }
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

struct VariationFamily {
    candidates: Vec<(usize, u32)>,
    total: u32,
}

impl VariationFamily {
    fn read(sequences: &[m2::Sequence], current: usize) -> Result<Self, String> {
        let id = sequences[current].id;
        let base = sequences
            .iter()
            .position(|sequence| sequence.id == id && sequence.variation_id == 0)
            .ok_or_else(|| format!("M2 animation {id} lacks base variation"))?;
        let mut candidates = Vec::new();
        let mut seen = std::collections::HashSet::new();
        let mut next = Some(base);
        let mut total = 0u32;
        while let Some(index) = next {
            if !seen.insert(index) {
                return Err(format!("M2 animation {id} has cyclic variations"));
            }
            let sequence = sequences
                .get(index)
                .ok_or_else(|| format!("M2 animation {id} links absent variation {index}"))?;
            if sequence.id != id {
                return Err(format!(
                    "M2 animation {id} links other animation {}",
                    sequence.id
                ));
            }
            let weight = u32::try_from(sequence.frequency)
                .map_err(|_| format!("M2 variation {index} has negative weight"))?;
            if weight > 0 && sequence.duration == 0 {
                return Err(format!("M2 variation {index} has zero duration"));
            }
            total = total
                .checked_add(weight)
                .ok_or("M2 variation weight overflow")?;
            candidates.push((index, weight));
            next = match sequence.variation_next {
                -1 => None,
                index if index >= 0 => Some(index as usize),
                index => return Err(format!("M2 invalid variation link {index}")),
            };
        }
        if candidates.len() > 1 && total == 0 {
            return Err(format!("M2 animation {id} has no variation weights"));
        }
        Ok(Self { candidates, total })
    }

    fn choose(&self, roll: u32) -> Result<usize, String> {
        if roll >= self.total {
            return Err(format!("M2 variation roll {roll} exceeds {}", self.total));
        }
        let mut remaining = roll;
        for &(index, weight) in &self.candidates {
            if remaining < weight {
                return Ok(index);
            }
            remaining -= weight;
        }
        Err("M2 variation weights do not cover roll".into())
    }
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

/// Deterministic M2 sequence controller, independent of Godot frame timing.
/// The caller owns explicit clip selection, time advancement and pause policy.
pub struct AnimationState {
    sequences: Vec<m2::Sequence>,
    tracks: Vec<m2::BoneAnimTracks>,
    local_pivots: Vec<Vector3>,
    current: usize,
    time_ms: f64,
    looping: bool,
    transition: Option<Transition>,
    random_state: u64,
}

impl AnimationState {
    pub fn new(model: &m2::Model) -> Result<Self, String> {
        if model.sequences.is_empty() || model.bones.len() != model.bone_tracks.len() {
            return Err("M2 animation requires sequences and a track for each bone".into());
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
        Ok(Self {
            sequences: model.sequences.clone(),
            tracks: model.bone_tracks.clone(),
            local_pivots,
            current: 0,
            time_ms: 0.0,
            looping: true,
            transition: None,
            random_state: 0,
        })
    }

    fn sample_sequence(&self, index: usize, time_ms: f64) -> Vec<BonePose> {
        self.tracks
            .iter()
            .zip(&self.local_pivots)
            .map(|(track, &pivot)| {
                let time = time_ms as u32;
                let translation = m2_anim::evaluate_vec3_track(&track.translation, index, time)
                    .map(wow_vec3)
                    .unwrap_or(Vector3::ZERO);
                let rotation = m2_anim::evaluate_rotation_track(&track.rotation, index, time)
                    .map(|q| Quaternion::new(q[0], q[1], q[2], q[3]).normalized())
                    .unwrap_or(Quaternion::IDENTITY);
                let scale = m2_anim::evaluate_vec3_track(&track.scale, index, time)
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
    pub fn poses(&self) -> Vec<Transform3D> {
        self.sampled_poses()
            .into_iter()
            .map(BonePose::transform)
            .collect()
    }

    pub fn select(&mut self, index: usize, looping: bool) -> Result<(), String> {
        let Some(sequence) = self.sequences.get(index) else {
            return Err(format!("M2 sequence index {index} is out of range"));
        };
        if self.current == index {
            self.looping = looping;
            return Ok(());
        }
        let outgoing = if self.transition.is_some() {
            Outgoing::Snapshot(self.sampled_poses())
        } else {
            Outgoing::Sequence {
                index: self.current,
                time_ms: self.time_ms,
            }
        };
        self.transition = Some(Transition {
            outgoing,
            elapsed_ms: 0.0,
            duration_ms: (sequence.blend_time as f32).max(MIN_MOVEMENT_BLEND_MS),
        });
        self.current = index;
        self.time_ms = 0.0;
        self.looping = looping;
        Ok(())
    }

    pub fn advance(&mut self, delta_ms: f64) -> Result<(), String> {
        let mut state = self.random_state;
        let result = self.advance_with_roll(delta_ms, |upper| sample_roll(&mut state, upper));
        self.random_state = state;
        result
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
        let family = VariationFamily::read(&self.sequences, self.current)?;
        if family.candidates.len() == 1 {
            self.time_ms = elapsed % duration;
            self.tick_transition(delta_ms);
            return Ok(());
        }
        let shortest = family
            .candidates
            .iter()
            .filter(|(_, weight)| *weight > 0)
            .map(|(index, _)| self.sequences[*index].duration)
            .min()
            .ok_or("M2 variation family has no playable duration")?;
        if elapsed / f64::from(shortest) >= 4096.0 {
            return Err("M2 elapsed animation requires 4096 or more variation boundaries".into());
        }
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
            let next = family.choose(roll(family.total))?;
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
}

#[godot_api]
impl INode for WowAnimationPlayer {
    fn init(base: Base<Node>) -> Self {
        Self {
            base,
            animation: None,
            skeleton: None,
            paused: false,
        }
    }

    fn process(&mut self, delta: f64) {
        self.advance_time_ms(delta * 1000.0);
    }
}

impl WowAnimationPlayer {
    pub fn from_model(model: &m2::Model, skeleton: Gd<Skeleton3D>) -> Result<Gd<Self>, String> {
        let animation = AnimationState::new(model)?;
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
        });
        player.set_name("WowAnimationPlayer");
        player.bind_mut().write_poses();
        Ok(player)
    }

    fn write_poses(&mut self) {
        let (Some(animation), Some(skeleton)) = (&self.animation, &mut self.skeleton) else {
            return;
        };
        for (index, pose) in animation.sampled_poses().into_iter().enumerate() {
            skeleton.set_bone_pose_position(index as i32, pose.position);
            skeleton.set_bone_pose_rotation(index as i32, pose.rotation);
            skeleton.set_bone_pose_scale(index as i32, pose.scale);
        }
    }
}

#[godot_api]
impl WowAnimationPlayer {
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

    #[func]
    fn advance_time_ms(&mut self, delta_ms: f64) -> bool {
        if self.paused {
            return true;
        }
        let result = self
            .animation
            .as_mut()
            .ok_or("M2 animation has no bound model".to_string())
            .and_then(|animation| animation.advance(delta_ms));
        if let Err(error) = result {
            godot_error!("{error}");
            return false;
        }
        self.write_poses();
        true
    }

    #[func]
    fn set_paused(&mut self, paused: bool) {
        self.paused = paused;
    }
}

#[cfg(test)]
mod tests {
    use super::AnimationState;
    use game_engine_core::{asset::m2_format::m2_anim, m2};
    use godot::builtin::{Basis, Quaternion, Transform3D, Vector3};
    use std::{fs, path::PathBuf};

    fn model() -> m2::Model {
        let root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../data/models");
        let read = |name| fs::read(root.join(name)).expect("authored fixture");
        m2::parse_model_with_skeleton(
            &read("humanmale_hd.m2"),
            &read("humanmale_hd00.skin"),
            Some(&read("humanmale_hd.skel")),
        )
        .expect("HD model with authored tracks")
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
    fn hd_stand_samples_authored_bone_and_local_pivot() {
        let model = model();
        let mut player = AnimationState::new(&model).expect("animated model");
        let initial = player.poses();
        player.advance(1000.0).expect("valid time");
        let sampled = player.poses();
        assert_eq!(sampled.len(), 216);
        assert_eq!(model.sequences[0].id, 0);
        assert_eq!(model.sequences[0].duration, 2667);
        let bone = model
            .bone_tracks
            .iter()
            .enumerate()
            .find(|(index, track)| {
                m2_anim::evaluate_vec3_track(&track.translation, 0, 0)
                    != m2_anim::evaluate_vec3_track(&track.translation, 0, 1000)
                    && model.bones[*index].parent_bone_id >= 0
            })
            .map(|(index, _)| index)
            .expect("Stand has an animated child translation");
        let parent = model.bones[bone].parent_bone_id as usize;
        let local_pivot = basis(model.bones[bone].pivot) - basis(model.bones[parent].pivot);
        let translation =
            m2_anim::evaluate_vec3_track(&model.bone_tracks[bone].translation, 0, 1000)
                .expect("authored translation at 1000ms");
        assert!(near(sampled[bone].origin, local_pivot + basis(translation)));
        assert!(!near_pose(sampled[bone], initial[bone]));
        let rotation = m2_anim::evaluate_rotation_track(&model.bone_tracks[bone].rotation, 0, 1000)
            .map(|q| Quaternion::new(q[0], q[1], q[2], q[3]).normalized())
            .unwrap_or(Quaternion::IDENTITY);
        assert!(near(
            sampled[bone].basis.rows[0],
            Basis::from_quaternion(rotation).rows[0]
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
    fn looping_and_nonlooping_advance_are_deterministic() {
        let model = model();
        let mut looping = AnimationState::new(&model).expect("animated model");
        let mut direct = AnimationState::new(&model).expect("animated model");
        looping.advance(2800.0).expect("wrap");
        direct.advance(133.0).expect("remainder");
        assert!(
            looping
                .poses()
                .iter()
                .zip(direct.poses())
                .all(|(a, b)| near_pose(*a, b))
        );
        direct.select(1, false).expect("nonlooping clip");
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
}
