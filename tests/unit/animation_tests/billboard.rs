use super::*;

#[test]
fn spherical_billboard_does_not_propagate_to_descendants() {
    let bones = vec![
        M2Bone {
            key_bone_id: -1,
            flags: 0x8,
            parent_bone_id: -1,
            submesh_id: 0,
            pivot: [0.0, 0.0, 0.0],
        },
        M2Bone {
            key_bone_id: -1,
            flags: 0,
            parent_bone_id: 0,
            submesh_id: 0,
            pivot: [0.0, 0.0, 0.0],
        },
        M2Bone {
            key_bone_id: -1,
            flags: 0,
            parent_bone_id: 1,
            submesh_id: 0,
            pivot: [0.0, 0.0, 0.0],
        },
    ];
    assert_eq!(
        propagate_spherical_billboards(&bones),
        vec![true, false, false]
    );
}

#[test]
fn non_billboard_child_world_pose_is_camera_stable() {
    let bones = vec![
        M2Bone {
            key_bone_id: -1,
            flags: 0x8,
            parent_bone_id: -1,
            submesh_id: 0,
            pivot: [0.0, 0.0, 0.0],
        },
        M2Bone {
            key_bone_id: -1,
            flags: 0,
            parent_bone_id: 0,
            submesh_id: 0,
            pivot: [1.0, 0.0, 0.0],
        },
    ];
    let sbb = vec![true, false];

    let mut pre_a = vec![Mat4::IDENTITY; 2];
    let mut post_a = vec![Mat4::IDENTITY; 2];
    let (_, pp_a, pp_post_a) = compute_bone_stages(
        &bones,
        &sbb,
        0,
        Vec3::ZERO,
        Quat::IDENTITY,
        Vec3::ONE,
        Some(Quat::IDENTITY),
        &pre_a,
        &post_a,
    )
    .unwrap();
    pre_a[0] = pp_a;
    post_a[0] = pp_post_a;
    let (_, _, child_post_a) = compute_bone_stages(
        &bones,
        &sbb,
        1,
        Vec3::ZERO,
        Quat::IDENTITY,
        Vec3::ONE,
        Some(Quat::IDENTITY),
        &pre_a,
        &post_a,
    )
    .unwrap();

    let mut pre_b = vec![Mat4::IDENTITY; 2];
    let mut post_b = vec![Mat4::IDENTITY; 2];
    let (_, pp_b, pp_post_b) = compute_bone_stages(
        &bones,
        &sbb,
        0,
        Vec3::ZERO,
        Quat::IDENTITY,
        Vec3::ONE,
        Some(Quat::from_rotation_y(1.1)),
        &pre_b,
        &post_b,
    )
    .unwrap();
    pre_b[0] = pp_b;
    post_b[0] = pp_post_b;
    let (_, _, child_post_b) = compute_bone_stages(
        &bones,
        &sbb,
        1,
        Vec3::ZERO,
        Quat::IDENTITY,
        Vec3::ONE,
        Some(Quat::from_rotation_y(1.1)),
        &pre_b,
        &post_b,
    )
    .unwrap();

    let (_, child_rot_a, child_pos_a) = child_post_a.to_scale_rotation_translation();
    let (_, child_rot_b, child_pos_b) = child_post_b.to_scale_rotation_translation();
    assert!(child_pos_a.distance(child_pos_b) < 1.0e-5);
    assert!(child_rot_a.dot(child_rot_b).abs() > 0.9999);
}

#[test]
fn spherical_billboard_bone_tracks_camera_motion() {
    let bones = vec![M2Bone {
        key_bone_id: -1,
        flags: 0x8,
        parent_bone_id: -1,
        submesh_id: 0,
        pivot: [0.0, 0.0, 0.0],
    }];
    let sbb = vec![true];

    fn cam_bb_rot(camera_rotation: Quat) -> Quat {
        let forward = camera_rotation * -Vec3::Z;
        let right = forward.cross(Vec3::Y).normalize();
        let up = right.cross(forward).normalize();
        Quat::from_mat3(&Mat3::from_cols(right, up, forward))
            * Quat::from_rotation_y(-std::f32::consts::FRAC_PI_2)
    }

    fn root_stage(bones: &[M2Bone], sbb: &[bool], camera_rotation: Quat) -> Mat4 {
        let pre = [Mat4::IDENTITY];
        let post = [Mat4::IDENTITY];
        let (_, _, stage) = compute_bone_stages(
            bones,
            sbb,
            0,
            Vec3::ZERO,
            Quat::IDENTITY,
            Vec3::ONE,
            Some(cam_bb_rot(camera_rotation)),
            &pre,
            &post,
        )
        .unwrap();
        stage
    }

    let focus = Vec3::Y * 0.5;
    let camera_rot_a = Transform::from_translation(Vec3::new(2.2, 1.1, 2.8))
        .looking_at(focus, Vec3::Y)
        .rotation;
    let camera_rot_b = Transform::from_translation(Vec3::new(-2.4, 1.7, 1.9))
        .looking_at(focus, Vec3::Y)
        .rotation;

    let stage_a = root_stage(&bones, &sbb, camera_rot_a);
    let stage_b = root_stage(&bones, &sbb, camera_rot_b);
    let (_, rot_a, pos_a) = stage_a.to_scale_rotation_translation();
    let (_, rot_b, pos_b) = stage_b.to_scale_rotation_translation();
    assert!(
        pos_a.distance(pos_b) < 1.0e-6,
        "billboard pivot translation should remain stable when camera moves"
    );
    assert!(
        rot_a.dot(rot_b).abs() < 0.999,
        "billboard orientation should update when camera moves"
    );
}
