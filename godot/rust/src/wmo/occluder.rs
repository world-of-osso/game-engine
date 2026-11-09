//! Occlusion-culling geometry of a WMO: its exterior shell. Portal culling cannot hide
//! one WMO behind another (Stormwind's far interiors pass their doorways but sit behind
//! nearer districts), so Godot's occlusion buffer gets the opaque batches of every
//! EXTERIOR group. Interior groups (rooms, dungeons) and blended or alpha-keyed batches,
//! which have holes, never occlude.

use godot::{
    classes::{ArrayOccluder3D, OccluderInstance3D},
    prelude::*,
};

/// MOMT blend mode of an opaque batch.
const OPAQUE_BLEND_MODE: u32 = 0;

/// Whether a drawn batch of a group with `exterior` set and material `blend_mode`
/// occludes what lies behind it.
pub(super) fn occludes(exterior: bool, blend_mode: u32) -> bool {
    exterior && blend_mode == OPAQUE_BLEND_MODE
}

/// The occluding batches of one WMO, concatenated in WMO-local engine axes.
#[derive(Default)]
pub(super) struct OccluderGeometry {
    positions: Vec<Vector3>,
    indices: Vec<i32>,
}

impl OccluderGeometry {
    /// Appends one batch's vertices and triangles (indices into those vertices).
    pub(super) fn append(&mut self, positions: &[[f32; 3]], indices: &[u32]) {
        let offset = self.positions.len() as i32;
        self.positions.extend(
            positions
                .iter()
                .map(|&position| Vector3::from_array(position)),
        );
        self.indices
            .extend(indices.iter().map(|&index| index as i32 + offset));
    }

    pub(super) fn triangles(&self) -> usize {
        self.indices.len() / 3
    }

    /// One occluder instance for the whole shell; `None` without occluding batches.
    pub(super) fn into_node(self) -> Option<Gd<OccluderInstance3D>> {
        if self.indices.is_empty() {
            return None;
        }
        let mut occluder = ArrayOccluder3D::new_gd();
        occluder.set_arrays(
            &PackedVector3Array::from(self.positions.as_slice()),
            &PackedInt32Array::from(self.indices.as_slice()),
        );
        let mut node = OccluderInstance3D::new_alloc();
        node.set_name("Occluder");
        node.set_occluder(&occluder);
        Some(node)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn only_opaque_exterior_batches_occlude() {
        assert!(occludes(true, 0));
        assert!(!occludes(true, 1), "alpha-keyed batches have holes");
        assert!(!occludes(true, 2), "blended batches are see-through");
        assert!(
            !occludes(false, 0),
            "interior rooms and dungeons never occlude"
        );
    }

    #[test]
    fn appended_batches_index_their_own_vertices() {
        let mut geometry = OccluderGeometry::default();
        geometry.append(
            &[[0.0, 0.0, 0.0], [1.0, 0.0, 0.0], [0.0, 1.0, 0.0]],
            &[0, 1, 2],
        );
        geometry.append(
            &[[5.0, 0.0, 0.0], [6.0, 0.0, 0.0], [5.0, 1.0, 0.0]],
            &[0, 2, 1],
        );
        assert_eq!(geometry.triangles(), 2);
        assert_eq!(geometry.indices, vec![0, 1, 2, 3, 5, 4]);
        assert_eq!(
            geometry.positions[geometry.indices[4] as usize],
            Vector3::new(5.0, 1.0, 0.0)
        );
    }
}
