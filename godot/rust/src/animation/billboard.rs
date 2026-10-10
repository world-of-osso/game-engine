//! Billboard bones turned toward the current camera before a pose is written
//! (`game_engine_core::m2_billboard`); their children follow.
use game_engine_core::{m2, m2_billboard};
use glam::{Affine3A, Mat3, Quat, Vec3};
use godot::{
    builtin::{Quaternion, Transform3D, Vector3},
    classes::Skeleton3D,
    prelude::*,
};

use super::BonePose;

/// A model's bone flags and parents, when some bone is a billboard.
pub(super) struct Billboards {
    flags: Vec<u32>,
    parents: Vec<i16>,
}

impl Billboards {
    pub fn new(model: &m2::Model) -> Option<Self> {
        model
            .bones
            .iter()
            .any(|bone| m2_billboard::is_billboard(bone.flags))
            .then(|| Self {
                flags: model.bones.iter().map(|bone| bone.flags).collect(),
                parents: model.bones.iter().map(|bone| bone.parent_bone_id).collect(),
            })
    }

    /// Rewrites the local poses of billboard bones so that, under `skeleton` as the
    /// current camera sees it, they face the camera. Without a camera nothing changes.
    pub fn apply(&self, poses: &mut [BonePose], skeleton: &Gd<Skeleton3D>) {
        let Some(view_from_skeleton) = view_from_skeleton(skeleton) else {
            return;
        };
        self.apply_in_view(poses, view_from_skeleton);
    }

    fn apply_in_view(&self, poses: &mut [BonePose], view_from_skeleton: Affine3A) {
        let locals: Vec<Affine3A> = poses.iter().map(|pose| affine(*pose)).collect();
        let mut globals = vec![None; poses.len()];
        for index in 0..poses.len() {
            self.global(index, &locals, &mut globals, view_from_skeleton);
        }
        for (index, pose) in poses.iter_mut().enumerate() {
            if !m2_billboard::is_billboard(self.flags[index]) {
                continue;
            }
            let parent = match self.parents[index] {
                parent if parent >= 0 => globals[parent as usize].unwrap_or(Affine3A::IDENTITY),
                _ => Affine3A::IDENTITY,
            };
            let global = globals[index].unwrap_or(locals[index]);
            // Authored zero-scale bones (portal165651 starts this way) are invisible.
            // Their orientation is undefined: preserve the sampled TRS until they
            // expand, rather than decomposing a zero matrix into a quaternion.
            if global.matrix3 == glam::Mat3A::ZERO {
                continue;
            }
            *pose = bone_pose(parent.inverse() * global);
        }
    }

    /// Skeleton-space transform of bone `index`, billboards applied down its chain.
    fn global(
        &self,
        index: usize,
        locals: &[Affine3A],
        globals: &mut [Option<Affine3A>],
        view_from_skeleton: Affine3A,
    ) -> Affine3A {
        if let Some(global) = globals[index] {
            return global;
        }
        let parent = match self.parents[index] {
            parent if parent >= 0 => {
                self.global(parent as usize, locals, globals, view_from_skeleton)
            }
            _ => Affine3A::IDENTITY,
        };
        let mut global = parent * locals[index];
        let flags = self.flags[index];
        if m2_billboard::is_billboard(flags) {
            let local = Mat3::from(locals[index].matrix3);
            let view = m2_billboard::billboard_bone(flags, view_from_skeleton * global, local);
            global = view_from_skeleton.inverse() * view;
        }
        globals[index] = Some(global);
        global
    }
}

fn view_from_skeleton(skeleton: &Gd<Skeleton3D>) -> Option<Affine3A> {
    if !skeleton.is_inside_tree() {
        return None;
    }
    let camera = skeleton.get_viewport()?.get_camera_3d()?;
    Some(
        glam_affine(camera.get_global_transform()).inverse()
            * glam_affine(skeleton.get_global_transform()),
    )
}

fn glam_affine(transform: Transform3D) -> Affine3A {
    let column = |v: Vector3| Vec3::new(v.x, v.y, v.z);
    Affine3A::from_cols(
        column(transform.basis.col_a()).into(),
        column(transform.basis.col_b()).into(),
        column(transform.basis.col_c()).into(),
        column(transform.origin).into(),
    )
}

fn affine(pose: BonePose) -> Affine3A {
    let q = pose.rotation;
    Affine3A::from_scale_rotation_translation(
        Vec3::new(pose.scale.x, pose.scale.y, pose.scale.z),
        Quat::from_xyzw(q.x, q.y, q.z, q.w),
        Vec3::new(pose.position.x, pose.position.y, pose.position.z),
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::animation::AnimationState;
    use std::{fs, path::PathBuf};

    #[test]
    fn toy_live_portal_billboards_preserve_authored_scale_through_growth() {
        let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("../../data/products/wow/dcfc90fffd79ba00406ae46f5f657592/models");
        for fdid in [192572, 165651, 192573, 165622] {
            let read =
                |name: String| fs::read(root.join(name)).expect("authenticated portal asset");
            let model = m2::parse_model_with_skeleton(
                &read(format!("{fdid}.m2")),
                &read(format!("{fdid}00.skin")),
                None,
                |child| fs::read(root.join(format!("{child}.anim"))).ok(),
            )
            .expect("authored Ethereal Portal visual model");
            let Some(billboards) = Billboards::new(&model) else {
                continue;
            };
            let mut animation = AnimationState::new(&model).unwrap();
            for time in [0.0, 10.0, 100.0, 500.0, 1000.0] {
                animation.seek_fixed_time_ms(time).unwrap();
                let mut poses = animation.sampled_poses();
                let authored = poses.clone();
                billboards.apply_in_view(&mut poses, Affine3A::IDENTITY);
                for (bone, pose) in poses.into_iter().enumerate() {
                    assert!(affine(pose).is_finite(), "portal {fdid} time={time}");
                    if authored[bone].scale == Vector3::ZERO {
                        assert_eq!(pose.scale, Vector3::ZERO);
                        assert_eq!(pose.position, authored[bone].position);
                        assert_eq!(pose.rotation, authored[bone].rotation);
                    }
                }
            }
        }
    }
}

fn bone_pose(transform: Affine3A) -> BonePose {
    let (scale, rotation, position) = transform.to_scale_rotation_translation();
    BonePose {
        position: Vector3::new(position.x, position.y, position.z),
        rotation: Quaternion::new(rotation.x, rotation.y, rotation.z, rotation.w),
        scale: Vector3::new(scale.x, scale.y, scale.z),
    }
}
