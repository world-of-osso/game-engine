//! Bevy-free parsers over authored WoW asset bytes.
pub mod adt;
pub mod asset;
pub mod blp;
#[path = "../../../src/rendering/lighting/light_lookup_data.rs"]
pub mod light_lookup_data;
#[path = "../../../src/game/state/loading_readiness.rs"]
pub mod loading_readiness;
pub mod m2;
#[path = "../../../src/rendering/lighting/retail_light_data.rs"]
pub mod retail_light_data;
#[path = "../../../src/rendering/skybox/sky_lightdata_data.rs"]
pub mod sky_lightdata_data;
#[path = "../../../src/rendering/terrain/terrain_material_data.rs"]
pub mod terrain_material_data;
pub mod wdt;
pub mod wmo;
