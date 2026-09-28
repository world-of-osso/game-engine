use std::path::{Path, PathBuf};

use super::pure::parser::{
    M2Chunks, SkelData, SkinData, parse_chunks, parse_skel_data, parse_skin_full,
};

fn load_skel_data(skel_path: &Path) -> Result<SkelData, String> {
    let data = std::fs::read(skel_path).map_err(|e| format!("Failed to read .skel file: {e}"))?;
    parse_skel_data(&data)
}

fn load_anim_from_md20(md20: &[u8]) -> SkelData {
    SkelData {
        bones: super::m2_anim::parse_bones(md20).unwrap_or_default(),
        sequences: super::m2_anim::parse_sequences(md20).unwrap_or_default(),
        bone_tracks: super::m2_anim::parse_bone_animations(md20).unwrap_or_default(),
        global_sequences: super::m2_anim::parse_global_sequences(md20).unwrap_or_default(),
    }
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
    load_anim_from_md20(chunks.md20)
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
