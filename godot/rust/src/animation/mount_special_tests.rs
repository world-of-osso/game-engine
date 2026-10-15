use super::AnimationState;
use super::npc_pose_tests::{human_male_hd, pose_distance};
use game_engine_core::mount_special::MountSpecialCatalog;
use std::{collections::HashMap, path::Path};

fn kit_catalog() -> MountSpecialCatalog {
    MountSpecialCatalog::read(&Path::new(env!("CARGO_MANIFEST_DIR")).join("../../data")).unwrap()
}

#[test]
fn mount_special_rider_upper_body_keeps_the_seated_legs_and_finishes() {
    let mut rider = human_male_hd();
    let mut seated = human_male_hd();
    rider.update_locomotion(91, false, false).unwrap();
    seated.update_locomotion(91, false, false).unwrap();
    let catalog = kit_catalog();
    rider
        .play_mount_kits(&[catalog.kit(19345).unwrap()], 5000.0, &HashMap::new())
        .unwrap();
    rider.advance(599.0).unwrap();
    seated.advance(599.0).unwrap();
    assert!(pose_distance(&rider.poses(), &seated.poses()) < 1e-4);
    rider.advance(301.0).unwrap();
    seated.advance(301.0).unwrap();
    let (upper, lower) = rider
        .poses()
        .iter()
        .zip(seated.poses())
        .zip(&rider.upper_body)
        .fold(
            (0.0_f32, 0.0_f32),
            |(upper, lower), ((actual, expected), is_upper)| {
                let distance = pose_distance(
                    std::slice::from_ref(actual),
                    std::slice::from_ref(&expected),
                );
                if *is_upper {
                    (upper.max(distance), lower)
                } else {
                    (upper, lower.max(distance))
                }
            },
        );
    let fishing = rider
        .sequences
        .iter()
        .position(|sequence| sequence.id == 133 && sequence.variation_id == 0)
        .unwrap();
    eprintln!(
        "FishingCast duration={} flags={} time={} upper_bones={} keyed_tracks={}",
        rider.sequences[fishing].duration,
        rider.sequences[fishing].flags,
        rider.global_ms,
        rider.upper_body.iter().filter(|bone| **bone).count(),
        rider
            .tracks
            .iter()
            .filter(|tracks| super::keyframed(&tracks.rotation, fishing)
                || super::keyframed(&tracks.translation, fishing))
            .count()
    );
    assert!(upper > 0.01, "rider upper body did not animate: {upper}");
    assert!(lower < 1e-4, "rider legs left saddle pose: {lower}");
    rider.advance(6000.0).unwrap();
    seated.advance(6000.0).unwrap();
    assert!(pose_distance(&rider.poses(), &seated.poses()) < 1e-4);
}

#[test]
fn mount_special_spelltome_late_rider_segment_outlives_the_first_segment() {
    let mut rider = human_male_hd();
    let mut seated = human_male_hd();
    rider.update_locomotion(91, false, false).unwrap();
    seated.update_locomotion(91, false, false).unwrap();
    let catalog = kit_catalog();
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../data/db2/active");
    let fallbacks = game_engine_core::spell_visual::read_animation_fallbacks(&root).unwrap();
    rider
        .play_mount_kits(&[catalog.kit(23323).unwrap()], 12000.0, &fallbacks)
        .unwrap();
    rider.advance(8000.0).unwrap();
    seated.advance(8000.0).unwrap();
    assert!(
        pose_distance(&rider.poses(), &seated.poses()) > 0.01,
        "later Spelltome segments were dropped"
    );
    rider.advance(5000.0).unwrap();
    seated.advance(5000.0).unwrap();
    assert!(pose_distance(&rider.poses(), &seated.poses()) < 1e-4);
}

#[test]
fn mount_special_full_body_sequence_is_not_replaced_by_per_frame_locomotion() {
    let mut mount: AnimationState = human_male_hd();
    let fallback = HashMap::from([(94, 60)]);
    let resolved = mount.resolve_clip(94, &fallback).unwrap();
    let duration = mount.play_mount_special(&fallback).unwrap();
    assert!(duration > 0.0);
    mount.advance(100.0).unwrap();
    mount.update_locomotion(0, false, false).unwrap();
    assert_eq!(mount.sequences[mount.current].id, resolved);
    mount.advance(duration + 200.0).unwrap();
    mount.update_locomotion(0, false, false).unwrap();
    assert_eq!(mount.sequences[mount.current].id, 0);
}
