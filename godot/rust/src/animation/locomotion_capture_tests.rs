//! Offline capture input: authored HumanMale mesh skinned with production sampled poses.
//! The host renders these records without a display server or live game server.
use super::npc_pose_tests::{human_male_hd, human_male_model};
use super::{AnimationState, wow_vec3};
use godot::builtin::Transform3D;

fn skinned_positions(player: &AnimationState) -> Vec<[f32; 3]> {
    let model = human_male_model();
    let mut globals = vec![Transform3D::IDENTITY; model.bones.len()];
    for (index, pose) in player.poses().into_iter().enumerate() {
        let parent = model.bones[index].parent_bone_id;
        globals[index] = if parent < 0 {
            pose
        } else {
            globals[parent as usize] * pose
        };
    }
    model
        .vertices
        .iter()
        .map(|vertex| {
            let position = wow_vec3(vertex.position);
            let mut skinned = godot::builtin::Vector3::ZERO;
            for (&bone, &weight) in vertex.bone_indices.iter().zip(&vertex.bone_weights) {
                if weight == 0 {
                    continue;
                }
                let index = usize::from(bone);
                let relative = position - wow_vec3(model.bones[index].pivot);
                skinned += (globals[index] * relative) * (f32::from(weight) / 255.0);
            }
            [skinned.x, skinned.y, skinned.z]
        })
        .collect()
}

#[test]
fn locomotion_offline_capture_jump_and_water_entry() {
    let model = human_male_model();
    // Body geosets only: no mutually exclusive clothing, hair, capes or equipment.
    let indices: Vec<u16> = model
        .submeshes
        .iter()
        .filter(|mesh| mesh.mesh_part_id == 0)
        .flat_map(|mesh| {
            let start = mesh.triangle_start as usize;
            model.indices[start..start + usize::from(mesh.triangle_count)]
                .iter()
                .copied()
        })
        .collect();
    assert!(!indices.is_empty());
    println!(r#"LOCOMOTION_MESH {{"indices":{indices:?},"model":1011653}}"#);
    let mut player = human_male_hd();
    let mut seen = Vec::new();
    for step in 0..140 {
        let (movement, jumping) = match step {
            0..=9 => (0, false),
            10..=49 => (0, true),
            50..=89 => (0, false),
            90..=99 => (5, false),
            100..=109 => (5, true),
            110..=129 => (42, false),
            _ => (41, false),
        };
        player.advance(25.0).unwrap();
        player
            .update_locomotion(movement, jumping, movement == 5)
            .unwrap();
        let id = player.sequences[player.current].id;
        if seen.last() != Some(&id) {
            seen.push(id);
        }
        if step % 4 == 0 || [10, 50, 100, 110, 130].contains(&step) {
            let weight = player
                .transition
                .as_ref()
                .map_or(1.0, |blend| blend.elapsed_ms / blend.duration_ms);
            let ms = step * 25;
            let vertices = skinned_positions(&player);
            println!(
                r#"LOCOMOTION_FRAME {{"step":{step},"ms":{ms},"id":{id},"weight":{weight},"vertices":{vertices:?}}}"#
            );
        }
    }
    assert!(
        seen.windows(4).any(|ids| ids == [37, 38, 39, 0]),
        "{seen:?}"
    );
    assert!(seen.windows(3).any(|ids| ids == [37, 42, 41]), "{seen:?}");
}
