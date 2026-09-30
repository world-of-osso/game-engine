//! WMO group liquids (MLIQ): LiquidType resolution and surface as WebWowViewerCpp builds
//! them (`wmoGroupObject.cpp:151-201`, `wmoGroupGeom.cpp:512-616`).
use game_engine_core::{
    asset::wmo_format::parser::{
        WmoGroupFlags, WmoGroupHeader, WmoLiquid, WmoLiquidHeader, WmoLiquidTile,
        WmoLiquidVertex, WmoRootFlags,
    },
    wmo::{parse_group, parse_root},
    wmo_liquid::{group_interior_lit, group_liquid_type, liquid_geometry},
};

fn read(fdid: u32) -> Vec<u8> {
    std::fs::read(format!("data/models/{fdid}.wmo")).unwrap()
}

fn header(flags: u32, group_liquid: u32) -> WmoGroupHeader {
    WmoGroupHeader {
        group_name_offset: 0,
        descriptive_group_name_offset: 0,
        flags,
        group_flags: WmoGroupFlags::from_bits(flags),
        bbox_min: [0.0; 3],
        bbox_max: [0.0; 3],
        portal_start: 0,
        portal_count: 0,
        trans_batch_count: 0,
        int_batch_count: 0,
        ext_batch_count: 0,
        batch_type_d: 0,
        fog_ids: [0; 4],
        group_liquid,
        unique_id: 0,
        flags2: 0,
        parent_split_group_index: -1,
        next_split_child_group_index: -1,
    }
}

/// `setLiquidType`: with MOHD 0x4 a basic id n below 21 is `to_wmo_liquid(n - 1)`, any other
/// a LiquidType id; without it n below 20 is `to_wmo_liquid(n)`, Green Lava (15) none, and
/// others n + 1. `to_wmo_liquid` takes the low two bits: water (ocean with MOGP 0x80000),
/// ocean, magma, slime.
#[test]
fn group_liquid_types_follow_set_liquid_type() {
    let dbc = WmoRootFlags::from_bits(0x4);
    let legacy = WmoRootFlags::from_bits(0);
    const OCEAN_GROUP: u32 = 0x8_0000;
    for (root, flags, liquid, expected) in [
        (dbc, 0, 1, Some(13)),
        (dbc, OCEAN_GROUP, 1, Some(14)),
        (dbc, 0, 2, Some(14)),
        (dbc, 0, 3, Some(19)),
        (dbc, 0, 4, Some(20)),
        (dbc, 0, 5, Some(13)),
        (dbc, 0, 21, Some(21)),
        (dbc, 0, 1200, Some(1200)),
        (legacy, 0, 0, Some(13)),
        (legacy, OCEAN_GROUP, 4, Some(14)),
        (legacy, 0, 1, Some(14)),
        (legacy, 0, 2, Some(19)),
        (legacy, 0, 3, Some(20)),
        (legacy, 0, 15, None),
        (legacy, 0, 20, Some(21)),
    ] {
        assert_eq!(
            group_liquid_type(root, &header(flags, liquid)),
            expected,
            "root {root:?} flags {flags:#x} liquid {liquid}"
        );
    }
}

/// `isInteriorLightingLit`: interior (0x2000) without exterior (0x8) or exterior-lit (0x40).
#[test]
fn interior_lighting_needs_an_unlit_interior_group() {
    assert!(group_interior_lit(0x2000));
    assert!(!group_interior_lit(0x2008));
    assert!(!group_interior_lit(0x2040));
    assert!(!group_interior_lit(0x1009));
}

/// Northshire Abbey's gate fountain (`abbeygate01.wmo` 108104, group 108105): MOHD 0x5,
/// group liquid 5 → WMO Water 13, an exterior group. 3 × 4 tiles at (-29.17, -8.33); tile
/// 0x3F (low nibble 15) and 0x0F are not drawn, 10 are. Positions are WMO-local in Godot
/// axes (x, z, -y); texture coordinates are WMO-local position / 33.33 yd.
#[test]
fn abbey_gate_fountain_draws_ten_water_tiles() {
    let root = parse_root(&read(108_104)).unwrap();
    let group = parse_group(&read(108_105)).unwrap();
    let liquid = group.geometry.liquid.as_ref().unwrap();
    assert_eq!(group_liquid_type(root.flags, &group.header), Some(13));
    assert!(!group_interior_lit(group.header.flags));
    assert_eq!(liquid.tiles[2].liquid_type, 0x0F, "0x3F keeps only its low nibble");
    let geometry = liquid_geometry(liquid, 13);
    assert_eq!(geometry.positions.len(), 20);
    assert_eq!(geometry.indices.len(), 10 * 6);
    let [x, y, z] = geometry.positions[0];
    assert!((x + 29.166_666).abs() < 1e-4 && (y - 1.463_379).abs() < 1e-4 && (z - 8.333_333).abs() < 1e-4);
    let [u, v] = geometry.uvs[0];
    assert!((u + 0.875).abs() < 1e-5 && (v + 0.25).abs() < 1e-5, "{u} {v}");
    // Tile 0: vertices 0, 1, 5, 4 in two triangles; the hidden tile 2 uses vertex 3.
    assert_eq!(&geometry.indices[..6], &[0, 1, 5, 0, 5, 4]);
    assert!(!geometry.indices.contains(&3));
    assert!(geometry.colors.iter().all(|&color| color == [1.0; 4]));
}

/// Cultists' Quay delve (5356285, group 5533972): an interior-lit group of WMO Water.
#[test]
fn cultists_quay_cave_water_is_interior_wmo_water() {
    let root = parse_root(&read(5_356_285)).unwrap();
    let group = parse_group(&read(5_533_972)).unwrap();
    assert_eq!(group_liquid_type(root.flags, &group.header), Some(13));
    assert!(group_interior_lit(group.header.flags));
    let geometry = liquid_geometry(group.geometry.liquid.as_ref().unwrap(), 13);
    assert_eq!(geometry.indices.len(), 2804 * 6);
}

/// WMO Magma (19) maps the MLIQ magma vertex `s, t` (int16) * 3 / 256; other types the
/// position.
#[test]
fn wmo_magma_uses_the_vertex_texture_coordinates() {
    let vertex = |s: i16, t: i16, height| {
        let [s0, s1] = s.to_le_bytes();
        let [t0, t1] = t.to_le_bytes();
        WmoLiquidVertex {
            raw: [s0, s1, t0, t1],
            height,
        }
    };
    let liquid = WmoLiquid {
        header: WmoLiquidHeader {
            x_verts: 2,
            y_verts: 2,
            x_tiles: 1,
            y_tiles: 1,
            position: [100.0, 50.0, 0.0],
            material_id: 0,
        },
        vertices: vec![
            vertex(256, -512, 2.0),
            vertex(0, 0, 2.0),
            vertex(0, 0, 2.0),
            vertex(0, 0, 3.0),
        ],
        tiles: vec![WmoLiquidTile {
            liquid_type: 2,
            fishable: false,
            shared: false,
        }],
    };
    let magma = liquid_geometry(&liquid, 19);
    assert_eq!(magma.uvs[0], [3.0, -6.0]);
    let slime = liquid_geometry(&liquid, 20);
    assert_eq!(slime.uvs[0], [3.0, 1.5]);
    // Vertex 3 at (i 1, j 1): x 100 + 4.1667, height 3 → Godot (104.17, 3, -54.17).
    let [x, y, z] = magma.positions[3];
    assert!((x - 104.166_67).abs() < 1e-3 && y == 3.0 && (z + 54.166_67).abs() < 1e-3);
}
