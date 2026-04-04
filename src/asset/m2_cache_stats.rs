use bevy::mesh::Mesh;

use crate::mesh_asset_stats::{
    estimate_indices_bytes, estimate_vertex_attribute_bytes, slice_bytes,
};

use super::M2Model;
use super::m2_loader::M2_MODEL_CACHE;

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
