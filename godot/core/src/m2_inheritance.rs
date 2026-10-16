//! Authored low-bit parent contribution, in engine axes (WoW x,z,-y).
use glam::{Affine3A, Vec3};

/// Select the parent skin transform before applying a child's pivoted animation.
/// Roots have no parent contribution and must not enter this branch.
pub fn select_parent(parent: Affine3A, model: Affine3A, pivot: Vec3, flags: u32) -> Affine3A {
    if flags & 7 == 0 {
        return parent;
    }
    let mut selected = parent;
    let parent_axes = [
        parent.matrix3.x_axis,
        parent.matrix3.y_axis,
        parent.matrix3.z_axis,
    ];
    let model_axes = [
        model.matrix3.x_axis,
        model.matrix3.y_axis,
        model.matrix3.z_axis,
    ];
    let axes = std::array::from_fn::<_, 3, _>(|axis| match flags & 6 {
        2 => parent_axes[axis].normalize_or_zero() * model_axes[axis].length(),
        4 => model_axes[axis].normalize_or_zero() * parent_axes[axis].length(),
        6 => model_axes[axis],
        _ => parent_axes[axis],
    });
    selected.matrix3 = glam::Mat3A::from_cols(axes[0], axes[1], axes[2]);
    selected.translation = if flags & 1 != 0 {
        model.translation
    } else {
        parent.transform_point3a(pivot.into()) - selected.matrix3 * glam::Vec3A::from(pivot)
    };
    selected
}

#[cfg(test)]
mod tests {
    use super::*;
    use glam::{Quat, Vec3A};

    fn close(actual: Vec3, expected: Vec3) {
        assert!(
            (actual - expected).length() < 1e-5,
            "{actual:?} != {expected:?}"
        );
    }

    #[test]
    fn a6_parent_uniform_scale_and_unflagged_control() {
        let parent = Affine3A::from_scale(Vec3::splat(2.0));
        for (flags, length) in [(0, 2.0), (2, 1.0)] {
            let result = select_parent(parent, Affine3A::IDENTITY, Vec3::ZERO, flags);
            close(result.transform_point3(Vec3::X), Vec3::X * length);
        }
    }

    #[test]
    fn a6_translation_rotation_and_pivot_compensation() {
        // WoW rotation about +Z becomes engine rotation about +Y.
        let parent = Affine3A::from_scale_rotation_translation(
            Vec3::splat(2.0),
            Quat::from_rotation_y(std::f32::consts::FRAC_PI_2),
            Vec3::new(3.0, 4.0, -5.0),
        );
        let pivot = Vec3::new(1.0, 0.0, -2.0);
        let transformed_pivot = parent.transform_point3(pivot);
        let bit1 = select_parent(parent, Affine3A::IDENTITY, pivot, 1);
        close(bit1.translation.into(), Vec3::ZERO);
        close(bit1.matrix3.x_axis.into(), Vec3::new(0.0, 0.0, -2.0));
        for (flags, axis) in [(2, Vec3::NEG_Z), (4, Vec3::X * 2.0), (6, Vec3::X)] {
            let result = select_parent(parent, Affine3A::IDENTITY, pivot, flags);
            close(result.matrix3.x_axis.into(), axis);
            close(result.transform_point3(pivot), transformed_pivot);
        }
        assert_eq!(select_parent(parent, Affine3A::IDENTITY, pivot, 0), parent);
    }

    #[test]
    fn a6_model_basis_and_translation_not_identity() {
        let model = Affine3A::from_scale_rotation_translation(
            Vec3::new(3.0, 4.0, 5.0),
            Quat::from_rotation_y(0.4),
            Vec3::new(7.0, 8.0, 9.0),
        );
        let parent = model * Affine3A::from_scale(Vec3::splat(2.0));
        for flags in [2, 6] {
            let result = select_parent(parent, model, Vec3::ZERO, flags | 1);
            close(result.matrix3.x_axis.into(), model.matrix3.x_axis.into());
            close(result.matrix3.y_axis.into(), model.matrix3.y_axis.into());
            close(result.matrix3.z_axis.into(), model.matrix3.z_axis.into());
            assert_eq!(result.translation, Vec3A::from(Vec3::new(7.0, 8.0, 9.0)));
        }
    }
}
