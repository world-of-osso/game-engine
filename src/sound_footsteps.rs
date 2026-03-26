use std::collections::HashMap;
use std::path::{Path, PathBuf};

use bevy::prelude::{Assets, AudioSource, Handle};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum FootstepCreature {
    HumanoidSmall,
    HumanoidMedium,
    HumanoidLarge,
    Hoof,
    Paw,
    Horse,
    Mechanical,
    Water,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum FootstepSurface {
    Grass,
    Dirt,
    Stone,
    Wood,
    Metal,
    Snow,
    Water,
    Mud,
    Carpet,
    Ice,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FootstepMovement {
    Walk,
    Run,
    Strafe,
    Backpedal,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FootstepCatalogEntry {
    pub fdid: u32,
    pub path: String,
    pub creature: FootstepCreature,
    pub surface: FootstepSurface,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct FootstepRequest {
    pub creature: FootstepCreature,
    pub surface: FootstepSurface,
    pub movement: FootstepMovement,
    pub seed: u64,
}

#[derive(Debug, Default)]
pub struct FootstepCatalog {
    pub entries: Vec<FootstepCatalogEntry>,
}

#[derive(Debug, Default)]
pub struct LoadedFootstepCatalog {
    catalog: FootstepCatalog,
    handles: Vec<Handle<AudioSource>>,
}

impl FootstepCatalogEntry {
    pub fn from_path(fdid: u32, path: &str) -> Option<Self> {
        let file_name = Path::new(path).file_name()?.to_str()?.to_ascii_lowercase();
        Some(Self {
            fdid,
            path: path.to_string(),
            creature: classify_catalog_creature(&file_name),
            surface: classify_catalog_surface(&file_name),
        })
    }
}

impl FootstepCatalog {
    pub fn select(&self, request: FootstepRequest) -> Option<&FootstepCatalogEntry> {
        let mut matches: Vec<_> = self
            .entries
            .iter()
            .filter(|entry| creature_matches(request.creature, entry.creature, request.movement))
            .collect();
        matches.sort_by_key(|entry| {
            (
                surface_penalty(request.surface, entry.surface),
                path_rank_for_movement(&entry.path, request.movement),
                creature_penalty(request.creature, entry.creature),
            )
        });
        let best = *matches.first()?;
        let best_score = (
            surface_penalty(request.surface, best.surface),
            path_rank_for_movement(&best.path, request.movement),
            creature_penalty(request.creature, best.creature),
        );
        let tied: Vec<_> = matches
            .into_iter()
            .take_while(|entry| {
                (
                    surface_penalty(request.surface, entry.surface),
                    path_rank_for_movement(&entry.path, request.movement),
                    creature_penalty(request.creature, entry.creature),
                ) == best_score
            })
            .collect();
        Some(tied[(request.seed as usize) % tied.len()])
    }

    pub fn select_index(&self, request: FootstepRequest) -> Option<usize> {
        let selected = self.select(request)?;
        self.entries.iter().position(|entry| entry == selected)
    }
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

pub fn load_wow_footstep_catalog(
    audio_assets: &mut Assets<AudioSource>,
) -> LoadedFootstepCatalog {
    let mut loaded = LoadedFootstepCatalog::default();
    let Ok(listfile) = std::fs::read_to_string("data/community-listfile.csv") else {
        return loaded;
    };
    let mut counts = HashMap::new();
    for (fdid, path) in parse_listfile_lines(&listfile) {
        try_push_catalog_entry(audio_assets, &mut loaded, &mut counts, fdid, path);
    }
    loaded
}

pub fn classify_surface_from_texture_path(path: &str) -> FootstepSurface {
    classify_catalog_surface(&path.to_ascii_lowercase())
}

pub fn classify_player_creature(race: u8) -> FootstepCreature {
    match race {
        6 | 11 | 28 | 30 => FootstepCreature::Hoof,
        7 | 9 | 34 | 35 | 37 => FootstepCreature::HumanoidSmall,
        22 | 25 => FootstepCreature::Paw,
        2 | 8 | 31 | 36 => FootstepCreature::HumanoidLarge,
        _ => FootstepCreature::HumanoidMedium,
    }
}

pub fn classify_model_creature(path: &str) -> FootstepCreature {
    let lower = path.to_ascii_lowercase();
    if is_horse_like(&lower) {
        return FootstepCreature::Horse;
    }
    if is_mechanical(&lower) {
        return FootstepCreature::Mechanical;
    }
    if is_water_like(&lower) {
        return FootstepCreature::Water;
    }
    if is_paw_like(&lower) {
        return FootstepCreature::Paw;
    }
    if is_hoof_like(&lower) {
        return FootstepCreature::Hoof;
    }
    if is_small_humanoid(&lower) {
        return FootstepCreature::HumanoidSmall;
    }
    if is_large_humanoid(&lower) {
        return FootstepCreature::HumanoidLarge;
    }
    FootstepCreature::HumanoidMedium
}

pub fn movement_from_anim(anim_id: u16) -> Option<FootstepMovement> {
    match anim_id {
        4 => Some(FootstepMovement::Walk),
        5 => Some(FootstepMovement::Run),
        11 | 12 => Some(FootstepMovement::Strafe),
        13 => Some(FootstepMovement::Backpedal),
        _ => None,
    }
}

fn classify_catalog_creature(path: &str) -> FootstepCreature {
    if path.contains("horse_footstep")
        || path.contains("mfootstepshorse")
        || path.contains("/horse/")
    {
        return FootstepCreature::Horse;
    }
    if path.contains("footstepfish") {
        return FootstepCreature::Water;
    }
    if path.contains("goblinshredder") || path.contains("golem") || path.contains("spidertank") {
        return FootstepCreature::Mechanical;
    }
    if path.contains("spider") || path.contains("rat") || path.contains("frog") || path.contains("crab") {
        return FootstepCreature::Paw;
    }
    if path.contains("mfootsmall") {
        return FootstepCreature::HumanoidSmall;
    }
    if path.contains("mfoothuge") || path.contains("footstepshuge") {
        return FootstepCreature::HumanoidLarge;
    }
    FootstepCreature::HumanoidMedium
}

fn classify_catalog_surface(path: &str) -> FootstepSurface {
    let lower = path.to_ascii_lowercase();
    if matches_any(&lower, &["metal", "mech", "forge"]) {
        return FootstepSurface::Metal;
    }
    if matches_any(&lower, &["snow", "ice", "frost"]) {
        return if lower.contains("ice") {
            FootstepSurface::Ice
        } else {
            FootstepSurface::Snow
        };
    }
    if matches_any(&lower, &["wood", "plank", "timber"]) {
        return FootstepSurface::Wood;
    }
    if matches_any(&lower, &["stone", "rock", "marble", "cobble", "flagstone"]) {
        return FootstepSurface::Stone;
    }
    if matches_any(&lower, &["water", "shore", "slime"]) {
        return FootstepSurface::Water;
    }
    if lower.contains("mud") {
        return FootstepSurface::Mud;
    }
    if matches_any(&lower, &["carpet", "rug", "fabric"]) {
        return FootstepSurface::Carpet;
    }
    if matches_any(&lower, &["grass", "moss", "leaf", "forest"]) {
        return FootstepSurface::Grass;
    }
    FootstepSurface::Dirt
}

fn matches_any(path: &str, needles: &[&str]) -> bool {
    needles.iter().any(|needle| path.contains(needle))
}

fn is_horse_like(path: &str) -> bool {
    matches_any(path, &["horse", "charger"])
}

fn is_mechanical(path: &str) -> bool {
    matches_any(path, &["mechanical", "mech", "shredder", "golem", "spidertank"])
}

fn is_water_like(path: &str) -> bool {
    matches_any(path, &["fish", "murloc", "water"])
}

fn is_paw_like(path: &str) -> bool {
    matches_any(
        path,
        &["wolf", "worgen", "vulpera", "fox", "cat", "panther", "tiger", "bear"],
    )
}

fn is_hoof_like(path: &str) -> bool {
    matches_any(
        path,
        &["tauren", "draenei", "goat", "stag", "deer", "talbuk", "kodo", "hoof"],
    )
}

fn is_small_humanoid(path: &str) -> bool {
    matches_any(path, &["gnome", "goblin", "mechagnome", "dwarf"])
}

fn is_large_humanoid(path: &str) -> bool {
    matches_any(path, &["orc", "troll", "ogre", "taunka"])
}

fn creature_matches(
    requested: FootstepCreature,
    available: FootstepCreature,
    movement: FootstepMovement,
) -> bool {
    if requested == available {
        return true;
    }
    match movement {
        FootstepMovement::Run if requested == FootstepCreature::HumanoidSmall => {
            available == FootstepCreature::HumanoidMedium
        }
        FootstepMovement::Run if requested == FootstepCreature::HumanoidMedium => {
            available == FootstepCreature::HumanoidLarge
        }
        _ => false,
    }
}

fn creature_penalty(requested: FootstepCreature, available: FootstepCreature) -> u8 {
    if requested == available {
        return 0;
    }
    match (requested, available) {
        (FootstepCreature::HumanoidSmall, FootstepCreature::HumanoidMedium) => 1,
        (FootstepCreature::HumanoidMedium, FootstepCreature::HumanoidLarge) => 1,
        _ => 4,
    }
}

fn surface_penalty(requested: FootstepSurface, available: FootstepSurface) -> u8 {
    if requested == available {
        return 0;
    }
    match (requested, available) {
        (FootstepSurface::Mud, FootstepSurface::Dirt) => 1,
        (FootstepSurface::Ice, FootstepSurface::Snow) => 1,
        (FootstepSurface::Carpet, FootstepSurface::Wood) => 2,
        (FootstepSurface::Grass, FootstepSurface::Dirt) => 2,
        (FootstepSurface::Dirt, FootstepSurface::Grass) => 2,
        _ => 4,
    }
}

fn path_rank_for_movement(path: &str, movement: FootstepMovement) -> u8 {
    let lower = path.to_ascii_lowercase();
    match movement {
        FootstepMovement::Run => {
            if lower.contains("mfootmediumlarge") || lower.contains("mfoothuge") {
                0
            } else {
                1
            }
        }
        _ => {
            if lower.contains("mfootsmall") {
                0
            } else {
                1
            }
        }
    }
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
    counts.get(&(creature, surface)).copied().unwrap_or_default() >= 6
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
    let local = game_engine::asset::casc_resolver::ensure_file_at_path(fdid, &out_path)?;
    std::fs::read(local).ok()
}

fn footstep_output_path(fdid: u32, path: &str) -> PathBuf {
    let ext = Path::new(path)
        .extension()
        .and_then(|ext| ext.to_str())
        .unwrap_or("ogg");
    PathBuf::from("data/sounds/footsteps").join(format!("{fdid}.{ext}"))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_entry(path: &str) -> FootstepCatalogEntry {
        FootstepCatalogEntry::from_path(1, path).expect("sample footstep entry should parse")
    }

    #[test]
    fn footstep_surface_detects_texture_path_keywords() {
        assert_eq!(
            classify_surface_from_texture_path("tileset/elwynn/elwynngrassbase.blp"),
            FootstepSurface::Grass
        );
        assert_eq!(
            classify_surface_from_texture_path("tileset/ironforge/ironforge_metaltrim.blp"),
            FootstepSurface::Metal
        );
        assert_eq!(
            classify_surface_from_texture_path("tileset/winter/snowpacked.blp"),
            FootstepSurface::Snow
        );
        assert_eq!(
            classify_surface_from_texture_path("tileset/deadmines/planks_woodfloor.blp"),
            FootstepSurface::Wood
        );
    }

    #[test]
    fn footstep_creature_class_uses_race_and_model_keywords() {
        assert_eq!(classify_player_creature(1), FootstepCreature::HumanoidMedium);
        assert_eq!(classify_player_creature(7), FootstepCreature::HumanoidSmall);
        assert_eq!(classify_player_creature(6), FootstepCreature::Hoof);
        assert_eq!(
            classify_model_creature("character/vulpera/male/vulperamale.m2"),
            FootstepCreature::Paw
        );
        assert_eq!(
            classify_model_creature("creature/horse/horse.m2"),
            FootstepCreature::Horse
        );
    }

    #[test]
    fn footstep_movement_classifies_anim_ids() {
        assert_eq!(movement_from_anim(4), Some(FootstepMovement::Walk));
        assert_eq!(movement_from_anim(5), Some(FootstepMovement::Run));
        assert_eq!(movement_from_anim(11), Some(FootstepMovement::Strafe));
        assert_eq!(movement_from_anim(13), Some(FootstepMovement::Backpedal));
        assert_eq!(movement_from_anim(0), None);
    }

    #[test]
    fn footstep_catalog_prefers_exact_surface_match_before_fallback() {
        let catalog = FootstepCatalog {
            entries: vec![
                sample_entry("sound/character/footsteps/mfootsmallgrassa.ogg"),
                sample_entry("sound/character/footsteps/mfootsmalldirta.ogg"),
                sample_entry("sound/character/footsteps/mfootsmallstonea.ogg"),
            ],
        };

        let grass = catalog
            .select(FootstepRequest {
                creature: FootstepCreature::HumanoidSmall,
                surface: FootstepSurface::Grass,
                movement: FootstepMovement::Walk,
                seed: 7,
            })
            .expect("grass match");
        let mud = catalog
            .select(FootstepRequest {
                creature: FootstepCreature::HumanoidSmall,
                surface: FootstepSurface::Mud,
                movement: FootstepMovement::Walk,
                seed: 7,
            })
            .expect("fallback match");

        assert!(grass.path.contains("grass"));
        assert!(mud.path.contains("dirt"));
    }

    #[test]
    fn footstep_catalog_picks_heavier_variant_for_run() {
        let catalog = FootstepCatalog {
            entries: vec![
                sample_entry("sound/character/footsteps/mfootsmallgrassa.ogg"),
                sample_entry("sound/character/footsteps/mfootmediumlargegrassa.ogg"),
            ],
        };

        let walk = catalog
            .select(FootstepRequest {
                creature: FootstepCreature::HumanoidSmall,
                surface: FootstepSurface::Grass,
                movement: FootstepMovement::Walk,
                seed: 1,
            })
            .expect("walk match");
        let run = catalog
            .select(FootstepRequest {
                creature: FootstepCreature::HumanoidSmall,
                surface: FootstepSurface::Grass,
                movement: FootstepMovement::Run,
                seed: 1,
            })
            .expect("run match");

        assert!(walk.path.contains("mfootsmall"));
        assert!(run.path.contains("mfootmediumlarge"));
    }
}
