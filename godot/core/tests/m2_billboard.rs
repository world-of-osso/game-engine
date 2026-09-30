//! M2 billboard bones (WebWowViewerCpp `calcBoneMatrix`, solarityclient
//! `billboard_transform`), in Godot view space: x right, y up, z toward the camera.
use game_engine_core::m2_billboard::{billboard_bone, is_billboard};
use glam::{Affine3A, Mat3, Quat, Vec3};

fn near(actual: Vec3, expected: Vec3) -> bool {
    actual.distance(expected) < 1e-5
}

/// A bone turned away from the camera, scaled 2 / 3 / 4 along its axes, at (1, 2, -5).
fn tilted_bone() -> Affine3A {
    Affine3A::from_scale_rotation_translation(
        Vec3::new(2.0, 3.0, 4.0),
        Quat::from_euler(glam::EulerRot::YXZ, 0.7, -0.4, 0.25),
        Vec3::new(1.0, 2.0, -5.0),
    )
}

#[test]
fn billboard_flags_are_the_spherical_and_locked_modes() {
    // Torch halo 0x208, torch flame 0x8, candle flame 0x40 (club_1h_torch_a_01, candle01).
    for flags in [0x8, 0x208, 0x10, 0x20, 0x40] {
        assert!(is_billboard(flags), "{flags:#x}");
    }
    for flags in [0x0, 0x200, 0x280, 0x1, 0x4] {
        assert!(!is_billboard(flags), "{flags:#x}");
    }
}

#[test]
fn spherical_bone_faces_the_camera_and_keeps_its_pivot_and_scale() {
    let bone = billboard_bone(0x8, tilted_bone(), Mat3::IDENTITY);
    // WoW X (Godot x) toward the camera, WoW Z (Godot y) up, WoW Y (Godot -z) right.
    assert!(near(bone.matrix3.x_axis.into(), Vec3::new(0.0, 0.0, 2.0)));
    assert!(near(bone.matrix3.y_axis.into(), Vec3::new(0.0, 3.0, 0.0)));
    assert!(near(bone.matrix3.z_axis.into(), Vec3::new(-4.0, 0.0, 0.0)));
    assert!(near(bone.translation.into(), Vec3::new(1.0, 2.0, -5.0)));
}

#[test]
fn transformed_spherical_bone_keeps_its_own_animated_rotation() {
    // Rolled a quarter turn about its WoW X (Godot x): WoW Z now lies along WoW -Y.
    let local = Mat3::from_rotation_x(std::f32::consts::FRAC_PI_2);
    let bone = billboard_bone(0x208, tilted_bone(), local);
    assert!(near(bone.matrix3.x_axis.into(), Vec3::new(0.0, 0.0, 2.0)));
    // Godot y (WoW Z) now points along the view of WoW -Y: left.
    assert!(near(bone.matrix3.y_axis.into(), Vec3::new(-3.0, 0.0, 0.0)));
    assert!(near(bone.matrix3.z_axis.into(), Vec3::new(0.0, -4.0, 0.0)));
}

#[test]
fn z_locked_bone_keeps_its_axis_and_turns_about_it_toward_the_camera() {
    // A candle flame bone standing upright, yawed 60 degrees away from the camera.
    let yawed = Affine3A::from_rotation_translation(
        Quat::from_rotation_y(1.0472),
        Vec3::new(0.0, 0.0, -3.0),
    );
    let bone = billboard_bone(0x40, yawed, Mat3::IDENTITY);
    assert!(near(bone.matrix3.y_axis.into(), Vec3::Y));
    assert!(near(bone.matrix3.x_axis.into(), Vec3::Z));
    assert!(near(bone.matrix3.z_axis.into(), Vec3::NEG_X));
}

#[test]
fn non_billboard_bone_is_unchanged() {
    assert_eq!(
        billboard_bone(0x200, tilted_bone(), Mat3::IDENTITY),
        tilted_bone()
    );
}
