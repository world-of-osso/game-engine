//! Authored head orientation versus the model-local portrait camera.
use std::{fs, path::PathBuf};

use game_engine_core::{asset::m2_format::m2_camera::parse_portrait_camera, m2};
use godot::prelude::*;

use crate::{animation::AnimationState, assets::wow_vec3};

#[test]
fn portrait_camera_faces_authored_heads() {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../data/models");
    for fdid in [
        1011653, 1000764, 1100258, 1100087, 878772, 950080, 917116, 949470,
    ] {
        let read = |suffix: &str| fs::read(root.join(format!("{fdid}{suffix}"))).unwrap();
        let bytes = read(".m2");
        let skeleton = fs::read(root.join(format!("{fdid}.skel"))).ok();
        let model =
            m2::parse_model_with_skeleton(&bytes, &read("00.skin"), skeleton.as_deref(), |id| {
                fs::read(root.join(format!("{id}.anim"))).ok()
            })
            .unwrap();
        let camera = parse_portrait_camera(&bytes).unwrap();
        let head = model
            .bones
            .iter()
            .position(|bone| bone.key_bone_id == 6)
            .unwrap();
        let poses = AnimationState::new(&model).unwrap().poses();
        let mut transform = poses[head];
        let mut parent = model.bones[head].parent_bone_id;
        while parent >= 0 {
            transform = poses[parent as usize] * transform;
            parent = model.bones[parent as usize].parent_bone_id;
        }
        let facing = transform.basis * Vector3::RIGHT;
        let to_eye = (wow_vec3(camera.position) - wow_vec3(camera.target)).normalized();
        let angle = facing.angle_to(to_eye).to_degrees();
        println!("portrait {fdid}: head={head}, facing={facing}, angle={angle}");
        assert!(angle < 55.0, "{fdid} portrait faces away: {angle} degrees");
    }
}
