//! One spell batch's drape, independent of where its inputs come from: the vertex streams
//! read once per source mesh, and the drape over a floor and a surface query.

use std::collections::HashMap;
use std::sync::Arc;

use game_engine_core::ground_drape::{BatchMesh, DrapedBatch, GroundDrape};
use glam::Affine3A;
use godot::prelude::*;

/// A batch mesh's vertex streams, read from its Godot mesh.
pub(crate) struct BatchArrays {
    pub positions: Vec<[f32; 3]>,
    pub uvs: Vec<[f32; 2]>,
    pub uvs2: Option<Vec<[f32; 2]>>,
    pub colors: Option<Vec<[f32; 4]>>,
    pub indices: Vec<u32>,
    pub normals: Option<Vec<[f32; 3]>>,
}

/// The result of draping one batch.
pub(crate) enum Drape {
    /// Not a flat ground batch; it keeps its mesh.
    NotGround,
    /// No column found a surface yet; the batch keeps its mesh and the kit retries.
    NoSurface,
    /// The draped arrays and whether their winding normals must be negated.
    Draped(DrapedBatch, bool),
}

impl BatchArrays {
    /// Drapes the batch placed by `placement` over `floor_at` and `surface_at`.
    pub fn drape(
        &self,
        drape: &GroundDrape,
        placement: Affine3A,
        ground_y: f32,
        window: f32,
        floor_at: &dyn Fn(f32, f32) -> Option<f32>,
        surface_at: &mut dyn FnMut(f32, f32, f32, f32) -> Option<f32>,
    ) -> Drape {
        let batch = BatchMesh {
            positions: &self.positions,
            uvs: &self.uvs,
            uvs2: self.uvs2.as_deref(),
            colors: self.colors.as_deref(),
            indices: &self.indices,
        };
        let Some(draped) =
            drape.drape_batch(&batch, placement, ground_y, window, floor_at, surface_at)
        else {
            return Drape::NotGround;
        };
        if draped.dropped == 0 {
            return Drape::NoSurface;
        }
        let flip =
            winding_normals_are_inward(&self.positions, &self.indices, self.normals.as_deref());
        Drape::Draped(draped, flip)
    }
}

/// Whether the mesh's winding normals point into the surface. The drawn meshes carry
/// Godot's front-face winding, so a rebuilt mesh must negate them to keep the authored
/// lighting; the mesh's own normal stream decides, so a source that already agrees stays.
pub(crate) fn winding_normals_are_inward(
    positions: &[[f32; 3]],
    indices: &[u32],
    normals: Option<&[[f32; 3]]>,
) -> bool {
    let Some(normals) = normals else {
        return true;
    };
    let mut agreement = 0.0f32;
    for triangle in indices.chunks_exact(3).take(64) {
        let corners = [
            glam::Vec3::from_array(positions[triangle[0] as usize]),
            glam::Vec3::from_array(positions[triangle[1] as usize]),
            glam::Vec3::from_array(positions[triangle[2] as usize]),
        ];
        let winding = (corners[1] - corners[0]).cross(corners[2] - corners[0]);
        let authored = triangle
            .iter()
            .map(|&index| glam::Vec3::from_array(normals[index as usize]))
            .sum::<glam::Vec3>();
        agreement += winding.dot(authored);
    }
    agreement < 0.0
}

/// The vertex streams of `mesh`'s single surface; `None` for any other mesh, or one
/// without positions or UVs, which is not a drapeable batch.
fn read_batch(mesh: &Gd<godot::classes::ArrayMesh>) -> Option<BatchArrays> {
    use godot::classes::mesh::ArrayType;
    if mesh.get_surface_count() != 1 {
        return None;
    }
    let arrays = mesh.surface_get_arrays(0);
    let array = |kind: ArrayType| arrays.get(kind.ord() as usize);
    let positions = array(ArrayType::VERTEX)?
        .try_to::<PackedVector3Array>()
        .ok()?;
    let uvs = array(ArrayType::TEX_UV)?
        .try_to::<PackedVector2Array>()
        .ok()?;
    let positions: Vec<[f32; 3]> = positions
        .as_slice()
        .iter()
        .map(|point| [point.x, point.y, point.z])
        .collect();
    let uvs: Vec<[f32; 2]> = uvs.as_slice().iter().map(|uv| [uv.x, uv.y]).collect();
    let uvs2: Option<Vec<[f32; 2]>> = array(ArrayType::TEX_UV2)
        .and_then(|value| value.try_to::<PackedVector2Array>().ok())
        .map(|uvs| uvs.as_slice().iter().map(|uv| [uv.x, uv.y]).collect());
    let colors: Option<Vec<[f32; 4]>> = array(ArrayType::COLOR)
        .and_then(|value| value.try_to::<PackedColorArray>().ok())
        .map(|colors| {
            colors
                .as_slice()
                .iter()
                .map(|color| [color.r, color.g, color.b, color.a])
                .collect()
        });
    let indices: Vec<u32> = array(ArrayType::INDEX)
        .and_then(|value| value.try_to::<PackedInt32Array>().ok())
        .filter(|indices| !indices.is_empty())
        .map(|indices| {
            indices
                .as_slice()
                .iter()
                .filter_map(|index| (*index >= 0).then_some(*index as u32))
                .collect()
        })
        .unwrap_or_else(|| (0..positions.len() as u32).collect());
    let normals: Option<Vec<[f32; 3]>> = array(ArrayType::NORMAL)
        .and_then(|value| value.try_to::<PackedVector3Array>().ok())
        .map(|normals| normals.as_slice().iter().map(|n| [n.x, n.y, n.z]).collect());
    Some(BatchArrays {
        positions,
        uvs,
        uvs2,
        colors,
        indices,
        normals,
    })
}

/// Most source meshes [`BatchArraysCache`] keeps; reached, it starts over.
const BATCH_ARRAYS_CAPACITY: usize = 512;

/// Vertex streams by source mesh. Kit batch meshes are shared and never change, so their
/// streams are read from Godot once instead of on every drape.
#[derive(Default)]
pub(crate) struct BatchArraysCache {
    arrays: HashMap<InstanceId, Option<Arc<BatchArrays>>>,
}

impl BatchArraysCache {
    /// `mesh`'s streams; `None` when it is not a drapeable batch.
    pub fn read(&mut self, mesh: &Gd<godot::classes::ArrayMesh>) -> Option<Arc<BatchArrays>> {
        let key = mesh.instance_id();
        if let Some(arrays) = self.arrays.get(&key) {
            return arrays.clone();
        }
        if self.arrays.len() >= BATCH_ARRAYS_CAPACITY {
            self.arrays.clear();
        }
        let arrays = read_batch(mesh).map(Arc::new);
        self.arrays.insert(key, arrays.clone());
        arrays
    }
}
