//! Client-side spell catalog built from build-pinned wago.tools DB2 CSV exports.
//!
//! Loaded once on a background task at startup; repeat launches read a compact
//! bincode cache under `data/cache/` that is keyed by build and source CSV
//! size/mtime.

mod build;
mod cache;
pub(crate) mod csv_records;
mod render;
mod tabs;

#[cfg(test)]
mod real_data_tests;
#[cfg(test)]
mod render_tests;

use std::path::{Path, PathBuf};

use bevy::prelude::*;
use bevy::tasks::{AsyncComputeTaskPool, Task, futures::check_ready};
use serde::{Deserialize, Serialize};

pub use tabs::{SpecTabInfo, SpellbookTabIndex, SpellbookTabKind};

pub const SPELL_DB2_BUILD: &str = "12.1.0.69933";

/// Range in yards. Index 0 is the hostile range, index 1 the friendly range.
#[derive(Clone, Copy, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct SpellRange {
    pub min_yd: [f32; 2],
    pub max_yd: [f32; 2],
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct SpellCooldown {
    pub recovery_ms: u32,
    pub category_recovery_ms: u32,
    /// Global cooldown triggered by the cast (`StartRecoveryTime`).
    pub gcd_ms: u32,
}

#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub struct SpellCharges {
    pub max_charges: u32,
    pub recovery_ms: u32,
}

#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub struct SpellPowerCost {
    /// Raw `SpellPower.PowerType` DB value (0 mana, 1 rage, 3 energy, ...).
    pub power_type: i8,
    pub flat: i32,
    pub pct: f32,
    /// Cost row only applies while this aura is active (0 = always).
    pub required_aura_spell_id: u32,
}

#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub struct CatalogEffect {
    pub index: u8,
    pub effect: u16,
    pub aura: u16,
    pub base_points: f32,
    pub aura_period_ms: u32,
    /// Raw DB value; a few rows hold negative counts.
    pub chain_targets: i32,
    pub radius_yd: f32,
}

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct CatalogSpell {
    pub id: u32,
    pub name: Box<str>,
    pub subtext: Box<str>,
    /// Raw `Spell.Description_lang`; see [`SpellCatalog::render_description`].
    pub description: Box<str>,
    pub aura_description: Box<str>,
    pub icon_fdid: u32,
    /// Raw DB value; one row holds -1.
    pub active_icon_fdid: i32,
    pub school_mask: u32,
    /// `SpellMisc.Attributes_0 & 0x40`.
    pub passive: bool,
    pub cast_time_ms: i32,
    pub range: SpellRange,
    /// `SpellDuration.Duration`; 0 = no duration, negative = until cancelled.
    pub duration_ms: i32,
    pub max_duration_ms: i32,
    pub cooldown: SpellCooldown,
    pub charges: Option<SpellCharges>,
    pub max_stacks: u32,
    /// Raw DB value; -1 on a few rows.
    pub proc_charges: i32,
    pub proc_chance: u32,
    pub powers: Box<[SpellPowerCost]>,
    pub effects: Box<[CatalogEffect]>,
}

impl CatalogSpell {
    pub fn effect(&self, index: u8) -> Option<&CatalogEffect> {
        self.effects.iter().find(|effect| effect.index == index)
    }
}

/// Spells sorted by id.
#[derive(Debug, Default)]
pub struct SpellCatalogData {
    spells: Vec<CatalogSpell>,
    pub tabs: SpellbookTabIndex,
}

impl SpellCatalogData {
    fn from_sorted(mut spells: Vec<CatalogSpell>, tabs: SpellbookTabIndex) -> Self {
        // Deserialized Vecs grow by doubling; ~50 MB of slack otherwise.
        spells.shrink_to_fit();
        Self { spells, tabs }
    }

    /// Catalog from unsorted rows, for fixtures.
    pub fn from_parts(mut spells: Vec<CatalogSpell>, tabs: SpellbookTabIndex) -> Self {
        spells.sort_unstable_by_key(|spell| spell.id);
        Self::from_sorted(spells, tabs)
    }

    pub fn len(&self) -> usize {
        self.spells.len()
    }

    pub fn is_empty(&self) -> bool {
        self.spells.is_empty()
    }

    pub fn get(&self, id: u32) -> Option<&CatalogSpell> {
        let index = self
            .spells
            .binary_search_by_key(&id, |spell| spell.id)
            .ok()?;
        Some(&self.spells[index])
    }

    pub fn render_description(&self, id: u32) -> Option<String> {
        let spell = self.get(id)?;
        Some(render::render_spell_text(&spell.description, spell, self))
    }

    pub fn render_aura_description(&self, id: u32) -> Option<String> {
        let spell = self.get(id)?;
        Some(render::render_spell_text(
            &spell.aura_description,
            spell,
            self,
        ))
    }
}

#[derive(Debug, Default)]
pub enum SpellCatalogState {
    #[default]
    Loading,
    Ready(SpellCatalogData),
    Failed(String),
}

#[derive(Resource, Debug, Default)]
pub struct SpellCatalog {
    pub state: SpellCatalogState,
}

impl SpellCatalog {
    pub fn ready(data: SpellCatalogData) -> Self {
        Self {
            state: SpellCatalogState::Ready(data),
        }
    }

    pub fn is_ready(&self) -> bool {
        matches!(self.state, SpellCatalogState::Ready(_))
    }

    pub fn data(&self) -> Option<&SpellCatalogData> {
        match &self.state {
            SpellCatalogState::Ready(data) => Some(data),
            _ => None,
        }
    }

    pub fn get(&self, id: u32) -> Option<&CatalogSpell> {
        self.data()?.get(id)
    }

    pub fn render_description(&self, id: u32) -> Option<String> {
        self.data()?.render_description(id)
    }

    pub fn render_aura_description(&self, id: u32) -> Option<String> {
        self.data()?.render_aura_description(id)
    }
}

pub struct SpellCatalogPaths {
    pub source_dir: PathBuf,
    pub cache_path: PathBuf,
}

impl SpellCatalogPaths {
    pub fn for_data_dir(data_dir: &Path) -> Self {
        Self {
            source_dir: data_dir.join("db2").join(SPELL_DB2_BUILD),
            cache_path: data_dir
                .join("cache")
                .join(format!("spell_catalog-{SPELL_DB2_BUILD}.bin")),
        }
    }
}

/// Loads from the cache when fresh, otherwise rebuilds from CSV and rewrites the cache.
/// The spellbook tab index is small and always rebuilt from CSV.
pub fn load_spell_catalog(paths: &SpellCatalogPaths) -> Result<SpellCatalogData, String> {
    let tabs = tabs::load_tab_index(&paths.source_dir)?;
    let key = cache::cache_key(&paths.source_dir)?;
    let spells = crate::db2_cache::load_or_build(&paths.cache_path, &key, || {
        build::build_spells(&paths.source_dir)
    })?;
    Ok(SpellCatalogData::from_sorted(spells, tabs))
}

#[derive(Resource)]
struct SpellCatalogLoadTask(Task<Result<SpellCatalogData, String>>);

pub struct SpellCatalogPlugin;

impl Plugin for SpellCatalogPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<SpellCatalog>()
            .add_systems(Startup, spawn_spell_catalog_load)
            .add_systems(
                Update,
                finish_spell_catalog_load.run_if(resource_exists::<SpellCatalogLoadTask>),
            );
    }
}

fn spawn_spell_catalog_load(mut commands: Commands) {
    let paths = SpellCatalogPaths::for_data_dir(Path::new("data"));
    let task = AsyncComputeTaskPool::get().spawn(async move {
        let started = std::time::Instant::now();
        let result = load_spell_catalog(&paths);
        if let Ok(data) = &result {
            info!(
                "Spell catalog loaded {} spells in {:?}",
                data.len(),
                started.elapsed()
            );
        }
        result
    });
    commands.insert_resource(SpellCatalogLoadTask(task));
}

fn finish_spell_catalog_load(
    mut commands: Commands,
    mut task: ResMut<SpellCatalogLoadTask>,
    mut catalog: ResMut<SpellCatalog>,
) {
    let Some(result) = check_ready(&mut task.0) else {
        return;
    };
    commands.remove_resource::<SpellCatalogLoadTask>();
    catalog.state = match result {
        Ok(data) => SpellCatalogState::Ready(data),
        Err(err) => {
            error!("Spell catalog load failed: {err}");
            SpellCatalogState::Failed(err)
        }
    };
}
