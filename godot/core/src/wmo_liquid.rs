//! WMO group liquid (MLIQ) as WebWowViewerCpp draws it: the group's LiquidType
//! (`WmoGroupObject::setLiquidType`, `objects/wmo/wmoGroupObject.cpp:151-201`) and its
//! surface (`WmoGroupGeom::getWaterVertexBindings`, `geometry/wmoGroupGeom.cpp:512-616`).
use crate::adt::WaterGeometry;
use crate::asset::wmo_format::parser::{
    WmoGroupHeader, WmoLiquid, WmoRootFlags, wmo_local_to_bevy,
};

/// LiquidType IDs of the basic WMO liquids (`geometry/wmoGroupGeom.h:14-30`).
pub const WMO_WATER: u16 = 13;
pub const WMO_OCEAN: u16 = 14;
const GREEN_LAVA: u32 = 15;
pub const WMO_MAGMA: u16 = 19;
pub const WMO_SLIME: u16 = 20;
/// `LIQUID_END_BASIC_LIQUIDS` and `LIQUID_FIRST_NONBASIC_LIQUID_TYPE`.
const END_BASIC_LIQUIDS: u32 = 20;
const FIRST_NONBASIC_LIQUID: u32 = 21;
/// MOGP `is_not_water_but_ocean` (`persistance/header/wmoFileHeader.h:102-103`).
const GROUP_OCEAN: u32 = 0x8_0000;
/// MOGP exterior-lit, exterior and interior (`wmoGroupObject.h:130-135`).
const GROUP_EXTERIOR_LIT: u32 = 0x40;
const GROUP_EXTERIOR: u32 = 0x8;
const GROUP_INTERIOR: u32 = 0x2000;
/// A tile whose legacy type (low nibble) is 15 is not drawn.
const TILE_HIDDEN: u8 = 0x0F;
/// Yards per liquid tile (`MathHelper::UNITSIZE`, 1600 / 3 / 16 / 8).
const UNIT_SIZE: f32 = 1600.0 / 3.0 / 16.0 / 8.0;
/// Water texture coordinates repeat every chunk (1600 / 3 / 16 yd).
const CHUNK_SIZE: f32 = 1600.0 / 3.0 / 16.0;

/// The LiquidType of `group`'s liquid; `None` when it resolves to no liquid.
pub fn group_liquid_type(root: WmoRootFlags, group: &WmoGroupHeader) -> Option<u16> {
    let liquid = group.group_liquid;
    let resolved = if root.use_liquid_type_dbc_id {
        if liquid < FIRST_NONBASIC_LIQUID {
            basic_liquid(liquid.wrapping_sub(1), group.flags)
        } else {
            liquid
        }
    } else if liquid == GREEN_LAVA {
        0
    } else if liquid < END_BASIC_LIQUIDS {
        basic_liquid(liquid, group.flags)
    } else {
        liquid + 1
    };
    u16::try_from(resolved).ok().filter(|&id| id != 0)
}

/// `to_wmo_liquid`: the low two bits are water, ocean, magma, slime.
fn basic_liquid(liquid: u32, group_flags: u32) -> u32 {
    match liquid & 3 {
        0 if group_flags & GROUP_OCEAN != 0 => u32::from(WMO_OCEAN),
        0 => u32::from(WMO_WATER),
        1 => u32::from(WMO_OCEAN),
        2 => u32::from(WMO_MAGMA),
        _ => u32::from(WMO_SLIME),
    }
}

/// `isInteriorLightingLit`: an interior group lit by neither the exterior nor its own
/// exterior-lit flag, whose procedural WMO water is white.
pub fn group_interior_lit(group_flags: u32) -> bool {
    group_flags & (GROUP_EXTERIOR_LIT | GROUP_EXTERIOR) == 0 && group_flags & GROUP_INTERIOR != 0
}

/// The drawn tiles of `liquid` in WMO-local Godot axes, before the WMO placement: each
/// vertex at its MLIQ height, depth 1 (alpha), and texture coordinates in chunks of
/// WMO-local position, or the magma vertex `s, t * 3 / 256` for `liquid_type` 19.
pub fn liquid_geometry(liquid: &WmoLiquid, liquid_type: u16) -> WaterGeometry {
    let (positions, uvs) = liquid_vertices(liquid, liquid_type);
    let count = positions.len();
    WaterGeometry {
        positions,
        normals: vec![[0.0, 1.0, 0.0]; count],
        uvs,
        colors: vec![[1.0; 4]; count],
        indices: drawn_tile_indices(liquid),
    }
}

/// Godot-axis positions and texture coordinates of every MLIQ vertex.
fn liquid_vertices(liquid: &WmoLiquid, liquid_type: u16) -> (Vec<[f32; 3]>, Vec<[f32; 2]>) {
    let header = &liquid.header;
    let x_verts = header.x_verts.max(1) as usize;
    liquid
        .vertices
        .iter()
        .enumerate()
        .map(|(index, vertex)| {
            let (i, j) = (index % x_verts, index / x_verts);
            let x = header.position[0] + UNIT_SIZE * i as f32;
            let y = header.position[1] + UNIT_SIZE * j as f32;
            let uv = if liquid_type == WMO_MAGMA {
                let [s0, s1, t0, t1] = vertex.raw;
                let (s, t) = (i16::from_le_bytes([s0, s1]), i16::from_le_bytes([t0, t1]));
                [f32::from(s) * 3.0 / 256.0, f32::from(t) * 3.0 / 256.0]
            } else {
                [x / CHUNK_SIZE, y / CHUNK_SIZE]
            };
            (wmo_local_to_bevy(x, y, vertex.height), uv)
        })
        .unzip()
}

/// Two triangles per drawn tile, (0, 1, 2) and (0, 2, 3) of its corners.
fn drawn_tile_indices(liquid: &WmoLiquid) -> Vec<u32> {
    let header = &liquid.header;
    let (x_verts, x_tiles, y_tiles) = (
        header.x_verts.max(0) as usize,
        header.x_tiles.max(0) as usize,
        header.y_tiles.max(0) as usize,
    );
    let mut indices = Vec::new();
    for j in 0..y_tiles {
        for i in 0..x_tiles {
            let drawn = liquid
                .tiles
                .get(j * x_tiles + i)
                .is_some_and(|tile| tile.liquid_type != TILE_HIDDEN);
            let corner = |di: usize, dj: usize| ((j + dj) * x_verts + i + di) as u32;
            let quad = [corner(0, 0), corner(1, 0), corner(1, 1), corner(0, 1)];
            if drawn && quad.iter().all(|&v| (v as usize) < liquid.vertices.len()) {
                indices.extend([quad[0], quad[1], quad[2], quad[0], quad[2], quad[3]]);
            }
        }
    }
    indices
}
