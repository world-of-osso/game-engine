#[path = "camera/camera.rs"]
pub mod camera;
#[path = "camera/orbit_camera.rs"]
pub mod orbit_camera;

#[path = "character/character_customization.rs"]
pub mod character_customization;
#[path = "character/character_models.rs"]
pub mod character_models;
#[path = "character/m2_texture_composite.rs"]
pub mod m2_texture_composite;

#[path = "lighting/light_lookup.rs"]
pub mod light_lookup;

#[path = "model/animation.rs"]
pub mod animation;
#[path = "model/m2_effect_material.rs"]
pub mod m2_effect_material;
#[path = "model/m2_scene/mod.rs"]
pub mod m2_scene;
#[path = "model/m2_spawn.rs"]
pub mod m2_spawn;

#[path = "particles/mod.rs"]
pub mod particle;

#[path = "skybox/mod.rs"]
pub mod sky;
#[path = "skybox/sky_lightdata.rs"]
pub mod sky_lightdata;
#[path = "skybox/sky_material.rs"]
pub mod sky_material;
#[path = "skybox/skybox_m2_material.rs"]
pub mod skybox_m2_material;

#[path = "terrain/ground.rs"]
pub mod ground;
#[path = "terrain/terrain.rs"]
pub mod terrain;
#[path = "terrain/terrain_heightmap.rs"]
pub mod terrain_heightmap;
#[path = "terrain/terrain_load_limits.rs"]
pub mod terrain_load_limits;
#[path = "terrain/terrain_load_progress.rs"]
pub mod terrain_load_progress;
#[path = "terrain/terrain_lod.rs"]
pub mod terrain_lod;
#[path = "terrain/terrain_material.rs"]
pub mod terrain_material;
#[path = "terrain/terrain_memory_debug.rs"]
pub mod terrain_memory_debug;
#[path = "terrain/terrain_objects.rs"]
pub mod terrain_objects;
#[path = "terrain/terrain_tile.rs"]
pub mod terrain_tile;
#[path = "terrain/water_material.rs"]
pub mod water_material;

#[path = "ui/action_bar.rs"]
pub mod action_bar;
#[path = "ui/health_bar.rs"]
pub mod health_bar;
#[path = "ui/minimap.rs"]
pub mod minimap;
#[path = "ui/minimap_render.rs"]
pub mod minimap_render;
#[path = "ui/nameplate.rs"]
pub mod nameplate;
#[path = "ui/target.rs"]
pub mod target;
#[path = "ui/targeting.rs"]
pub mod targeting;
#[path = "ui/unit_frames.rs"]
pub mod unit_frames;
#[path = "ui/wow_cursor.rs"]
pub mod wow_cursor;
