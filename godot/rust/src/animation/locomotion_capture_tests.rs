//! Offline capture input: authored HumanMale mesh skinned with production sampled poses.
//! The host renders these records without a display server or live game server.
use super::npc_pose_tests::{human_male_hd, human_male_model};
use super::{AnimationState, wow_vec3};
use godot::builtin::Transform3D;

fn skinned_positions(player: &AnimationState, vertices: &[u16]) -> Vec<[f32; 3]> {
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
    vertices
        .iter()
        .map(|&index| {
            let vertex = &model.vertices[usize::from(index)];
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
    // The production default body/face variants, with group-zero hairstyles disabled.
    let indices: Vec<u16> = model
        .submeshes
        .iter()
        .filter(|mesh| {
            game_engine_core::geoset_visibility_data::is_geoset_visible(
                mesh.mesh_part_id,
                &[(0, 0)],
                &[0],
            )
        })
        .flat_map(|mesh| {
            let start = mesh.triangle_start as usize;
            model.indices[start..start + usize::from(mesh.triangle_count)]
                .iter()
                .copied()
        })
        .collect();
    assert!(!indices.is_empty());
    let vertices: Vec<u16> = indices
        .iter()
        .copied()
        .collect::<std::collections::BTreeSet<_>>()
        .into_iter()
        .collect();
    let indices: Vec<u16> = indices
        .iter()
        .map(|index| vertices.binary_search(index).unwrap() as u16)
        .collect();
    println!(r#"LOCOMOTION_MESH {{"indices":{indices:?},"model":1011653}}"#);
    let mut player = human_male_hd();
    let mut seen = Vec::new();
    for step in 0..160 {
        let (movement, jumping) = match step {
            0..=9 => (0, false),
            10..=49 => (0, true),
            50..=109 => (0, false),
            110..=119 => (5, false),
            120..=129 => (5, true),
            130..=149 => (42, false),
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
        if step % 4 == 0 || [10, 50, 110, 120, 130, 150].contains(&step) {
            let weight = player
                .transition
                .as_ref()
                .map_or(1.0, |blend| blend.elapsed_ms / blend.duration_ms);
            let ms = (step + 1) * 25;
            let vertices = skinned_positions(&player, &vertices);
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
