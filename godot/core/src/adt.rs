//! ADT root, texture companion and object companion byte parsers.
use crate::asset::adt_format::{adt, adt_geometry, adt_obj, adt_tex};

pub use adt::{
    BlendBatch, BlendMeshData, ChunkHeightGrid, FlightBounds, ParsedLodData, SoundEmitter,
    UNIT_SIZE,
};
pub use adt_geometry::Geometry;
pub use adt_obj::{AdtObjData, ChunkObjectRefs, DoodadPlacement, WmoPlacement};
pub use adt_tex::{AdtTexData, AdtWaterData, ChunkTexLayers, TextureLayer, TextureParams};

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
