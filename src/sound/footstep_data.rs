use std::path::Path;

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

/// Observes the current movement clip at one instant; missed halves are not replayed.
#[derive(Debug, Default)]
pub struct FootstepPhaseTracker {
    last_half: u8,
    last_seq_idx: usize,
}

impl FootstepPhaseTracker {
    pub fn observe(
        &mut self,
        seq_idx: usize,
        anim_id: u16,
        duration: f32,
        time_ms: f32,
    ) -> Option<(FootstepMovement, u64)> {
        let Some(movement) = movement_from_anim(anim_id) else {
            self.last_seq_idx = seq_idx;
            return None;
        };
        if seq_idx != self.last_seq_idx {
            self.last_half = 0;
            self.last_seq_idx = seq_idx;
        }
        if duration <= 0.0 {
            return None;
        }
        let progress = (time_ms % duration) / duration;
        let current_half = if progress < 0.5 { 0 } else { 1 };
        if current_half == self.last_half {
            return None;
        }
        self.last_half = current_half;
        let seed = (seq_idx as u64) << 8 | u64::from(current_half);
        Some((movement, seed))
    }
}

#[derive(Debug, Default)]
pub struct FootstepCatalog {
    pub entries: Vec<FootstepCatalogEntry>,
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
    if path.contains("spider")
        || path.contains("rat")
        || path.contains("frog")
        || path.contains("crab")
    {
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
    matches_any(
        path,
        &["mechanical", "mech", "shredder", "golem", "spidertank"],
    )
}

fn is_water_like(path: &str) -> bool {
    matches_any(path, &["fish", "murloc", "water"])
}

fn is_paw_like(path: &str) -> bool {
    matches_any(
        path,
        &[
            "wolf", "worgen", "vulpera", "fox", "cat", "panther", "tiger", "bear",
        ],
    )
}

fn is_hoof_like(path: &str) -> bool {
    matches_any(
        path,
        &[
            "tauren", "draenei", "goat", "stag", "deer", "talbuk", "kodo", "hoof",
        ],
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
