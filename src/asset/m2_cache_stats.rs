use bevy::mesh::{Indices, Mesh, VertexAttributeValues};

use super::{M2_MODEL_CACHE, M2Model};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct ModelCacheStats {
    pub entries: usize,
    pub est_cpu_bytes: u64,
}

pub fn model_cache_stats() -> ModelCacheStats {
    let Some(cache) = M2_MODEL_CACHE.get() else {
        return ModelCacheStats::default();
    };
    let cache = cache.lock().unwrap();
    ModelCacheStats {
        entries: cache.len(),
        est_cpu_bytes: cache
            .values()
            .filter_map(|result| result.as_ref().ok())
            .map(estimate_model_cpu_bytes)
            .sum(),
    }
}

fn estimate_model_cpu_bytes(model: &M2Model) -> u64 {
    estimate_batches_cpu_bytes(model)
        + slice_bytes(model.bones.as_slice())
        + slice_bytes(model.sequences.as_slice())
        + slice_bytes(model.bone_tracks.as_slice())
        + slice_bytes(model.global_sequences.as_slice())
        + slice_bytes(model.particle_emitters.as_slice())
        + slice_bytes(model.attachments.as_slice())
        + slice_bytes(model.attachment_lookup.as_slice())
}

fn estimate_batches_cpu_bytes(model: &M2Model) -> u64 {
    model
        .batches
        .iter()
        .map(|batch| estimate_mesh_cpu_bytes(&batch.mesh))
        .sum()
}

fn estimate_mesh_cpu_bytes(mesh: &Mesh) -> u64 {
    mesh.attributes()
        .map(|(_, values)| estimate_vertex_attribute_bytes(values))
        .sum::<u64>()
        + mesh.indices().map_or(0, estimate_indices_bytes)
}

fn estimate_indices_bytes(indices: &Indices) -> u64 {
    match indices {
        Indices::U16(values) => slice_bytes(values.as_slice()),
        Indices::U32(values) => slice_bytes(values.as_slice()),
    }
}

fn estimate_vertex_attribute_bytes(values: &VertexAttributeValues) -> u64 {
    match values {
        VertexAttributeValues::Float32(values) => slice_bytes(values.as_slice()),
        VertexAttributeValues::Sint32(values) => slice_bytes(values.as_slice()),
        VertexAttributeValues::Uint32(values) => slice_bytes(values.as_slice()),
        VertexAttributeValues::Float32x2(values) => slice_bytes(values.as_slice()),
        VertexAttributeValues::Sint32x2(values) => slice_bytes(values.as_slice()),
        VertexAttributeValues::Uint32x2(values) => slice_bytes(values.as_slice()),
        VertexAttributeValues::Float32x3(values) => slice_bytes(values.as_slice()),
        VertexAttributeValues::Sint32x3(values) => slice_bytes(values.as_slice()),
        VertexAttributeValues::Uint32x3(values) => slice_bytes(values.as_slice()),
        VertexAttributeValues::Float32x4(values) => slice_bytes(values.as_slice()),
        VertexAttributeValues::Sint32x4(values) => slice_bytes(values.as_slice()),
        VertexAttributeValues::Uint32x4(values) => slice_bytes(values.as_slice()),
        VertexAttributeValues::Sint16x2(values) => slice_bytes(values.as_slice()),
        VertexAttributeValues::Unorm16x2(values) => slice_bytes(values.as_slice()),
        VertexAttributeValues::Uint16x2(values) => slice_bytes(values.as_slice()),
        VertexAttributeValues::Snorm16x2(values) => slice_bytes(values.as_slice()),
        VertexAttributeValues::Sint16x4(values) => slice_bytes(values.as_slice()),
        VertexAttributeValues::Snorm16x4(values) => slice_bytes(values.as_slice()),
        VertexAttributeValues::Uint16x4(values) => slice_bytes(values.as_slice()),
        VertexAttributeValues::Unorm16x4(values) => slice_bytes(values.as_slice()),
        VertexAttributeValues::Sint8x2(values) => slice_bytes(values.as_slice()),
        VertexAttributeValues::Snorm8x2(values) => slice_bytes(values.as_slice()),
        VertexAttributeValues::Uint8x2(values) => slice_bytes(values.as_slice()),
        VertexAttributeValues::Unorm8x2(values) => slice_bytes(values.as_slice()),
        VertexAttributeValues::Sint8x4(values) => slice_bytes(values.as_slice()),
        VertexAttributeValues::Snorm8x4(values) => slice_bytes(values.as_slice()),
        VertexAttributeValues::Uint8x4(values) => slice_bytes(values.as_slice()),
        VertexAttributeValues::Unorm8x4(values) => slice_bytes(values.as_slice()),
    }
}

fn slice_bytes<T>(values: &[T]) -> u64 {
    std::mem::size_of_val(values) as u64
}
