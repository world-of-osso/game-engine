//! Shared spell catalog (`src/game/spell_catalog`) without the Bevy background loader.

#[path = "../../../src/game/spell_catalog/build.rs"]
mod build;
#[path = "../../../src/game/spell_catalog/cache.rs"]
mod cache;
#[path = "../../../src/game/spell_catalog/csv_records.rs"]
pub(crate) mod csv_records;
#[path = "../../../src/game/spell_catalog/data.rs"]
mod data;
#[path = "../../../src/game/spell_catalog/render.rs"]
mod render;
#[path = "../../../src/game/spell_catalog/render_eval.rs"]
mod render_eval;
#[path = "../../../src/game/spell_catalog/tabs.rs"]
mod tabs;

pub use data::*;
