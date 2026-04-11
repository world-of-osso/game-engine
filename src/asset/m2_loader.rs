//! Top-level M2 model loading: parse chunks, assemble M2Model, cache.
//! This is a submodule of `m2` — `super` refers to `m2`, `super::super` to `asset`.

use std::collections::HashMap;
use std::path::Path;
use std::sync::{Mutex, OnceLock};

use crate::asset::{m2_attach, m2_light, m2_particle};

use super::{M2Chunks, M2Model, ModelCacheKey, build_render_batches, parse_chunks, parse_txid};

fn find_chunk<'a>(data: &'a [u8], needle: &[u8; 4]) -> Option<&'a [u8]> {
    let mut off = 0;
    while off + 8 <= data.len() {
        let tag = &data[off..off + 4];
        let size = super::read_u32(data, off + 4).ok()? as usize;
        let end = off + 8 + size;
        if end > data.len() {
            return None;
        }
        if tag == needle {
            return Some(&data[off + 8..end]);
        }
        off = end;
    }
    None
}

fn load_attachment_data(chunks: &M2Chunks<'_>) -> (Vec<m2_attach::M2Attachment>, Vec<i16>) {
    let attachments = chunks
        .ska1
        .map(m2_attach::parse_ska1_attachments)
        .transpose()
        .unwrap_or_default()
        .filter(|parsed| !parsed.is_empty())
        .unwrap_or_else(|| m2_attach::parse_attachments(chunks.md20).unwrap_or_default());
    let attachment_lookup = chunks
        .ska1
        .map(m2_attach::parse_ska1_attachment_lookup)
        .transpose()
        .unwrap_or_default()
        .filter(|parsed| !parsed.is_empty())
        .unwrap_or_else(|| m2_attach::parse_attachment_lookup(chunks.md20).unwrap_or_default());
    (attachments, attachment_lookup)
}

fn load_skel_attachment_data(
    skel_path: &Path,
) -> Result<(Vec<m2_attach::M2Attachment>, Vec<i16>), String> {
    let data = std::fs::read(skel_path).map_err(|e| format!("Failed to read .skel file: {e}"))?;
    let Some(ska1) = find_chunk(&data, b"SKA1") else {
        return Ok((Vec::new(), Vec::new()));
    };
    Ok((
        m2_attach::parse_ska1_attachments(ska1)?,
        m2_attach::parse_ska1_attachment_lookup(ska1)?,
    ))
}

fn load_model_attachment_data(
    path: &Path,
    chunks: &M2Chunks<'_>,
) -> (Vec<m2_attach::M2Attachment>, Vec<i16>) {
    if !path.starts_with("data/models") {
        return load_attachment_data(chunks);
    }
    if let Some(skel_fdid) = chunks.skid {
        let stem = path.file_stem().and_then(|s| s.to_str()).unwrap_or("");
        let skel_path = path.with_file_name(format!("{stem}.skel"));
        super::super::asset_cache::file_at_path(skel_fdid, &skel_path);
        if let Ok((attachments, attachment_lookup)) = load_skel_attachment_data(&skel_path)
            && (!attachments.is_empty() || !attachment_lookup.is_empty())
        {
            return (attachments, attachment_lookup);
        }
    }
    load_attachment_data(chunks)
}

fn load_m2_uncached_impl(
    path: &Path,
    skin_fdids: &[u32; 3],
    keep_zero_opacity_batches: bool,
) -> Result<M2Model, String> {
    let data = std::fs::read(path).map_err(|e| format!("Failed to read M2 file: {e}"))?;
    let chunks = parse_chunks(&data)?;
    let txid = chunks.txid.map(parse_txid).unwrap_or_default();
    let anim = super::load_anim_data(path, &chunks);
    let batches = build_render_batches(
        chunks.md20,
        path,
        &chunks,
        &txid,
        !anim.bones.is_empty(),
        skin_fdids,
        keep_zero_opacity_batches,
    )?;
    let mut particles = m2_particle::parse_particle_emitters(chunks.md20);
    m2_particle::resolve_texture_fdids(&mut particles, &txid);
    let (attachments, attachment_lookup) = load_model_attachment_data(path, &chunks);
    let lights = m2_light::parse_lights(chunks.md20);
    let (bounding_box_min, bounding_box_max) =
        crate::asset::m2_format::parse_bounding_box(chunks.md20);
    Ok(M2Model {
        batches,
        bones: anim.bones,
        sequences: anim.sequences,
        bone_tracks: anim.bone_tracks,
        global_sequences: anim.global_sequences,
        particle_emitters: particles,
        attachments,
        attachment_lookup,
        lights,
        bounding_box_min,
        bounding_box_max,
    })
}

pub fn load_m2_uncached(path: &Path, skin_fdids: &[u32; 3]) -> Result<M2Model, String> {
    load_m2_uncached_impl(path, skin_fdids, false)
}

pub fn load_skybox_m2_uncached(path: &Path, skin_fdids: &[u32; 3]) -> Result<M2Model, String> {
    load_m2_uncached_impl(path, skin_fdids, true)
}

pub(super) static M2_MODEL_CACHE: OnceLock<Mutex<HashMap<ModelCacheKey, Result<M2Model, String>>>> =
    OnceLock::new();

/// Load an M2 model file (chunked MD21 format) and return per-batch meshes + textures.
pub fn load_m2(path: &Path, skin_fdids: &[u32; 3]) -> Result<M2Model, String> {
    load_m2_cached(path, skin_fdids, false)
}

pub fn load_skybox_m2(path: &Path, skin_fdids: &[u32; 3]) -> Result<M2Model, String> {
    load_m2_cached(path, skin_fdids, true)
}

fn load_m2_cached(
    path: &Path,
    skin_fdids: &[u32; 3],
    keep_zero_opacity_batches: bool,
) -> Result<M2Model, String> {
    let key = ModelCacheKey {
        path: path.to_path_buf(),
        skin_fdids: *skin_fdids,
        keep_zero_opacity_batches,
    };
    let cache = M2_MODEL_CACHE.get_or_init(|| Mutex::new(HashMap::new()));
    if let Some(cached) = cache.lock().unwrap().get(&key).cloned() {
        return cached;
    }
    let loaded = load_m2_uncached_impl(path, skin_fdids, keep_zero_opacity_batches);
    cache.lock().unwrap().insert(key, loaded.clone());
    loaded
}
