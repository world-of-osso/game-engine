use std::path::{Path, PathBuf};

use super::pure::parser::{
    M2Chunks, SkelData, SkinData, load_anim_track_chunks, parse_chunks, parse_skel_data_with_anims,
    parse_skin_full,
};

fn load_skel_data(skel_path: &Path) -> Result<SkelData, String> {
    let data = std::fs::read(skel_path).map_err(|e| format!("Failed to read .skel file: {e}"))?;
    parse_skel_data_with_anims(&data, |fdid| load_anim_file(skel_path, fdid))
}

fn load_anim_from_md20(path: &Path, chunks: &M2Chunks<'_>) -> SkelData {
    let md20 = chunks.md20;
    let sequences = super::m2_anim::parse_sequences(md20).unwrap_or_default();
    let anim_files = load_anim_track_chunks(&sequences, &chunks.afid, b"AFM2", |fdid| {
        load_anim_file(path, fdid)
    });
    let sources = super::m2_anim::sequence_data_sources(&sequences, &anim_files);
    let bone_tracks =
        super::m2_anim::parse_md20_bone_animations(md20, &sources).unwrap_or_default();
    SkelData {
        bones: super::m2_anim::parse_bones(md20).unwrap_or_default(),
        sequences,
        bone_tracks,
        global_sequences: super::m2_anim::parse_global_sequences(md20).unwrap_or_default(),
    }
}

/// An external sequence's `.anim` file (`AFID` FDID), cached beside the model.
fn load_anim_file(model_path: &Path, fdid: u32) -> Option<Vec<u8>> {
    let anim_path = model_path.with_file_name(format!("{fdid}.anim"));
    let loaded = super::super::asset_cache::file_at_path(fdid, &anim_path)
        .ok_or_else(|| format!("not extractable to {}", anim_path.display()))
        .and_then(|path| std::fs::read(&path).map_err(|error| error.to_string()));
    loaded
        .map_err(|error| {
            eprintln!(
                "M2 {}: .anim FDID {fdid}: {error}; its sequence has no keyframes",
                model_path.display()
            )
        })
        .ok()
}

pub(crate) fn load_anim_data(path: &Path, chunks: &M2Chunks<'_>) -> SkelData {
    if let Some(skel_fdid) = chunks.skid {
        let stem = path.file_stem().and_then(|s| s.to_str()).unwrap_or("");
        let skel_path = path.with_file_name(format!("{stem}.skel"));
        super::super::asset_cache::file_at_path(skel_fdid, &skel_path);
        match load_skel_data(&skel_path) {
            Ok(s) => return s,
            Err(e) => eprintln!("Failed to load .skel: {e}"),
        }
    }
    load_anim_from_md20(path, chunks)
}

pub(crate) fn load_skin_data(m2_path: &Path, sfid: &[u32]) -> Option<SkinData> {
    let stem = m2_path.file_stem()?.to_str()?;
    if let Some(&fdid) = sfid.first() {
        let canonical_skin_path = m2_path.with_file_name(format!("{stem}00.skin"));
        if let Some(resolved_path) =
            super::super::asset_cache::file_at_path(fdid, &canonical_skin_path)
            && let Ok(data) = std::fs::read(&resolved_path)
        {
            return parse_skin_full(&data).ok();
        }
        let numeric_skin_path = m2_path.with_file_name(format!("{fdid}.skin"));
        if let Some(resolved_path) =
            super::super::asset_cache::file_at_path(fdid, &numeric_skin_path)
            && let Ok(data) = std::fs::read(&resolved_path)
        {
            return parse_skin_full(&data).ok();
        }
    }
    let skin_path = m2_path.with_file_name(format!("{stem}00.skin"));
    let data = std::fs::read(&skin_path).ok()?;
    parse_skin_full(&data).ok()
}

pub fn ensure_primary_skin_path(m2_path: &Path) -> Option<PathBuf> {
    let data = std::fs::read(m2_path).ok()?;
    let chunks = parse_chunks(&data).ok()?;
    let stem = m2_path.file_stem()?.to_str()?;
    if let Some(&fdid) = chunks.sfid.first() {
        let canonical_skin_path = m2_path.with_file_name(format!("{stem}00.skin"));
        if let Some(path) = super::super::asset_cache::file_at_path(fdid, &canonical_skin_path) {
            return Some(path);
        }
        let numeric_skin_path = m2_path.with_file_name(format!("{fdid}.skin"));
        return super::super::asset_cache::file_at_path(fdid, &numeric_skin_path);
    }
    let skin_path = m2_path.with_file_name(format!("{stem}00.skin"));
    skin_path.exists().then_some(skin_path)
}

#[cfg(test)]
#[path = "../../../tests/unit/asset/m2_file_loader_tests.rs"]
mod tests;
