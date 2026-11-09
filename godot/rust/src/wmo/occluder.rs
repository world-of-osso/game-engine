//! Occlusion-culling geometry of a WMO: its exterior shell. Portal culling cannot hide
//! one WMO behind another (Stormwind's far interiors pass their doorways but sit behind
//! nearer districts), so Godot's occlusion buffer gets the opaque batches of every
//! EXTERIOR group. Interior groups (rooms, dungeons) and blended or alpha-keyed batches,
//! which have holes, never occlude. Every occluder added or removed rebuilds the occlusion
//! BVH inside the viewport render, so only shells of at least `MIN_OCCLUDER_YARDS` get
//! one: in Stormwind the 18 such WMOs of 61 hold 84% of the shell triangles and cull the
//! same draws as all 61.

use godot::{
    classes::{ArrayOccluder3D, OccluderInstance3D},
    prelude::*,
};

/// MOMT blend mode of an opaque batch.
const OPAQUE_BLEND_MODE: u32 = 0;

/// Smallest bounding-box diagonal (yards) of a shell that gets an occluder.
const MIN_OCCLUDER_YARDS: f32 = 100.0;

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

    /// Whether the shell's bounding box spans at least `MIN_OCCLUDER_YARDS` diagonally.
    fn is_large(&self) -> bool {
        let Some(&first) = self.positions.first() else {
            return false;
        };
        let (min, max) = self
            .positions
            .iter()
            .fold((first, first), |(min, max), &p| {
                (min.coord_min(p), max.coord_max(p))
            });
        (max - min).length() >= MIN_OCCLUDER_YARDS
    }

    /// One occluder instance for the whole shell; `None` for a small or empty shell.
    pub(super) fn into_node(self) -> Option<Gd<OccluderInstance3D>> {
        if self.indices.is_empty() || !self.is_large() {
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
    fn only_shells_spanning_the_minimum_diagonal_are_large() {
        let mut house = OccluderGeometry::default();
        house.append(
            &[[0.0, 0.0, 0.0], [40.0, 0.0, 0.0], [0.0, 30.0, 20.0]],
            &[0, 1, 2],
        );
        assert!(!house.is_large(), "a 54-yard house");
        let mut district = OccluderGeometry::default();
        district.append(
            &[[0.0, 0.0, 0.0], [90.0, 0.0, 0.0], [0.0, 20.0, 50.0]],
            &[0, 1, 2],
        );
        assert!(district.is_large(), "a 105-yard district shell");
        assert!(!OccluderGeometry::default().is_large());
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
        assert_eq!(geometry.indices, vec![0, 1, 2, 3, 5, 4]);
        assert_eq!(
            geometry.positions[geometry.indices[4] as usize],
            Vector3::new(5.0, 1.0, 0.0)
        );
    }
}
