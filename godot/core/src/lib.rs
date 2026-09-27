//! Bevy-free parsers over authored WoW asset bytes.
pub mod adt;
pub mod asset;
pub mod blp;
#[path = "../../../src/camera_control_data.rs"]
pub mod camera_control_data;
#[path = "../../../src/camera_follow_data.rs"]
pub mod camera_follow_data;
#[path = "../../../src/camera_input_data.rs"]
pub mod camera_input_data;
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
#[path = "../../../src/client_options_data.rs"]
pub mod client_options_data;
#[cfg(test)]
mod client_options_data_tests;
#[path = "../../../src/game/creatures/creature_display_data.rs"]
pub mod creature_display_data;
#[cfg(test)]
mod creature_display_data_tests;
#[path = "../../../src/csv_util.rs"]
pub mod csv_util;
#[path = "../../../src/rendering/character/customization_catalog.rs"]
pub mod customization_data;
#[path = "../../../src/rendering/character/customization_query_data.rs"]
mod customization_query_data;
#[path = "../../../src/geoset_visibility_data.rs"]
pub mod geoset_visibility_data;
#[path = "../../../src/input_bindings_data.rs"]
pub mod input_bindings_data;
#[path = "../../../src/rendering/lighting/light_lookup_data.rs"]
pub mod light_lookup_data;
pub mod lighting_assets;
#[path = "../../../src/game/state/loading_readiness.rs"]
pub mod loading_readiness;
pub mod m2;
#[path = "../../../src/asset/m2_effect_uv_data.rs"]
pub mod m2_effect_uv_data;
#[path = "../../../src/asset/m2_texture_composite_data.rs"]
pub mod m2_texture_composite_data;
#[path = "../../../src/movement_input_data.rs"]
pub mod movement_input_data;
#[path = "../../../src/game/nameplate_style_data.rs"]
pub mod nameplate_style_data;
#[path = "../../../src/realm_preset_data.rs"]
pub mod realm_preset_data;
pub use asset::m2_batch_data;
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
#[path = "../../../src/player_physics_data.rs"]
pub mod player_physics_data;
#[path = "../../../src/rendering/lighting/retail_light_data.rs"]
pub mod retail_light_data;
#[path = "../../../src/rendering/skybox/sky_cubemap_data.rs"]
pub mod sky_cubemap_data;
#[path = "../../../src/rendering/skybox/sky_lightdata_data.rs"]
pub mod sky_lightdata_data;
#[path = "../../../src/rendering/terrain/terrain_height_data.rs"]
pub mod terrain_height_data;
#[path = "../../../src/rendering/terrain/terrain_material_data.rs"]
pub mod terrain_material_data;
#[path = "../../../src/unit_motion_data.rs"]
pub mod unit_motion_data;
#[path = "../../../src/warband_scene_data.rs"]
pub mod warband_scene_data;
pub mod wdt;
pub mod wmo;
#[path = "../../../src/rendering/terrain/terrain_objects_wmo_material.rs"]
pub mod wmo_material_data;

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
