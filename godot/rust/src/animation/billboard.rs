//! Authored parent contribution followed by camera-facing billboards.
//! Ordinary bones retain Skeleton3D's authored hierarchy and local TRS.
use game_engine_core::{m2, m2_billboard, m2_inheritance};
use glam::{Affine3A, Mat3, Quat, Vec3};
use godot::{
    builtin::{Transform3D, Vector3},
    classes::Skeleton3D,
    prelude::*,
};

use super::BonePose;
#[cfg(test)]
use godot::builtin::Quaternion;

/// Flags, parents and pivots for models needing parent adjustment or billboarding.
pub(super) struct Billboards {
    flags: Vec<u32>,
    parents: Vec<i16>,
    pivots: Vec<Vec3>,
}

impl Billboards {
    pub fn new(model: &m2::Model) -> Option<Self> {
        model
            .bones
            .iter()
            .any(|bone| {
                m2_billboard::is_billboard(bone.flags)
                    || (bone.parent_bone_id >= 0 && bone.flags & 7 != 0)
            })
            .then(|| Self {
                flags: model.bones.iter().map(|bone| bone.flags).collect(),
                parents: model.bones.iter().map(|bone| bone.parent_bone_id).collect(),
                pivots: model
                    .bones
                    .iter()
                    .map(|bone| {
                        let [x, y, z] = bone.pivot;
                        Vec3::new(x, z, -y)
                    })
                    .collect(),
            })
    }

    /// Exact affine overrides avoid decomposing parent compensation into local TRS:
    /// that decomposition loses shear under a nonuniformly scaled parent. Only flagged
    /// bones are overridden; unflagged descendants still inherit normally in Godot.
    pub fn apply(&self, poses: &[BonePose], skeleton: &mut Gd<Skeleton3D>) {
        let model = if skeleton.is_inside_tree() {
            glam_affine(skeleton.get_global_transform())
        } else {
            Affine3A::IDENTITY
        };
        let view = view_from_skeleton(skeleton);
        let locals: Vec<_> = poses.iter().map(|pose| affine(*pose)).collect();
        let mut globals = vec![None; poses.len()];
        for index in 0..poses.len() {
            let global = self.global(index, &locals, &mut globals, model, view);
            if self.adjusted(index) {
                skeleton
                    .set_bone_global_pose_override_ex(index as i32, godot_transform(global), 1.0)
                    .persistent(true)
                    .done();
            }
        }
    }

    fn adjusted(&self, index: usize) -> bool {
        m2_billboard::is_billboard(self.flags[index])
            || (self.parents[index] >= 0 && self.flags[index] & 7 != 0)
    }

    #[cfg(test)]
    fn apply_in_view(&self, poses: &mut [BonePose], view_from_skeleton: Affine3A) {
        let locals: Vec<Affine3A> = poses.iter().map(|pose| affine(*pose)).collect();
        let mut globals = vec![None; poses.len()];
        for index in 0..poses.len() {
            self.global(
                index,
                &locals,
                &mut globals,
                Affine3A::IDENTITY,
                Some(view_from_skeleton),
            );
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
        model: Affine3A,
        view_from_skeleton: Option<Affine3A>,
    ) -> Affine3A {
        if let Some(global) = globals[index] {
            return global;
        }
        let parent = match self.parents[index] {
            parent if parent >= 0 => {
                self.global(parent as usize, locals, globals, model, view_from_skeleton)
            }
            _ => Affine3A::IDENTITY,
        };
        let flags = self.flags[index];
        let mut global = parent * locals[index];
        if self.parents[index] >= 0 && flags & 7 != 0 {
            let parent_pivot = self.pivots[self.parents[index] as usize];
            let parent_skin = model * parent * Affine3A::from_translation(-parent_pivot);
            let selected =
                m2_inheritance::select_parent(parent_skin, model, self.pivots[index], flags);
            let child_local = Affine3A::from_translation(parent_pivot) * locals[index];
            global = model.inverse() * selected * child_local;
        }
        if m2_billboard::is_billboard(flags)
            && let Some(view_from_skeleton) = view_from_skeleton
        {
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
    fn a6_root_control_and_ordinary_descendant_of_adjusted_bone() {
        let composition = Billboards {
            flags: vec![7, 2, 0],
            parents: vec![-1, 0, 1],
            pivots: vec![Vec3::ZERO; 3],
        };
        let locals = [
            Affine3A::from_scale(Vec3::splat(2.0)),
            Affine3A::IDENTITY,
            Affine3A::from_translation(Vec3::X),
        ];
        let mut globals = vec![None; 3];
        let root = composition.global(0, &locals, &mut globals, Affine3A::IDENTITY, None);
        let child = composition.global(2, &locals, &mut globals, Affine3A::IDENTITY, None);
        assert_eq!(root, locals[0], "root low bits do not select a parent");
        assert_eq!(
            child.transform_point3(Vec3::X),
            Vec3::X * 2.0,
            "unflagged grandchild inherits the corrected child normally"
        );
    }

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

fn godot_transform(transform: Affine3A) -> Transform3D {
    let vector = |v: glam::Vec3A| Vector3::new(v.x, v.y, v.z);
    Transform3D::new(
        godot::builtin::Basis::from_cols(
            vector(transform.matrix3.x_axis),
            vector(transform.matrix3.y_axis),
            vector(transform.matrix3.z_axis),
        ),
        vector(transform.translation),
    )
}

#[cfg(test)]
fn bone_pose(transform: Affine3A) -> BonePose {
    let (scale, rotation, position) = transform.to_scale_rotation_translation();
    BonePose {
        position: Vector3::new(position.x, position.y, position.z),
        rotation: Quaternion::new(rotation.x, rotation.y, rotation.z, rotation.w),
        scale: Vector3::new(scale.x, scale.y, scale.z),
    }
}
