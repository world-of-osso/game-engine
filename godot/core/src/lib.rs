//! Bevy-free parsers over authored WoW asset bytes.
pub mod adt;
#[path = "../../../src/area_zone_data.rs"]
pub mod area_zone_data;
pub mod asset;
pub mod asset_loader;
pub mod blp;
#[path = "../../../src/camera_control_data.rs"]
pub mod camera_control_data;
#[path = "../../../src/camera_follow_data.rs"]
pub mod camera_follow_data;
#[path = "../../../src/camera_input_data.rs"]
pub mod camera_input_data;
#[path = "../../../src/campsite_object_data.rs"]
pub mod campsite_object_data;
#[cfg(test)]
mod campsite_object_data_tests;
#[path = "../../../src/sound/catalog_data.rs"]
pub mod catalog_data;
#[path = "../../../src/char_select_camera_data.rs"]
pub mod char_select_camera_data;
#[path = "../../../src/asset/char_texture_data.rs"]
pub mod char_texture_data;
#[path = "../../../src/rendering/character/char_texture_query_data.rs"]
pub mod char_texture_query_data;
#[path = "../../../src/ui/character_creation_icon_mask_data.rs"]
pub mod character_creation_icon_mask_data;
#[path = "../../../src/character_model_data.rs"]
pub mod character_model_data;
#[path = "../../../src/game/equipment/component_file_data.rs"]
mod component_file_data;
#[path = "../../../src/client_options_data.rs"]
pub mod client_options_data;
#[cfg(test)]
mod client_options_data_tests;
#[path = "../../../src/scenes/char_create/background_data.rs"]
pub mod creation_scene_data;
#[path = "../../../src/game/creatures/creature_display_data.rs"]
pub mod creature_display_data;
#[cfg(test)]
mod creature_display_data_tests;
#[path = "../../../src/game/creature_health_scaling_data.rs"]
pub mod creature_health_scaling_data;
#[path = "../../../src/csv_util.rs"]
pub mod csv_util;
#[path = "../../../src/rendering/character/customization_catalog.rs"]
pub mod customization_data;
#[path = "../../../src/rendering/character/customization_query_data.rs"]
mod customization_query_data;
#[path = "../../../src/game/db2_cache.rs"]
pub mod db2_cache;
#[path = "../../../src/sound/footstep_data.rs"]
pub mod footstep_data;
#[cfg(test)]
mod footstep_data_tests;
#[path = "../../../src/geoset_visibility_data.rs"]
pub mod geoset_visibility_data;
#[path = "../../../src/sound/ground_effect_data.rs"]
pub mod ground_effect_data;
#[cfg(test)]
mod ground_effect_data_tests;
#[path = "../../../src/input_bindings_data.rs"]
pub mod input_bindings_data;
#[path = "../../../src/rendering/lighting/light_lookup_data.rs"]
pub mod light_lookup_data;
pub mod lighting_assets;
pub mod liquid_data;
#[path = "../../../src/game/state/loading_readiness.rs"]
pub mod loading_readiness;
pub mod m2;
pub mod m2_billboard;
pub mod m2_lights;
pub mod m2_material;
pub mod m2_particles;
#[path = "../../../src/asset/m2_texture_composite_data.rs"]
pub mod m2_texture_composite_data;
pub mod minimap_data;
#[path = "../../../src/movement_animation_data.rs"]
pub mod movement_animation_data;
#[path = "../../../src/movement_input_data.rs"]
pub mod movement_input_data;
#[path = "../../../src/game/nameplate_style_data.rs"]
pub mod nameplate_style_data;
#[path = "../../../src/rendering/ui/nameplate_visibility_data.rs"]
pub mod nameplate_visibility_data;
#[cfg(test)]
mod nameplate_visibility_data_tests;
#[path = "../../../src/quest_area_data.rs"]
pub mod quest_area_data;
#[path = "../../../src/realm_preset_data.rs"]
pub mod realm_preset_data;
#[path = "../../../src/scenes/scene_snapshot_data.rs"]
pub mod scene_snapshot;
pub mod spell_visual;
#[path = "../../../src/sound/ui_click_data.rs"]
pub mod ui_click_data;
pub mod ui_layout_data;
#[path = "../../../src/sound/wmo_surface_data.rs"]
pub mod wmo_surface_data;
#[cfg(test)]
mod wmo_surface_data_tests;
pub use asset::m2_batch_data;
#[path = "../../../src/cache_source_mtime.rs"]
mod cache_source_mtime;
#[path = "../../../src/cache_sqlite.rs"]
mod cache_sqlite;
#[path = "../../../src/game/equipment/helmet_geoset_data.rs"]
mod helmet_geoset_data;
#[cfg(test)]
mod helmet_geoset_data_tests;
#[path = "../../../src/little_endian.rs"]
mod little_endian;
#[cfg(test)]
mod m2_batch_tests;
pub mod npc_appearance_assets;
#[path = "../../../src/game/creatures/npc_appearance_data.rs"]
pub mod npc_appearance_data;
#[cfg(test)]
mod npc_appearance_data_tests;
#[path = "../../../src/rendering/character/npc_appearance_selection_data.rs"]
pub mod npc_appearance_selection_data;
#[path = "../../../src/game/creatures/npc_visibility_data.rs"]
pub mod npc_visibility_data;
#[cfg(test)]
mod npc_visibility_data_tests;
#[path = "../../../src/game/outfit_catalog_db.rs"]
pub mod outfit_catalog_db;
#[path = "../../../src/game/equipment/outfit_catalog.rs"]
pub mod outfit_data;
#[cfg(test)]
mod outfit_data_tests;
#[path = "../../../src/game/outfit_listfile.rs"]
mod outfit_listfile;
pub mod player_model_data;
#[path = "../../../src/player_physics_data.rs"]
pub mod player_physics_data;
pub mod retail_fog;
#[path = "../../../src/rendering/lighting/retail_light_data.rs"]
pub mod retail_light_data;
#[path = "../../../src/screen_arg_data.rs"]
pub mod screen_arg_data;
#[path = "../../../src/rendering/skybox/sky_cubemap_data.rs"]
pub mod sky_cubemap_data;
#[path = "../../../src/rendering/skybox/sky_lightdata_data.rs"]
pub mod sky_lightdata_data;
pub mod spell_catalog;
#[path = "../../../src/ui/spellbook_data.rs"]
pub mod spellbook_data;
#[path = "../../../src/sqlite_util.rs"]
mod sqlite_util;
#[path = "../../../src/startup_args_data.rs"]
pub mod startup_args_data;
#[cfg(test)]
mod startup_args_data_tests;
#[path = "../../../src/rendering/ui/target_selection_data.rs"]
pub mod target_selection_data;
#[cfg(test)]
mod target_selection_data_tests;
#[path = "../../../src/rendering/terrain/terrain_height_data.rs"]
pub mod terrain_height_data;
#[path = "../../../src/rendering/terrain/terrain_material_data.rs"]
pub mod terrain_material_data;
#[path = "../../../src/rendering/terrain/terrain_surface_data.rs"]
pub mod terrain_surface_data;
pub use footstep_data as sound_footsteps;
#[path = "../../../src/unit_motion_data.rs"]
pub mod unit_motion_data;
#[path = "../../../src/warband_scene_data.rs"]
pub mod warband_scene_data;
pub mod wdt;
pub mod wmo;
pub mod wmo_liquid;
#[path = "../../../src/rendering/terrain/terrain_objects_wmo_material.rs"]
pub mod wmo_material_data;

#[cfg(test)]
mod terrain_surface_data_tests {
    use crate::adt::{AdtTexData, ChunkTexLayers, TextureLayer};
    use crate::asset::adt_format::adt_tex::{MclyFlags, MphdFlags};
    use crate::footstep_data::FootstepSurface;
    use crate::terrain_surface_data::{
        dominant_effect_id, dominant_surface_for_chunk_with_resolver, dominant_texture_fdid,
    };

    fn layer(texture_index: u32, effect_id: u32, alpha_map: Option<Vec<u8>>) -> TextureLayer {
        TextureLayer {
            texture_index,
            flags: MclyFlags::default(),
            effect_id,
            material_id: 0,
            alpha_map,
        }
    }

    fn tex(fdids: Vec<u32>) -> AdtTexData {
        AdtTexData {
            map_flags: MphdFlags::default(),
            texture_amplifier: None,
            texture_fdids: fdids,
            height_texture_fdids: Vec::new(),
            texture_flags: Vec::new(),
            texture_params: Vec::new(),
            chunk_layers: Vec::new(),
        }
    }

    #[test]
    fn effects_skip_zero_and_use_base_weight_and_later_ties() {
        let chunk = ChunkTexLayers {
            layers: vec![
                layer(0, 5, None),
                layer(1, 0, Some(vec![255; 4096])),
                layer(2, 9, Some(vec![250; 4000])),
                layer(3, 11, Some(vec![250; 4000])),
            ],
        };
        assert_eq!(dominant_effect_id(&chunk), Some(11));
        assert_eq!(
            dominant_effect_id(&ChunkTexLayers {
                layers: vec![layer(0, 0, None)]
            }),
            None
        );
        assert_eq!(
            dominant_effect_id(&ChunkTexLayers {
                layers: vec![layer(0, 7, None), layer(1, 8, Some(vec![255; 100]))]
            }),
            Some(7)
        );
    }

    #[test]
    fn texture_alpha_sum_ties_and_invalid_index_stop_selection() {
        let tex = tex(vec![10, 20, 30]);
        let chunk = ChunkTexLayers {
            layers: vec![
                layer(0, 1, None),
                layer(1, 2, Some(vec![250; 4000])),
                layer(2, 3, Some(vec![250; 4000])),
            ],
        };
        assert_eq!(dominant_texture_fdid(&tex, &chunk), Some(30));
        let base = ChunkTexLayers {
            layers: vec![layer(0, 1, None), layer(1, 2, None)],
        };
        assert_eq!(dominant_texture_fdid(&tex, &base), Some(10));
        let invalid = ChunkTexLayers {
            layers: vec![
                layer(0, 1, None),
                layer(9, 2, None),
                layer(2, 3, Some(vec![255; 4096])),
            ],
        };
        assert_eq!(dominant_texture_fdid(&tex, &invalid), None);
        assert_eq!(
            dominant_texture_fdid(&tex, &ChunkTexLayers { layers: vec![] }),
            None
        );
    }

    #[test]
    fn resolved_effect_precedes_texture_and_unresolved_uses_path_or_dirt() {
        let tex = tex(vec![123]);
        let chunk = ChunkTexLayers {
            layers: vec![layer(0, 42, None)],
        };
        let surface = |effect| (effect == 42).then_some(FootstepSurface::Stone);
        let path = |fdid| (fdid == 123).then_some("world/terrain/grass.blp");
        assert_eq!(
            dominant_surface_for_chunk_with_resolver(&tex, &chunk, surface, path),
            FootstepSurface::Stone
        );
        assert_eq!(
            dominant_surface_for_chunk_with_resolver(&tex, &chunk, |_| None, path),
            FootstepSurface::Grass
        );
        assert_eq!(
            dominant_surface_for_chunk_with_resolver(&tex, &chunk, |_| None, |_| None),
            FootstepSurface::Dirt
        );
        assert_eq!(
            dominant_surface_for_chunk_with_resolver(
                &tex,
                &ChunkTexLayers {
                    layers: vec![layer(9, 42, None)]
                },
                |_| None,
                path
            ),
            FootstepSurface::Dirt
        );
    }
}

#[cfg(test)]
mod terrain_height_data_tests {
    use crate::adt::{ChunkHeightGrid, UNIT_SIZE};
    use crate::terrain_height_data::sample_chunk_height;

    fn authored_grid() -> ChunkHeightGrid {
        let mut heights = [0.0; 145];
        for row in 0..=8 {
            for col in 0..=8 {
                heights[row * 17 + col] = 10.0 * row as f32 + col as f32;
            }
            if row < 8 {
                for col in 0..8 {
                    heights[row * 17 + 9 + col] =
                        10.0 * (row as f32 + 0.5) + col as f32 + 0.5 + 3.0;
                }
            }
        }
        ChunkHeightGrid {
            index_x: 0,
            index_y: 0,
            origin_x: 100.0,
            origin_z: 200.0,
            base_y: 50.0,
            heights,
        }
    }

    fn sample(grid: &ChunkHeightGrid, row: f32, col: f32) -> Option<f32> {
        sample_chunk_height(grid, 100.0 - row * UNIT_SIZE, 200.0 + col * UNIT_SIZE)
    }

    fn assert_height(grid: &ChunkHeightGrid, row: f32, col: f32, expected: f32) {
        let actual = sample(grid, row, col).expect("point must lie inside chunk");
        assert!((actual - expected).abs() < 0.001, "{actual} != {expected}");
    }

    #[test]
    fn authored_axes_base_height_and_center_are_preserved() {
        let grid = authored_grid();
        assert_height(&grid, 2.0, 5.0, 75.0);
        assert_height(&grid, 2.5, 5.5, 83.5);
        assert_height(&grid, 0.0, 0.0, 50.0);
    }

    #[test]
    fn nonplanar_center_fan_interpolates_each_triangle() {
        let grid = authored_grid();
        for (row, col, expected) in [
            (0.25, 0.5, 54.5),
            (0.5, 0.75, 57.25),
            (0.75, 0.5, 59.5),
            (0.5, 0.25, 56.75),
        ] {
            assert_height(&grid, row, col, expected);
        }
    }

    #[test]
    fn chunk_bounds_are_half_open_in_authored_axes() {
        let grid = authored_grid();
        assert_height(&grid, 0.0, 0.0, 50.0);
        assert_eq!(sample(&grid, 8.01, 0.0), None);
        assert_eq!(sample(&grid, 0.0, 8.01), None);
        assert_eq!(sample(&grid, -0.25, 0.0), None);
        assert_eq!(sample(&grid, 0.0, -0.25), None);
        assert!(sample(&grid, 7.5, 7.5).is_some());
    }
}
