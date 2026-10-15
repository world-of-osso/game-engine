use super::AnimationState;
use super::npc_pose_tests::{human_male_hd, pose_distance};
use game_engine_core::mount_special::MountSpecialCatalog;
use std::{collections::HashMap, path::Path};

fn kit_catalog() -> MountSpecialCatalog {
    MountSpecialCatalog::read(&Path::new(env!("CARGO_MANIFEST_DIR")).join("../../data")).unwrap()
}

#[test]
fn mount_special_authored_upper_body_segment_keeps_the_seated_legs_and_finishes() {
    let mut rider = human_male_hd();
    let mut seated = human_male_hd();
    rider.update_locomotion(91, false, false).unwrap();
    seated.update_locomotion(91, false, false).unwrap();
    let catalog = kit_catalog();
    rider
        // Isolate the actual DND upper-body row to prove mask composition, not a
        // substitute claim about its simultaneous full kit or Grrloc's timing gap.
        .play_mount_kits(&[&catalog.kit(3012).unwrap()[1..]], 5000.0, &HashMap::new())
        .unwrap();
    rider.advance(900.0).unwrap();
    seated.advance(900.0).unwrap();
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
    assert!(upper > 0.01, "rider upper body did not animate: {upper}");
    assert!(lower < 1e-4, "rider legs left saddle pose: {lower}");
    rider.advance(6000.0).unwrap();
    seated.advance(6000.0).unwrap();
    assert!(pose_distance(&rider.poses(), &seated.poses()) < 1e-4);
}

#[test]
fn mount_special_grrloc_oversized_one_shot_is_a_named_gap_not_silent_rider_success() {
    let mut rider = human_male_hd();
    let catalog = kit_catalog();
    let error = rider
        .play_mount_kits(&[catalog.kit(19345).unwrap()], 5000.0, &HashMap::new())
        .unwrap_err();
    assert!(error.contains("one-shot offset 2000"), "{error}");
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

#[test]
fn mount_special_spelltome_authored_attachment_diagnostic() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../data/models");
    let model = game_engine_core::m2::parse_model_with_skeleton(
        &std::fs::read(root.join("4061989.m2")).unwrap(),
        &std::fs::read(root.join("406198900.skin")).unwrap(),
        None,
        |fdid| std::fs::read(root.join(format!("{fdid}.anim"))).ok(),
    )
    .unwrap();
    let seat = model.attachments.iter().find(|seat| seat.id == 0).unwrap();
    println!("SPELLTOME_AUTHORED_SEAT {seat:?}");
    println!(
        "SPELLTOME_TEXTURES {:?} MATERIALS {:?}",
        model.texture_fdids, model.materials
    );
    println!(
        "SPELLTOME_SUBMESHES {:?} BATCHES {:?}",
        model.submeshes, model.batches
    );
    for emitter in &model.particle_emitters {
        println!(
            "SPELLTOME_EMITTER texture={:?} multi={:?} blend={} position={:?} bone={}",
            emitter.texture_fdid,
            emitter.multi_texture,
            emitter.blend_type,
            emitter.position,
            emitter.bone_index
        );
    }
    let mut mount = AnimationState::new(&model).unwrap();
    mount.update_locomotion(0, false, false).unwrap();
    let origin = authored_seat_position(&model, &mount.poses(), seat);
    println!("SPELLTOME_IDLE_SEAT {origin:?}");
    let duration = mount.play_mount_special(&HashMap::new()).unwrap();
    println!("SPELLTOME_SPECIAL_DURATION {duration}");
    for elapsed in [0.0, 500.0, 500.0, 500.0, 500.0, 1000.0] {
        mount.advance(elapsed).unwrap();
        println!(
            "SPELLTOME_SPECIAL_SEAT time={} position={:?}",
            mount.time_ms,
            authored_seat_position(&model, &mount.poses(), seat)
        );
    }
    assert!(duration > 0.0);
}

fn authored_seat_position(
    model: &game_engine_core::m2::Model,
    poses: &[godot::builtin::Transform3D],
    seat: &game_engine_core::asset::m2_format::m2_attach::M2Attachment,
) -> godot::builtin::Vector3 {
    let mut global = poses[usize::from(seat.bone)];
    let mut parent = model.bones[usize::from(seat.bone)].parent_bone_id;
    while parent >= 0 {
        global = poses[parent as usize] * global;
        parent = model.bones[parent as usize].parent_bone_id;
    }
    global
        * (super::wow_vec3(seat.position)
            - super::wow_vec3(model.bones[usize::from(seat.bone)].pivot))
}
