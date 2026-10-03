//! Shared spell catalog (`src/game/spell_catalog`) without the Bevy background loader.

#[path = "game/spell_catalog/build.rs"]
mod build;
#[path = "game/spell_catalog/cache.rs"]
mod cache;
#[path = "game/spell_catalog/csv_records.rs"]
pub(crate) mod csv_records;
#[path = "game/spell_catalog/data.rs"]
mod data;
#[path = "game/spell_catalog/render.rs"]
mod render;
#[path = "game/spell_catalog/render_eval.rs"]
mod render_eval;
#[path = "game/spell_catalog/tabs.rs"]
mod tabs;

pub use data::*;
