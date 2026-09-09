//! M2 raw-TRS curves for Bevy's animation graph; pivot adjustment happens after blending.

use std::{
    any::{Any, TypeId},
    fmt,
    sync::Arc,
};

use bevy::animation::{
    AnimationEntityMut, AnimationEvaluationError, AnimationTargetId,
    animation_curves::{
        AnimatableCurve, AnimatableProperty, AnimationCurve, AnimationCurveEvaluator, EvaluatorId,
    },
    graph::AnimationNodeIndex,
};
use bevy::math::curve::{Curve, Interval};
use bevy::prelude::*;

use super::{BonePivot, M2AnimData, SphericalBillboard, evaluate_bone_components};
use crate::asset::m2_format::m2_anim::{AnimTrack, BoneAnimTracks};

#[derive(Component, Default, Clone, Copy)]
pub(super) struct RawBonePose(pub Transform);

pub(super) fn bone_target_id(index: usize) -> AnimationTargetId {
    AnimationTargetId::from_name(&Name::new(format!("m2-bone-{index}")))
}

pub(super) fn build_clip(data: &M2AnimData, seq_idx: usize) -> AnimationClip {
    let duration = clip_duration(data, seq_idx);
    let mut clip = AnimationClip::default();
    for (index, tracks) in data.bone_tracks.iter().enumerate() {
        let curve = M2RawCurve {
            source: build_curve_source(tracks, seq_idx),
            domain: Interval::new(0.0, duration).expect("M2 clip duration must be positive"),
        };
        clip.add_curve_to_target(
            bone_target_id(index),
            PivotCurve(AnimatableCurve::new(WholeTransform, curve)),
        );
    }
    clip.set_duration(duration);
    clip
}

pub(super) fn build_pose_clip(
    poses: impl IntoIterator<Item = (usize, Transform)>,
) -> AnimationClip {
    let mut clip = AnimationClip::default();
    for (index, pose) in poses {
        let curve = M2RawCurve {
            source: RawCurveSource::Pose(pose),
            domain: Interval::new(0.0, 1.0).expect("snapshot clip duration must be positive"),
        };
        clip.add_curve_to_target(
            bone_target_id(index),
            PivotCurve(AnimatableCurve::new(WholeTransform, curve)),
        );
    }
    clip.set_duration(1.0);
    clip
}

fn build_curve_source(tracks: &BoneAnimTracks, seq_idx: usize) -> RawCurveSource {
    if is_constant_sequence(&tracks.translation, seq_idx)
        && is_constant_sequence(&tracks.rotation, seq_idx)
        && is_constant_sequence(&tracks.scale, seq_idx)
    {
        return RawCurveSource::Pose(sample_raw_transform(tracks, seq_idx, 0));
    }
    RawCurveSource::Tracks(Arc::new(BoneAnimTracks {
        translation: select_sequence(&tracks.translation, seq_idx),
        rotation: select_sequence(&tracks.rotation, seq_idx),
        scale: select_sequence(&tracks.scale, seq_idx),
    }))
}

fn is_constant_sequence<T>(track: &AnimTrack<T>, seq_idx: usize) -> bool {
    track.global_sequence < 0
        && track
            .sequences
            .get(seq_idx)
            .is_none_or(|(times, values)| times.len() <= 1 && values.len() <= 1)
}

fn sample_raw_transform(tracks: &BoneAnimTracks, seq_idx: usize, time_ms: u32) -> Transform {
    let (translation, rotation, scale) = evaluate_bone_components(tracks, seq_idx, time_ms);
    Transform {
        translation,
        rotation,
        scale,
    }
}

fn select_sequence<T: Clone>(track: &AnimTrack<T>, seq_idx: usize) -> AnimTrack<T> {
    AnimTrack {
        interpolation_type: track.interpolation_type,
        global_sequence: track.global_sequence,
        sequences: track.sequences.get(seq_idx).cloned().into_iter().collect(),
    }
}

fn clip_duration(data: &M2AnimData, seq_idx: usize) -> f32 {
    let authored = data
        .sequences
        .get(seq_idx)
        .map(|sequence| sequence.duration)
        .unwrap_or(0);
    let duration_ms = if authored > 0 {
        authored
    } else {
        data.bone_tracks
            .iter()
            .flat_map(|bone| {
                [
                    last_timestamp(&bone.translation, seq_idx),
                    last_timestamp(&bone.rotation, seq_idx),
                    last_timestamp(&bone.scale, seq_idx),
                ]
            })
            .max()
            .unwrap_or(0)
    };
    // A static pose still needs a nonempty domain to be evaluated by Bevy.
    duration_ms.max(1) as f32 / 1000.0
}

fn last_timestamp<T>(track: &AnimTrack<T>, seq_idx: usize) -> u32 {
    track
        .sequences
        .get(seq_idx)
        .and_then(|(times, _)| times.last())
        .copied()
        .unwrap_or(0)
}

#[derive(Clone, Reflect)]
#[reflect(opaque)]
struct M2RawCurve {
    source: RawCurveSource,
    domain: Interval,
}

#[derive(Clone)]
enum RawCurveSource {
    Tracks(Arc<BoneAnimTracks>),
    Pose(Transform),
}

impl fmt::Debug for M2RawCurve {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("M2RawCurve")
            .field("domain", &self.domain)
            .finish_non_exhaustive()
    }
}

impl Curve<Transform> for M2RawCurve {
    fn domain(&self) -> Interval {
        self.domain
    }

    fn sample_unchecked(&self, time: f32) -> Transform {
        match &self.source {
            RawCurveSource::Tracks(tracks) => {
                sample_raw_transform(tracks, 0, (time * 1000.0) as u32)
            }
            RawCurveSource::Pose(pose) => *pose,
        }
    }
}

#[derive(Clone)]
struct WholeTransform;

impl AnimatableProperty for WholeTransform {
    type Property = Transform;

    fn get_mut<'a>(
        &self,
        entity: &'a mut AnimationEntityMut,
    ) -> Result<&'a mut Transform, AnimationEvaluationError> {
        entity
            .get_mut::<RawBonePose>()
            .map(|value| &mut value.into_inner().0)
            .ok_or(AnimationEvaluationError::ComponentNotPresent(TypeId::of::<
                RawBonePose,
            >(
            )))
    }

    fn evaluator_id(&self) -> EvaluatorId<'_> {
        EvaluatorId::Type(TypeId::of::<Self>())
    }
}

#[derive(Clone, Debug)]
struct PivotCurve(AnimatableCurve<WholeTransform, M2RawCurve>);

impl AnimationCurve for PivotCurve {
    fn clone_value(&self) -> Box<dyn AnimationCurve> {
        Box::new(self.clone())
    }
    fn domain(&self) -> Interval {
        self.0.domain()
    }
    fn evaluator_id(&self) -> EvaluatorId<'_> {
        EvaluatorId::Type(TypeId::of::<PivotEvaluator>())
    }
    fn create_evaluator(&self) -> Box<dyn AnimationCurveEvaluator> {
        Box::new(PivotEvaluator {
            raw: self.0.create_evaluator(),
        })
    }
    fn apply(
        &self,
        evaluator: &mut dyn AnimationCurveEvaluator,
        time: f32,
        weight: f32,
        node: AnimationNodeIndex,
    ) -> Result<(), AnimationEvaluationError> {
        let evaluator = evaluator.downcast_mut::<PivotEvaluator>().ok_or(
            AnimationEvaluationError::InconsistentEvaluatorImplementation(TypeId::of::<
                PivotEvaluator,
            >()),
        )?;
        self.0.apply(evaluator.raw.as_mut(), time, weight, node)
    }
    fn sample_clamped(&self, time: f32) -> Box<dyn Any> {
        self.0.sample_clamped(time)
    }
}

struct PivotEvaluator {
    raw: Box<dyn AnimationCurveEvaluator>,
}

impl AnimationCurveEvaluator for PivotEvaluator {
    fn blend(&mut self, node: AnimationNodeIndex) -> Result<(), AnimationEvaluationError> {
        self.raw.blend(node)
    }
    fn add(&mut self, node: AnimationNodeIndex) -> Result<(), AnimationEvaluationError> {
        self.raw.add(node)
    }
    fn push_blend_register(
        &mut self,
        weight: f32,
        node: AnimationNodeIndex,
    ) -> Result<(), AnimationEvaluationError> {
        self.raw.push_blend_register(weight, node)
    }
    fn commit(&mut self, mut entity: AnimationEntityMut) -> Result<(), AnimationEvaluationError> {
        let pivot = entity
            .get::<BonePivot>()
            .ok_or(AnimationEvaluationError::ComponentNotPresent(TypeId::of::<
                BonePivot,
            >(
            )))?
            .0;
        self.raw.commit(entity.reborrow())?;
        let mut pose = entity
            .get::<RawBonePose>()
            .ok_or(AnimationEvaluationError::ComponentNotPresent(TypeId::of::<
                RawBonePose,
            >(
            )))?
            .0;
        pose.translation = pose.translation + pivot - pose.rotation * (pose.scale * pivot);
        if let Some(mut billboard) = entity.get_mut::<SphericalBillboard>() {
            billboard.pending_pose = Some(pose);
            return Ok(());
        }
        let mut transform =
            entity
                .get_mut::<Transform>()
                .ok_or(AnimationEvaluationError::ComponentNotPresent(TypeId::of::<
                    Transform,
                >(
                )))?;
        transform.set_if_neq(pose);
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::asset::m2_format::m2_anim::{AnimTrack, BoneAnimTracks, M2AnimSequence};
    use bevy::animation::AnimatedBy;

    fn track<T>(sequences: Vec<(Vec<u32>, Vec<T>)>) -> AnimTrack<T> {
        AnimTrack {
            interpolation_type: 1,
            global_sequence: -1,
            sequences,
        }
    }

    fn data(tracks: BoneAnimTracks, sequence_count: usize) -> M2AnimData {
        M2AnimData {
            bones: vec![],
            spherical_billboards: vec![],
            sequences: (0..sequence_count)
                .map(|_| M2AnimSequence {
                    id: 0,
                    variation_id: 0,
                    duration: 1000,
                    movespeed: 0.0,
                    flags: 0,
                    blend_time: 150,
                    frequency: 32767,
                    replay: [0, 0],
                    variation_next: -1,
                })
                .collect(),
            bone_tracks: vec![tracks],
            joint_entities: vec![],
        }
    }

    fn evaluate(data: &M2AnimData, pivot: Vec3, samples: &[(usize, f32, f32)]) -> Transform {
        evaluate_clips(
            pivot,
            samples
                .iter()
                .map(|&(sequence, time, weight)| (build_clip(data, sequence), time, weight)),
        )
        .0
    }

    fn evaluate_clips(
        pivot: Vec3,
        samples: impl IntoIterator<Item = (AnimationClip, f32, f32)>,
    ) -> (Transform, Transform) {
        let mut app = App::new();
        app.add_plugins((
            MinimalPlugins,
            AssetPlugin::default(),
            bevy::animation::AnimationPlugin,
        ));
        let mut graph = AnimationGraph::new();
        let mut player = AnimationPlayer::default();
        for (clip, time, weight) in samples {
            let clip = app
                .world_mut()
                .resource_mut::<Assets<AnimationClip>>()
                .add(clip);
            let node = graph.add_clip(clip, 1.0, graph.root);
            player
                .play(node)
                .set_weight(weight)
                .set_seek_time(time)
                .pause();
        }
        let graph = app
            .world_mut()
            .resource_mut::<Assets<AnimationGraph>>()
            .add(graph);
        let owner = app
            .world_mut()
            .spawn((player, AnimationGraphHandle(graph)))
            .id();
        let bone = app
            .world_mut()
            .spawn((
                bone_target_id(0),
                AnimatedBy(owner),
                BonePivot(pivot),
                RawBonePose::default(),
                Transform::from_xyz(99.0, 98.0, 97.0),
            ))
            .id();
        // The first PostUpdate publishes graph asset events; Bevy builds its
        // threaded graph from those events on the following update.
        app.update();
        app.update();
        (
            *app.world().get::<Transform>(bone).unwrap(),
            app.world().get::<RawBonePose>(bone).unwrap().0,
        )
    }

    #[derive(Resource, Default)]
    struct TransformChangeLog(Vec<Transform>);

    fn record_transform_changes(
        bones: Query<&Transform, (With<BonePivot>, Changed<Transform>)>,
        mut changes: ResMut<TransformChangeLog>,
    ) {
        changes.0.extend(bones.iter().copied());
    }

    #[test]
    fn constant_bevy_pose_does_not_notify_transform_changes() {
        let data = data(
            BoneAnimTracks {
                translation: track(vec![(
                    vec![0, 600, 1000],
                    vec![[2.0, 4.0, 6.0], [2.0, 4.0, 6.0], [8.0, 10.0, 12.0]],
                )]),
                rotation: track(vec![(vec![0], vec![[32767, 32767, -1, 32767]])]),
                scale: track(vec![(vec![0], vec![[2.0, 3.0, 4.0]])]),
            },
            1,
        );
        let pivot = Vec3::new(2.0, -3.0, 4.0);
        let mut app = App::new();
        app.add_plugins((
            MinimalPlugins,
            AssetPlugin::default(),
            bevy::animation::AnimationPlugin,
        ));
        app.init_resource::<TransformChangeLog>();
        app.add_systems(Last, record_transform_changes);
        let clip = app
            .world_mut()
            .resource_mut::<Assets<AnimationClip>>()
            .add(build_clip(&data, 0));
        let mut graph = AnimationGraph::new();
        let node = graph.add_clip(clip, 1.0, graph.root);
        let graph = app
            .world_mut()
            .resource_mut::<Assets<AnimationGraph>>()
            .add(graph);
        let mut player = AnimationPlayer::default();
        player.play(node).pause().seek_to(0.0);
        let owner = app
            .world_mut()
            .spawn((player, AnimationGraphHandle(graph)))
            .id();
        let bone = app
            .world_mut()
            .spawn((
                bone_target_id(0),
                AnimatedBy(owner),
                BonePivot(pivot),
                Transform::from_xyz(99.0, 98.0, 97.0),
            ))
            .id();
        app.update();
        app.update();
        let initial = *app.world().get::<Transform>(bone).unwrap();
        let (translation, rotation, scale) =
            super::super::evaluate_bone_components(&data.bone_tracks[0], 0, 0);
        assert_pose(
            initial,
            Transform {
                translation,
                rotation,
                scale,
            },
            pivot,
        );
        app.world_mut()
            .resource_mut::<TransformChangeLog>()
            .0
            .clear();

        for time in [0.1, 0.3, 0.5] {
            app.world_mut()
                .get_mut::<AnimationPlayer>(owner)
                .unwrap()
                .animation_mut(node)
                .unwrap()
                .seek_to(time);
            app.update();
            assert_eq!(*app.world().get::<Transform>(bone).unwrap(), initial);
        }
        let stationary_changes =
            std::mem::take(&mut app.world_mut().resource_mut::<TransformChangeLog>().0);

        app.world_mut()
            .get_mut::<AnimationPlayer>(owner)
            .unwrap()
            .animation_mut(node)
            .unwrap()
            .seek_to(0.8);
        app.update();
        let changed = *app.world().get::<Transform>(bone).unwrap();
        assert_ne!(changed, initial);
        assert_eq!(
            app.world().resource::<TransformChangeLog>().0,
            vec![changed]
        );
        assert!(
            stationary_changes.is_empty(),
            "identical evaluated poses emitted {} downstream Transform changes",
            stationary_changes.len(),
        );
    }

    #[test]
    fn snapshot_clip_blends_raw_rotation_scale_before_pivot_and_retains_pose() {
        let pivot = Vec3::new(2.0, 3.0, 4.0);
        let pose = Transform {
            translation: Vec3::new(4.0, 6.0, 8.0),
            rotation: Quat::from_rotation_z(std::f32::consts::FRAC_PI_2),
            scale: Vec3::new(3.0, 5.0, 7.0),
        };
        let expected = Transform {
            translation: pose.translation * 0.5,
            rotation: Quat::IDENTITY.slerp(pose.rotation, 0.5),
            scale: Vec3::ONE.lerp(pose.scale, 0.5),
        };
        let (actual, retained) = evaluate_clips(
            pivot,
            [
                (build_pose_clip([(0, Transform::IDENTITY)]), 0.25, 0.5),
                (build_pose_clip([(0, pose)]), 0.75, 0.5),
            ],
        );
        assert_pose(actual, expected, pivot);
        assert_pose(retained, expected, Vec3::ZERO);
    }

    fn assert_pose(actual: Transform, raw: Transform, pivot: Vec3) {
        let expected_translation = raw.translation + pivot - raw.rotation * (raw.scale * pivot);
        assert!(
            actual.translation.abs_diff_eq(expected_translation, 1e-4),
            "translation {actual:?} expected {expected_translation:?}"
        );
        assert!(
            actual.rotation.abs_diff_eq(raw.rotation, 1e-4),
            "rotation {actual:?} expected {:?}",
            raw.rotation
        );
        assert!(
            actual.scale.abs_diff_eq(raw.scale, 1e-4),
            "scale {actual:?} expected {:?}",
            raw.scale
        );
    }

    fn constant_tracks() -> BoneAnimTracks {
        BoneAnimTracks {
            translation: track(vec![(vec![300], vec![[2.0, 4.0, 6.0]])]),
            rotation: track(vec![(vec![700], vec![[32767, 32767, -1, 32767]])]),
            scale: track(vec![(vec![900], vec![[2.0, 3.0, 4.0]])]),
        }
    }

    fn legacy_raw_pose(tracks: &BoneAnimTracks, sequence: usize, time_ms: u32) -> Transform {
        let (translation, rotation, scale) = evaluate_bone_components(tracks, sequence, time_ms);
        Transform {
            translation,
            rotation,
            scale,
        }
    }

    fn assert_curve_matches_legacy(tracks: BoneAnimTracks, sequence: usize) {
        let data = data(tracks, sequence + 1);
        let pivot = Vec3::new(2.0, -3.0, 4.0);
        for time_ms in [0, 100, 500, 900, 1000] {
            let expected = legacy_raw_pose(&data.bone_tracks[0], sequence, time_ms);
            let (actual, retained) = evaluate_clips(
                pivot,
                [(build_clip(&data, sequence), time_ms as f32 / 1000.0, 1.0)],
            );
            assert_pose(actual, expected, pivot);
            assert_pose(retained, expected, Vec3::ZERO);
        }
    }

    #[test]
    fn constant_curves_preserve_empty_single_key_and_missing_sequence_samples() {
        assert_curve_matches_legacy(
            BoneAnimTracks {
                translation: track(vec![]),
                rotation: track(vec![]),
                scale: track(vec![]),
            },
            0,
        );
        assert_curve_matches_legacy(
            BoneAnimTracks {
                translation: track(vec![(vec![300], vec![])]),
                rotation: track(vec![(vec![], vec![[32767, 32767, -1, 32767]])]),
                scale: track(vec![(vec![], vec![])]),
            },
            0,
        );
        assert_curve_matches_legacy(constant_tracks(), 0);
        let mut tracks = constant_tracks();
        tracks.translation.sequences[0] = (vec![0, 1000], vec![[1.0, 2.0, 3.0], [7.0, 8.0, 9.0]]);
        assert_curve_matches_legacy(tracks, 1);
    }

    #[test]
    fn mixed_and_global_curves_preserve_existing_samples() {
        let mut translation = constant_tracks();
        translation.translation.sequences[0] =
            (vec![0, 1000], vec![[1.0, 2.0, 3.0], [7.0, 8.0, 9.0]]);
        let mut rotation = constant_tracks();
        rotation.rotation.sequences[0] = (
            vec![0, 1000],
            vec![[32767, 32767, 32767, -1], [32767, 32767, -1, 32767]],
        );
        let mut scale = constant_tracks();
        scale.scale.sequences[0] = (vec![0, 1000], vec![[1.0; 3], [3.0, 5.0, 7.0]]);
        let mut global = constant_tracks();
        global.translation.global_sequence = 0;
        global.rotation.global_sequence = 1;
        global.scale.global_sequence = 2;
        for tracks in [translation, rotation, scale, global] {
            assert_curve_matches_legacy(tracks, 0);
        }
    }

    fn assert_crossfade_samples(
        data: &M2AnimData,
        pivot: Vec3,
        from: usize,
        to: usize,
        time_ms: u32,
    ) {
        let before = legacy_raw_pose(&data.bone_tracks[0], from, time_ms);
        let after = legacy_raw_pose(&data.bone_tracks[0], to, time_ms);
        for blend in [0.0, 0.4, 1.0] {
            let expected = Transform {
                translation: before.translation.lerp(after.translation, blend),
                rotation: before.rotation.slerp(after.rotation, blend),
                scale: before.scale.lerp(after.scale, blend),
            };
            let time = time_ms as f32 / 1000.0;
            let (actual, retained) = evaluate_clips(
                pivot,
                [
                    (build_clip(data, from), time, 1.0 - blend),
                    (build_clip(data, to), time, blend),
                ],
            );
            assert_pose(actual, expected, pivot);
            assert_pose(retained, expected, Vec3::ZERO);
        }
    }

    #[test]
    fn constant_and_varying_crossfades_preserve_raw_pose_in_both_directions() {
        let mut tracks = constant_tracks();
        tracks
            .translation
            .sequences
            .push((vec![0, 1000], vec![[0.0; 3], [10.0, -2.0, 4.0]]));
        tracks.rotation.sequences.push((
            vec![0, 1000],
            vec![[32767, 32767, 32767, -1], [32767, -9598, 32767, -9598]],
        ));
        tracks
            .scale
            .sequences
            .push((vec![0, 1000], vec![[1.0; 3], [3.0, 5.0, 7.0]]));
        let data = data(tracks, 2);
        let pivot = Vec3::new(2.0, -3.0, 4.0);
        for (from, to) in [(0, 1), (1, 0)] {
            for time_ms in [200, 800] {
                assert_crossfade_samples(&data, pivot, from, to, time_ms);
            }
        }
    }

    #[test]
    fn bevy_curve_defaults_replace_existing_transform_with_identity() {
        let data = data(
            BoneAnimTracks {
                translation: track(vec![]),
                rotation: track(vec![]),
                scale: track(vec![]),
            },
            1,
        );
        let pivot = Vec3::new(2.0, 3.0, -4.0);
        assert_pose(
            evaluate(&data, pivot, &[(0, 0.5, 1.0)]),
            Transform::IDENTITY,
            pivot,
        );
    }

    #[test]
    fn bevy_curve_samples_raw_translation_rotation_scale_before_pivot() {
        let tracks = BoneAnimTracks {
            translation: track(vec![(
                vec![0, 1000],
                vec![[0.0, 0.0, 0.0], [4.0, 6.0, 8.0]],
            )]),
            rotation: track(vec![(
                vec![0, 1000],
                vec![[32767, 32767, 32767, -1], [32767, 32767, -1, 32767]],
            )]),
            scale: track(vec![(vec![0, 1000], vec![[1.0; 3], [3.0, 5.0, 7.0]])]),
        };
        let data = data(tracks, 1);
        let pivot = Vec3::new(2.0, -1.0, 3.0);
        let (translation, rotation, scale) =
            super::super::evaluate_bone_components(&data.bone_tracks[0], 0, 500);
        assert_pose(
            evaluate(&data, pivot, &[(0, 0.5, 1.0)]),
            Transform {
                translation,
                rotation,
                scale,
            },
            pivot,
        );
    }

    #[test]
    fn bevy_curve_half_crossfade_blends_raw_pose_not_pivoted_translations() {
        let tracks = BoneAnimTracks {
            translation: track(vec![
                (vec![0], vec![[0.0; 3]]),
                (vec![0], vec![[4.0, 6.0, 8.0]]),
            ]),
            rotation: track(vec![
                (vec![0], vec![[32767, 32767, 32767, -1]]),
                (vec![0], vec![[32767, 32767, -1, 32767]]),
            ]),
            scale: track(vec![
                (vec![0], vec![[1.0; 3]]),
                (vec![0], vec![[3.0, 5.0, 7.0]]),
            ]),
        };
        let data = data(tracks, 2);
        let pivot = Vec3::new(2.0, -1.0, 3.0);
        let from = super::super::evaluate_bone_components(&data.bone_tracks[0], 0, 0);
        let to = super::super::evaluate_bone_components(&data.bone_tracks[0], 1, 0);
        let raw = Transform {
            translation: from.0.lerp(to.0, 0.5),
            rotation: from.1.slerp(to.1, 0.5),
            scale: from.2.lerp(to.2, 0.5),
        };
        let wrong = (from.0 + pivot - from.1 * (from.2 * pivot))
            .lerp(to.0 + pivot - to.1 * (to.2 * pivot), 0.5);
        assert!(!wrong.abs_diff_eq(
            raw.translation + pivot - raw.rotation * (raw.scale * pivot),
            0.01
        ));
        assert_pose(
            evaluate(&data, pivot, &[(0, 0.5, 0.5), (1, 0.5, 0.5)]),
            raw,
            pivot,
        );
    }

    #[test]
    fn bevy_curve_samples_sequence_zero_without_sequence_metadata() {
        let data = data(
            BoneAnimTracks {
                translation: track(vec![(vec![0], vec![[1.0, 2.0, 3.0]])]),
                rotation: track(vec![]),
                scale: track(vec![]),
            },
            0,
        );
        assert_pose(
            evaluate(&data, Vec3::ZERO, &[(0, 0.0, 1.0)]),
            Transform::from_xyz(1.0, 3.0, -2.0),
            Vec3::ZERO,
        );
    }
}
