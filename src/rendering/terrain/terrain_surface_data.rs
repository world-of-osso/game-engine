//! Bevy-free selection of authored terrain effects and footstep surfaces.

use crate::asset::adt_format::adt_tex::{AdtTexData, ChunkTexLayers};
use crate::sound_footsteps::{FootstepSurface, classify_surface_from_texture_path};

/// Select the last nonzero effect with maximum layer weight.
pub fn dominant_effect_id(chunk: &ChunkTexLayers) -> Option<u32> {
    let mut best = None;
    let mut best_weight = 0u64;
    for (layer_idx, layer) in chunk.layers.iter().enumerate() {
        if layer.effect_id == 0 {
            continue;
        }
        let weight = layer_weight(layer_idx, layer.alpha_map.as_deref());
        if weight >= best_weight {
            best = Some(layer.effect_id);
            best_weight = weight;
        }
    }
    best
}

/// Select the last texture with maximum layer weight; an invalid index ends selection.
pub fn dominant_texture_fdid(tex_data: &AdtTexData, chunk: &ChunkTexLayers) -> Option<u32> {
    let mut best = None;
    let mut best_weight = 0u64;
    for (layer_idx, layer) in chunk.layers.iter().enumerate() {
        let fdid = tex_data
            .texture_fdids
            .get(layer.texture_index as usize)
            .copied()?;
        let weight = layer_weight(layer_idx, layer.alpha_map.as_deref());
        if weight >= best_weight {
            best = Some(fdid);
            best_weight = weight;
        }
    }
    best
}

fn layer_weight(layer_idx: usize, alpha_map: Option<&[u8]>) -> u64 {
    if layer_idx == 0 {
        1_000_000
    } else {
        alpha_map
            .map(|alpha| alpha.iter().map(|v| u64::from(*v)).sum())
            .unwrap_or_default()
    }
}

/// Resolve the dominant effect first, then classify its dominant texture path.
pub fn dominant_surface_for_chunk_with_resolver<'a>(
    tex_data: &AdtTexData,
    chunk: &ChunkTexLayers,
    resolve_effect_surface: impl Fn(u32) -> Option<FootstepSurface>,
    resolve_texture_path: impl Fn(u32) -> Option<&'a str>,
) -> FootstepSurface {
    if let Some(effect_id) = dominant_effect_id(chunk)
        && let Some(surface) = resolve_effect_surface(effect_id)
    {
        return surface;
    }
    let Some(fdid) = dominant_texture_fdid(tex_data, chunk) else {
        return FootstepSurface::Dirt;
    };
    let Some(path) = resolve_texture_path(fdid) else {
        return FootstepSurface::Dirt;
    };
    classify_surface_from_texture_path(path)
}
