//! Bevy-free parsers over authored WoW asset bytes.
pub mod adt;
pub mod asset;
pub mod blp;
#[path = "../../../src/game/state/loading_readiness.rs"]
pub mod loading_readiness;
pub mod m2;
#[path = "../../../src/rendering/terrain/terrain_material_data.rs"]
pub mod terrain_material_data;
pub mod wdt;
pub mod wmo;
