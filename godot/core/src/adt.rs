//! ADT root, texture companion and object companion byte parsers.
pub use crate::asset::adt_format::adt_water_geometry::{WaterGeometry, build_water_geometry};
use crate::asset::adt_format::{adt, adt_geometry, adt_obj, adt_tex};

pub use crate::terrain_material_data::{
    TerrainBlendMode, pack_alpha_map_bytes, terrain_blend_mode, terrain_layer_animation,
    terrain_layer_animation_params, terrain_texture_repeat, texture_layer_params,
};
pub use adt::{
    BlendBatch, BlendMeshData, ChunkHeightGrid, FlightBounds, ParsedLodData, SoundEmitter,
    UNIT_SIZE,
};
pub use adt_geometry::Geometry;
pub use adt_obj::{AdtObjData, ChunkObjectRefs, DoodadPlacement, WmoPlacement};
pub use adt_tex::{
    AdtTexData, AdtWaterData, ChunkTexLayers, TextureLayer, TextureParams, WaterLayer,
};

pub struct Root {
    pub chunks: Vec<Chunk>,
    pub height_grids: Vec<ChunkHeightGrid>,
    pub center_surface: [f32; 3],
    pub chunk_positions: Vec<[f32; 3]>,
    pub blend_mesh: Option<BlendMeshData>,
    pub flight_bounds: Option<FlightBounds>,
    pub water: Option<AdtWaterData>,
    pub water_error: Option<String>,
}

pub struct Chunk {
    pub index_x: u32,
    pub index_y: u32,
    pub position: [f32; 3],
    pub area_id: u32,
    pub do_not_fix_alpha_map: bool,
    pub heights: [f32; 145],
    pub normals: [[f32; 3]; 145],
    pub vertex_colors: [[f32; 4]; 145],
    pub vertex_lighting: Option<[[f32; 4]; 145]>,
    pub sound_emitters: Vec<SoundEmitter>,
    pub blend_batches: Vec<BlendBatch>,
    pub detail_doodad_disable: Option<[u8; 64]>,
    /// MCNK header 0x40: per cell row, eight 2-bit MCLY indices.
    pub texture_selection: [u16; 8],
    /// MCNK header 0x50: per cell row, eight detail-doodad disable bits.
    pub detail_exclusion: [u8; 8],
    /// MCCV is authored (otherwise `vertex_colors` is white).
    pub has_vertex_colors: bool,
    pub holes_low_res: u16,
    pub holes_high_res: Option<u64>,
    pub shadow_map: Option<[u8; 512]>,
}

/// MCNK positions, normals, UVs, and original Bevy winding for a parsed chunk.
/// `tile_coords` are `(tile_y, tile_x)` when the authored tile is known.
pub fn chunk_geometry(chunk: &Chunk, tile_coords: Option<(u32, u32)>) -> Geometry {
    adt_geometry::build_mcnk_geometry(
        adt_geometry::GeometryChunk {
            index_x: chunk.index_x,
            index_y: chunk.index_y,
            position: chunk.position,
            heights: &chunk.heights,
            normals: &chunk.normals,
            holes_low_res: chunk.holes_low_res,
            holes_high_res: chunk.holes_high_res,
        },
        tile_coords,
    )
}

pub fn parse_root(data: &[u8]) -> Result<Root, String> {
    let parsed = adt::load_adt_parsed(data)?;
    Ok(root_from_parsed(parsed))
}

/// Parse a root tile with authored `(tile_y, tile_x)` and optional texture-companion shadows.
pub fn parse_root_for_tile(
    data: &[u8],
    tile_y: u32,
    tile_x: u32,
    texture_data: Option<&[u8]>,
) -> Result<Root, String> {
    let parsed = adt::load_adt_for_tile_parsed(data, tile_y, tile_x, texture_data)?;
    Ok(root_from_parsed(parsed))
}

pub fn parse_lod(data: &[u8]) -> Result<ParsedLodData, String> {
    adt::load_lod_adt(data)
}

fn root_from_parsed(parsed: adt::ParsedAdtData) -> Root {
    Root {
        chunks: parsed
            .chunks
            .into_iter()
            .map(|chunk| Chunk {
                index_x: chunk.index_x,
                index_y: chunk.index_y,
                position: chunk.pos,
                area_id: chunk.area_id,
                do_not_fix_alpha_map: chunk.flags.do_not_fix_alpha_map,
                heights: chunk.heights,
                normals: chunk.normals,
                vertex_colors: chunk.vertex_colors,
                vertex_lighting: chunk.vertex_lighting,
                sound_emitters: chunk.sound_emitters,
                blend_batches: chunk.blend_batches,
                detail_doodad_disable: chunk.detail_doodad_disable,
                texture_selection: chunk.texture_selection,
                detail_exclusion: chunk.detail_exclusion,
                has_vertex_colors: chunk.flags.has_mccv,
                holes_low_res: chunk.holes_low_res,
                holes_high_res: chunk.holes_high_res,
                shadow_map: chunk.shadow_map,
            })
            .collect(),
        height_grids: parsed.height_grids,
        center_surface: parsed.center_surface,
        chunk_positions: parsed.chunk_positions,
        blend_mesh: parsed.blend_mesh,
        flight_bounds: parsed.flight_bounds,
        water: parsed.water,
        water_error: parsed.water_error,
    }
}

/// Alpha-map interpretation requires each root chunk's authored `do_not_fix_alpha_map` bit.
pub fn parse_tex(
    data: &[u8],
    map_flags: crate::wdt::MphdFlags,
    root: &Root,
) -> Result<AdtTexData, String> {
    let flags: Vec<_> = root.chunks.iter().map(|c| c.do_not_fix_alpha_map).collect();
    adt_tex::load_adt_tex0(data, map_flags, &flags)
}

pub fn parse_obj(data: &[u8]) -> Result<AdtObjData, String> {
    adt_obj::load_adt_obj0(data)
}

#[cfg(test)]
mod material_data_tests {
    use super::*;

    fn layer(
        texture_index: u32,
        flags: u32,
        material_id: u8,
        alpha_map: Option<Vec<u8>>,
    ) -> TextureLayer {
        TextureLayer {
            texture_index,
            flags: adt_tex::MclyFlags { raw: flags },
            effect_id: 0,
            material_id,
            alpha_map,
        }
    }

    #[test]
    fn blend_modes_follow_map_flags_including_height_without_big_alpha_bit() {
        for (raw, expected) in [
            (0, TerrainBlendMode::Layered),
            (0x4, TerrainBlendMode::Weighted),
            (0x80, TerrainBlendMode::HeightWeighted),
            (0x84, TerrainBlendMode::HeightWeighted),
        ] {
            assert_eq!(terrain_blend_mode(crate::wdt::MphdFlags { raw }), expected);
        }
    }

    #[test]
    fn texture_repeat_uses_amplifier_and_caps_exponent() {
        assert_eq!(terrain_texture_repeat(None), 8.0);
        assert_eq!(terrain_texture_repeat(Some(2)), 32.0);
        assert_eq!(terrain_texture_repeat(Some(8)), 2048.0);
        assert_eq!(terrain_texture_repeat(Some(100)), 2048.0);
    }

    #[test]
    fn height_and_overbright_params_use_texture_index_and_mhid_presence() {
        let params = [
            TextureParams {
                flags: 0,
                height_scale: 1.25,
                height_offset: -0.5,
            },
            TextureParams {
                flags: 0,
                height_scale: 0.75,
                height_offset: 0.125,
            },
        ];
        let layers = [layer(1, 0x80, 9, None), layer(0, 0, 4, None)];
        let result = texture_layer_params(&params, &layers, [true, false, false, false]);
        assert_eq!(result[0], [0.75, 0.125, 9.0, 2.0]);
        assert_eq!(result[1], [0.0, -0.5, 4.0, 1.0]);
        assert_eq!(result[2], [0.0, 1.0, 0.0, 1.0]);
        assert_eq!(
            texture_layer_params(&[], &layers, [true; 4])[0],
            [0.0, 1.0, 9.0, 2.0]
        );
    }

    #[test]
    fn uv_velocity_and_reflection_follow_layer_flags() {
        let layers = [
            layer(0, 0x40 | 0x19 | 0x400, 0, None),
            layer(1, 0x400, 0, None),
        ];
        let params = terrain_layer_animation_params(&layers);
        assert!((params[0][0] + std::f32::consts::SQRT_2).abs() < 0.0001);
        assert!((params[0][1] - std::f32::consts::SQRT_2).abs() < 0.0001);
        assert_eq!(params[0][2..], [1.0, 0.0]);
        assert_eq!(params[1], [0.0, 0.0, 1.0, 0.0]);
        assert_eq!(params[2], [0.0; 4]);
    }

    #[test]
    fn packed_mcal_channels_preserve_bytes_and_zero_pad_missing_layers() {
        let layers = [
            layer(0, 0, 0, None),
            layer(1, 0, 0, Some(vec![64, 128])),
            layer(2, 0, 0, None),
            layer(3, 0, 0, Some(vec![255])),
        ];
        let rgba = pack_alpha_map_bytes(&layers);
        assert_eq!(rgba.len(), 64 * 64 * 4);
        assert_eq!(
            &rgba[..12],
            &[64, 0, 255, 255, 128, 0, 0, 255, 0, 0, 0, 255]
        );
        assert_eq!(&rgba[rgba.len() - 4..], &[0, 0, 0, 255]);
    }
}
