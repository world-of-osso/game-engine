//! Planned sampling returns bit-identical poses to evaluating every track, on HumanMale
//! HD 1011653 (216 bones, empty, single-key, keyframed and global-sequence tracks).
use super::{BonePose, npc_pose_tests::human_male_hd};

fn bits(poses: &[BonePose]) -> Vec<[u32; 10]> {
    poses
        .iter()
        .map(|pose| {
            let (p, r, s) = (pose.position, pose.rotation, pose.scale);
            [p.x, p.y, p.z, r.x, r.y, r.z, r.w, s.x, s.y, s.z].map(f32::to_bits)
        })
        .collect()
}

#[test]
fn planned_samples_equal_full_track_evaluation_bit_for_bit() {
    let mut player = human_male_hd();
    let own = player.plans.sequence_count();
    let mut compared = 0;
    for index in 0..own {
        let duration = f64::from(player.sequences[index].duration.max(1));
        // Global-sequence tracks follow the model clock, so vary it with the time.
        for step in 0..=24 {
            let time_ms = duration * f64::from(step) / 24.0 + 0.37;
            player.global_ms = time_ms * 1.7;
            let planned = player.sample_sequence(index, time_ms);
            let full = player.sample_unplanned(index, time_ms);
            assert_eq!(
                bits(&planned),
                bits(&full),
                "sequence {index} at {time_ms} ms"
            );
            compared += planned.len();
        }
    }
    assert!(
        own > 100 && compared > own * 25 * 200,
        "{own} sequences, {compared} poses"
    );
}

#[test]
fn instances_of_one_model_share_its_plans() {
    let a = human_male_hd();
    let b = human_male_hd();
    assert!(std::sync::Arc::ptr_eq(&a.plans, &b.plans));
}
