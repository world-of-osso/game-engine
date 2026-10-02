//! Client-side spell catalog built from build-pinned wago.tools DB2 CSV exports.
//!
//! Loaded once on a background task at startup; repeat launches read a compact
//! bincode cache under `data/cache/` that is keyed by build and source CSV
//! size/mtime.

mod build;
mod cache;
pub(crate) mod csv_records;
mod data;
mod render;
mod render_eval;
mod tabs;

#[cfg(test)]
mod real_data_tests;
#[cfg(test)]
mod render_tests;

use std::path::Path;

use bevy::prelude::*;
use bevy::tasks::{AsyncComputeTaskPool, Task, futures::check_ready};

pub use data::*;

impl SpellTextContext {
    pub fn for_player(
        known: Option<&crate::player_spells::KnownSpells>,
        spec: Option<&crate::player_spells::ActiveSpecialization>,
        auras: Vec<u32>,
    ) -> Self {
        Self {
            known_spells: known.map_or_else(Vec::new, |known| known.spells().to_vec()),
            auras,
            spec_id: spec.and_then(|spec| spec.0),
            caster_power: None,
        }
    }
}

/// Local player state for [`SpellTextContext`].
#[derive(bevy::ecs::system::SystemParam)]
pub struct SpellTextSources<'w> {
    known: Option<Res<'w, crate::player_spells::KnownSpells>>,
    auras: Option<Res<'w, crate::buff_data::AuraState>>,
    spec: Option<Res<'w, crate::player_spells::ActiveSpecialization>>,
}

impl SpellTextSources<'_> {
    pub fn is_changed(&self) -> bool {
        self.known.as_ref().is_some_and(|res| res.is_changed())
            || self.auras.as_ref().is_some_and(|res| res.is_changed())
            || self.spec.as_ref().is_some_and(|res| res.is_changed())
    }

    pub fn context(&self) -> SpellTextContext {
        let auras = self.auras.as_ref().map_or_else(Vec::new, |state| {
            state.auras.iter().map(|aura| aura.spell_id).collect()
        });
        SpellTextContext::for_player(self.known.as_deref(), self.spec.as_deref(), auras)
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

    pub fn render_description(&self, id: u32, ctx: &SpellTextContext) -> Option<String> {
        self.data()?.render_description(id, ctx)
    }

    pub fn render_aura_description(&self, id: u32, ctx: &SpellTextContext) -> Option<String> {
        self.data()?.render_aura_description(id, ctx)
    }
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
