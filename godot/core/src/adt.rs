//! ADT root, texture companion and object companion byte parsers.
use crate::asset::adt_format::{adt, adt_obj, adt_tex};

pub use adt::{BlendMeshData, ChunkHeightGrid, FlightBounds};
pub use adt_obj::{AdtObjData, ChunkObjectRefs, DoodadPlacement, WmoPlacement};
pub use adt_tex::{AdtTexData, AdtWaterData, ChunkTexLayers, TextureLayer, TextureParams};

pub struct Root {
    pub chunks: Vec<Chunk>,
    pub height_grids: Vec<ChunkHeightGrid>,
    pub center_surface: [f32; 3],
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
    pub holes_low_res: u16,
    pub holes_high_res: Option<u64>,
    pub shadow_map: Option<[u8; 512]>,
}

pub fn parse_root(data: &[u8]) -> Result<Root, String> {
    let parsed = adt::load_adt_parsed(data)?;
    Ok(Root {
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
                holes_low_res: chunk.holes_low_res,
                holes_high_res: chunk.holes_high_res,
                shadow_map: chunk.shadow_map,
            })
            .collect(),
        height_grids: parsed.height_grids,
        center_surface: parsed.center_surface,
        blend_mesh: parsed.blend_mesh,
        flight_bounds: parsed.flight_bounds,
        water: parsed.water,
        water_error: parsed.water_error,
    })
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
