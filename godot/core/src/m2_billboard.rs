//! M2 billboard bones: bone flags 0x8 (spherical) and 0x10 / 0x20 / 0x40 (cylindrical,
//! locked to the bone's X / Y / Z axis) turn a bone toward the camera.
//!
//! Follows WebWowViewerCpp `AnimationManager::calcBoneMatrix`
//! (`managers/animationManager.cpp:497-647`) and solarityclient `billboard_transform`
//! (`rendering/src/model/m2_animation/pose.rs:531-610`), which agree. Both rebuild the
//! bone's axes in the client's left-handed view space (x right, y up, z into the
//! screen); here that space is reached from Godot's right-handed one (z toward the
//! camera) by negating z.
use glam::{Affine3A, Mat3, Vec3};

const SPHERICAL: u32 = 0x8;
const LOCK_X: u32 = 0x10;
const LOCK_Y: u32 = 0x20;
const LOCK_Z: u32 = 0x40;
/// Transformed bones (0x80 | 0x200) keep their animated orientation inside a
/// spherical billboard.
const ANIMATED: u32 = 0x280;

/// Whether bone `flags` select a billboard this module applies.
pub fn is_billboard(flags: u32) -> bool {
    matches!(flags & 0x78, SPHERICAL | LOCK_X | LOCK_Y | LOCK_Z)
}

/// Right-handed Godot view space <-> the client's left-handed view space.
fn flip_z(v: Vec3) -> Vec3 {
    Vec3::new(v.x, v.y, -v.z)
}

fn normalize_or(v: Vec3, fallback: Vec3) -> Vec3 {
    let length_squared = v.length_squared();
    if length_squared.is_finite() && length_squared > 1.0e-6 {
        v / length_squared.sqrt()
    } else {
        fallback
    }
}

/// `view_from_bone` with the billboard of `flags` applied: the bone's axes rebuilt
/// facing the camera, each keeping its length, about the bone origin (its pivot in
/// Godot bone space). `local` is the bone's own animated basis, kept by transformed
/// spherical billboards. Godot bone axes (x, y, z) are WoW bone axes (X, Z, -Y).
pub fn billboard_bone(flags: u32, view_from_bone: Affine3A, local: Mat3) -> Affine3A {
    let basis = Mat3::from(view_from_bone.matrix3);
    // WoW bone axes in the client's view space.
    let wow = [basis.x_axis, -basis.z_axis, basis.y_axis].map(flip_z);
    let Some(axes) = billboard_axes(flags, wow, local) else {
        return view_from_bone;
    };
    let [x, y, z] = axes.map(flip_z);
    Affine3A::from_mat3_translation(
        Mat3::from_cols(
            x * basis.x_axis.length(),
            z * basis.y_axis.length(),
            -y * basis.z_axis.length(),
        ),
        view_from_bone.translation.into(),
    )
}

/// The WoW bone axes a billboard mode rebuilds from `wow`, in the client's view space.
fn billboard_axes(flags: u32, [x, y, z]: [Vec3; 3], local: Mat3) -> Option<[Vec3; 3]> {
    Some(match flags & 0x78 {
        SPHERICAL if flags & ANIMATED != 0 => animated_spherical_axes(local),
        SPHERICAL => [Vec3::NEG_Z, Vec3::X, Vec3::Y],
        LOCK_X => {
            let x = normalize_or(x, Vec3::X);
            let y = normalize_or(Vec3::new(x.y, -x.x, 0.0), Vec3::Y);
            [x, y, normalize_or(y.cross(x), Vec3::Z)]
        }
        LOCK_Y => {
            let y = normalize_or(y, Vec3::Y);
            let x = normalize_or(Vec3::new(-y.y, y.x, 0.0), Vec3::X);
            [x, y, normalize_or(y.cross(x), Vec3::Z)]
        }
        LOCK_Z => {
            let z = normalize_or(z, Vec3::Z);
            let y = normalize_or(Vec3::new(z.y, -z.x, 0.0), Vec3::Y);
            [normalize_or(z.cross(y), Vec3::X), y, z]
        }
        _ => return None,
    })
}

/// The bone's animated WoW axes, remapped as the view basis (x, y, z) -> (y, z, -x).
fn animated_spherical_axes(local: Mat3) -> [Vec3; 3] {
    let wow_local =
        [local.x_axis, -local.z_axis, local.y_axis].map(|godot| Vec3::new(godot.x, -godot.z, godot.y));
    let fallback = [Vec3::NEG_Z, Vec3::X, Vec3::Y];
    std::array::from_fn(|axis| {
        let a = wow_local[axis];
        normalize_or(Vec3::new(a.y, a.z, -a.x), fallback[axis])
    })
}
