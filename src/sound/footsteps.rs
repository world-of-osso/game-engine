use std::collections::HashMap;
use std::path::{Path, PathBuf};

use bevy::prelude::{Assets, AudioSource, Handle};
pub use game_engine::footstep_data::{
    FootstepCatalog, FootstepCatalogEntry, FootstepCreature, FootstepMovement, FootstepRequest,
    FootstepSurface, classify_model_creature, classify_player_creature,
    classify_surface_from_texture_path, movement_from_anim,
};

#[path = "footstep_cache.rs"]
mod cache;

#[derive(Debug, Default)]
pub struct LoadedFootstepCatalog {
    catalog: FootstepCatalog,
    handles: Vec<Handle<AudioSource>>,
}

impl LoadedFootstepCatalog {
    pub fn is_empty(&self) -> bool {
        self.handles.is_empty()
    }

    pub fn select_handle(&self, request: FootstepRequest) -> Option<Handle<AudioSource>> {
        let idx = self.catalog.select_index(request)?;
        self.handles.get(idx).cloned()
    }
}

pub fn load_wow_footstep_catalog(audio_assets: &mut Assets<AudioSource>) -> LoadedFootstepCatalog {
    let mut loaded = LoadedFootstepCatalog::default();
    let mut counts = HashMap::new();
    let rows = match cache::load_cached_footstep_rows(Path::new("data/community-listfile.csv")) {
        Ok(rows) => rows,
        Err(err) => {
            eprintln!("Failed to load footstep listfile cache: {err}");
            cache::load_footstep_rows_uncached(Path::new("data/community-listfile.csv"))
                .unwrap_or_default()
        }
    };
    for (fdid, path) in rows {
        try_push_catalog_entry(audio_assets, &mut loaded, &mut counts, fdid, &path);
    }
    loaded
}

fn parse_listfile_lines(data: &str) -> impl Iterator<Item = (u32, &str)> {
    data.lines().filter_map(|line| {
        let (fdid, path) = line.split_once(';')?;
        let fdid = fdid.parse().ok()?;
        Some((fdid, path))
    })
}

fn try_push_catalog_entry(
    audio_assets: &mut Assets<AudioSource>,
    loaded: &mut LoadedFootstepCatalog,
    counts: &mut HashMap<(FootstepCreature, FootstepSurface), usize>,
    fdid: u32,
    path: &str,
) {
    if !is_supported_footstep_path(path) {
        return;
    }
    let Some(entry) = FootstepCatalogEntry::from_path(fdid, path) else {
        return;
    };
    if bucket_full(counts, entry.creature, entry.surface) {
        return;
    }
    let Some(bytes) = load_footstep_bytes(fdid, path) else {
        return;
    };
    loaded.catalog.entries.push(entry.clone());
    loaded.handles.push(audio_assets.add(AudioSource {
        bytes: bytes.into(),
    }));
    increment_bucket(counts, entry.creature, entry.surface);
}

fn is_supported_footstep_path(path: &str) -> bool {
    let lower = path.to_ascii_lowercase();
    lower.ends_with(".ogg")
        && !lower.ends_with(".ogg.meta")
        && (lower.starts_with("sound/character/footsteps/")
            || (lower.starts_with("sound/creature/") && lower.contains("footstep")))
}

fn bucket_full(
    counts: &HashMap<(FootstepCreature, FootstepSurface), usize>,
    creature: FootstepCreature,
    surface: FootstepSurface,
) -> bool {
    counts
        .get(&(creature, surface))
        .copied()
        .unwrap_or_default()
        >= 6
}

fn increment_bucket(
    counts: &mut HashMap<(FootstepCreature, FootstepSurface), usize>,
    creature: FootstepCreature,
    surface: FootstepSurface,
) {
    *counts.entry((creature, surface)).or_default() += 1;
}

fn load_footstep_bytes(fdid: u32, path: &str) -> Option<Vec<u8>> {
    let out_path = footstep_output_path(fdid, path);
    let local = game_engine::asset::asset_cache::file_at_path(fdid, &out_path)?;
    std::fs::read(local).ok()
}

fn footstep_output_path(fdid: u32, path: &str) -> PathBuf {
    let ext = Path::new(path)
        .extension()
        .and_then(|ext| ext.to_str())
        .unwrap_or("ogg");
    PathBuf::from("data/sounds/footsteps").join(format!("{fdid}.{ext}"))
}
