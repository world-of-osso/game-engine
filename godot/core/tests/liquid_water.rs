//! Retail ADT water inputs: MH2O LVF 0 vertex depths and the DB2 water material chain.
use std::path::PathBuf;

use game_engine_core::adt::parse_root;
use game_engine_core::asset::adt_format::adt_tex::parse_mh2o;
use game_engine_core::liquid_data::{
    LiquidCatalog, LiquidMaterial, LiquidShader, WaterColorSource,
};

fn data_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../data")
}

fn catalog() -> LiquidCatalog {
    LiquidCatalog::read(&data_root().join("db2/12.1.0.69933")).expect("liquid DB2 CSVs")
}

/// One chunk-0 SLiquidInstance of 1x1 cells with `liquid_object_or_lvf` and vertex bytes.
fn mh2o_payload(liquid_object: u16, vertex_data: &[u8]) -> Vec<u8> {
    let header_size = 256 * 12;
    let mut payload = vec![0u8; header_size];
    payload[0..4].copy_from_slice(&(header_size as u32).to_le_bytes());
    payload[4..8].copy_from_slice(&1u32.to_le_bytes());
    let vertex_offset = header_size as u32 + 24;
    payload.extend_from_slice(&5u16.to_le_bytes());
    payload.extend_from_slice(&liquid_object.to_le_bytes());
    payload.extend_from_slice(&10.0f32.to_le_bytes());
    payload.extend_from_slice(&11.0f32.to_le_bytes());
    payload.extend_from_slice(&[0, 0, 1, 1]);
    payload.extend_from_slice(&0u32.to_le_bytes());
    payload.extend_from_slice(&vertex_offset.to_le_bytes());
    payload.extend_from_slice(vertex_data);
    payload
}

const HEIGHTS: [f32; 4] = [10.0, 10.25, 10.5, 11.0];

fn heights() -> Vec<u8> {
    HEIGHTS
        .iter()
        .flat_map(|height| height.to_le_bytes())
        .collect()
}

/// wowdev MH2O uvmap entries (u16 u, u16 v) for the four vertices.
fn uvs() -> Vec<u8> {
    [(0u16, 255u16), (51, 102), (255, 0), (128, 64)]
        .iter()
        .flat_map(|(u, v)| [u.to_le_bytes(), v.to_le_bytes()].concat())
        .collect()
}

const DEPTHS: [u8; 4] = [0, 40, 128, 255];

#[test]
fn lvf0_layer_keeps_authored_depths_after_heights() {
    let water =
        parse_mh2o(&mh2o_payload(427, &[heights(), DEPTHS.to_vec()].concat())).expect("LVF 0 MH2O");
    let layer = &water.chunks[0].layers[0];
    assert_eq!(layer.vertex_heights, HEIGHTS);
    assert_eq!(layer.vertex_depths, DEPTHS);
}

const PARSED_UVS: [[f32; 2]; 4] = [
    [0.0, 1.0],
    [0.2, 0.4],
    [1.0, 0.0],
    [128.0 / 255.0, 64.0 / 255.0],
];

#[test]
fn lvf1_layer_reads_height_array_then_uv_array() {
    let water = parse_mh2o(&mh2o_payload(1, &[heights(), uvs()].concat())).expect("LVF 1 MH2O");
    let layer = &water.chunks[0].layers[0];
    assert_eq!(layer.vertex_heights, HEIGHTS);
    assert_eq!(layer.vertex_uvs, PARSED_UVS);
    assert!(layer.vertex_depths.is_empty());
}

#[test]
fn lvf3_layer_reads_height_uv_and_depth_arrays() {
    let bytes = [heights(), uvs(), DEPTHS.to_vec()].concat();
    let water = parse_mh2o(&mh2o_payload(3, &bytes)).expect("LVF 3 MH2O");
    let layer = &water.chunks[0].layers[0];
    assert_eq!(layer.vertex_heights, HEIGHTS);
    assert_eq!(layer.vertex_uvs, PARSED_UVS);
    assert_eq!(layer.vertex_depths, DEPTHS);
}

#[test]
fn northshire_stream_layers_carry_varying_vertex_depth() {
    let bytes = std::fs::read(data_root().join("terrain/azeroth_32_48.adt")).expect("cached ADT");
    let water = parse_root(&bytes).expect("parse ADT").water.expect("MH2O");
    let layers: Vec<_> = water
        .chunks
        .iter()
        .flat_map(|chunk| &chunk.layers)
        .collect();
    assert!(!layers.is_empty());
    for layer in &layers {
        assert_eq!((layer.liquid_type, layer.liquid_object), (5, 427));
        let vertices = (layer.width as usize + 1) * (layer.height as usize + 1);
        assert_eq!(layer.vertex_depths.len(), vertices);
    }
    let depths: Vec<u8> = layers
        .iter()
        .flat_map(|layer| layer.vertex_depths.clone())
        .collect();
    assert_eq!(
        depths.iter().min(),
        Some(&0),
        "shore vertices are authored at depth 0"
    );
    assert!(
        depths.iter().any(|&depth| depth > 64),
        "stream centre is deeper"
    );
}

#[test]
fn liquid_object_427_is_slow_river_water_with_bump_and_foam_slots() {
    let material = catalog().liquid_material(5, 427).expect("LiquidObject 427");
    assert_eq!(material.liquid_type, 5);
    assert_eq!((material.material_id, material.lvf), (1, 0));
    assert_eq!(material.color_source, WaterColorSource::River);
    assert_eq!(material.shader, LiquidShader::Water);
    assert_eq!(material.texture_slots[1], [0]);
    assert_eq!(material.texture_slots[2], [463_849]);
    assert_eq!(material.texture_slots[3], [317_230]);
    assert_eq!((material.flow_direction, material.flow_speed), (0.0, 0.0));
    assert_eq!(
        material.floats[..16],
        [
            1.0, 0.0, 1.0, 1.0, 2.5, 90.0, 0.7, 0.5, 0.5, 0.2, 0.5, 0.2, 0.1, 1.0, 0.7, 2.5
        ]
    );
    assert_eq!(
        material.depth_coefficients,
        [-0.003_213_882_4, 1.011_566_2, 0.002_624_511_7, 0.009_704_59]
    );
    assert_eq!(material.wave_periods, [0.0, 0.0]);
}

#[test]
fn liquid_type_below_object_range_resolves_directly_and_ocean_uses_ocean_colours() {
    let catalog = catalog();
    let ocean = catalog.liquid_material(2, 2).expect("Ocean");
    assert_eq!(ocean.liquid_type, 2);
    assert_eq!(ocean.color_source, WaterColorSource::Ocean);
    let shallow = catalog.liquid_material(301, 0).expect("Shallow Water");
    assert_eq!(shallow.wave_periods, [1.0, 0.4]);
}

/// MH2O `(liquid_type, liquid_object)` pairs whose LiquidObject has no row in the active
/// 12.1.0.69933 DB2 or its hotfix caches: ocean 42 on every open-sea tile, and the five
/// Adventurer's Rest objects of tiles 2703_31_36 (FDID 5493433) and 2703_31_37 (5493438).
const OBJECTLESS_PAIRS: [(u16, u16); 6] = [
    (2, 42),
    (5, 13134),
    (5, 13136),
    (5, 13137),
    (81, 13138),
    (5, 13139),
];

/// WebWowViewerCpp `CSqliteDB::getLiquidObjectData`: without a LiquidObject row the layer's
/// own MH2O liquid_type is the LiquidType, with no flow. Ocean 42 stores LVF 2 depth-only
/// vertices; the others their material's LVF 0.
#[test]
fn objectless_liquid_layers_use_their_mh2o_liquid_type() {
    let catalog = catalog();
    for (liquid_type, liquid_object) in OBJECTLESS_PAIRS {
        let material = catalog
            .liquid_material(liquid_type, liquid_object)
            .unwrap_or_else(|error| panic!("({liquid_type}, {liquid_object}): {error}"));
        let direct = catalog.liquid_material(liquid_type, 0).expect("LiquidType");
        let lvf = if liquid_object == 42 { 2 } else { 0 };
        assert_eq!(
            material,
            LiquidMaterial { lvf, ..direct },
            "({liquid_type}, {liquid_object})"
        );
        assert_eq!(u32::from(liquid_type), material.liquid_type);
        assert_eq!(material.material_id, 1);
        assert_eq!(material.shader, LiquidShader::Water);
        assert_eq!((material.flow_direction, material.flow_speed), (0.0, 0.0));
    }
    assert_eq!(
        catalog.liquid_material(2, 42).expect("ocean").color_source,
        WaterColorSource::Ocean
    );
}

/// The MH2O payload of a root ADT (chunk tags are stored byte-reversed). These roots shadow
/// from `_tex0`, so a root-only `parse_root` cannot load them.
fn mh2o_chunk(adt: &[u8]) -> &[u8] {
    let mut offset = 0;
    while offset + 8 <= adt.len() {
        let size = u32::from_le_bytes(adt[offset + 4..offset + 8].try_into().unwrap()) as usize;
        if &adt[offset..offset + 4] == b"O2HM" {
            return &adt[offset + 8..offset + 8 + size];
        }
        offset += 8 + size;
    }
    panic!("root ADT has no MH2O");
}

/// Every Adventurer's Rest layer, 133 of them on objectless LiquidObjects, has a material
/// whose LVF reads its whole vertex block: ocean depths at sea level, river heights and
/// depths.
#[test]
fn adventurers_rest_tiles_resolve_every_liquid_layer() {
    let catalog = catalog();
    let mut objectless = 0;
    for fdid in [5_493_433, 5_493_438] {
        let bytes =
            std::fs::read(data_root().join(format!("terrain/{fdid}.adt"))).expect("cached ADT");
        let mut water = parse_mh2o(mh2o_chunk(&bytes)).expect("MH2O");
        for layer in water.chunks.iter_mut().flat_map(|chunk| &mut chunk.layers) {
            let key = (layer.liquid_type, layer.liquid_object);
            let material = catalog
                .liquid_material(key.0, key.1)
                .unwrap_or_else(|error| panic!("{fdid} {key:?}: {error}"));
            layer
                .decode_object_vertices(material.lvf)
                .unwrap_or_else(|error| panic!("{fdid} {key:?}: {error}"));
            if !layer.object_vertex_bytes.is_empty() {
                assert_eq!(layer.vertex_depths.len(), 81, "{fdid} {key:?}");
                let heights = if key == (2, 42) { 0 } else { 81 };
                assert_eq!(layer.vertex_heights.len(), heights, "{fdid} {key:?}");
            }
            objectless += usize::from(OBJECTLESS_PAIRS.contains(&key));
        }
    }
    assert_eq!(objectless, 133);
}

#[test]
fn unknown_liquid_type_is_an_error() {
    let error = catalog().liquid_material(65_000, 0).unwrap_err();
    assert_eq!(error, "LiquidType 65000 has no DB2 row");
}

#[test]
fn magma_and_slime_use_the_magma_shader_with_lvf1_layers_and_int2_base_colour() {
    let catalog = catalog();
    let magma = catalog.liquid_material(3, 3).expect("Magma");
    assert_eq!(magma.shader, LiquidShader::Magma);
    assert_eq!((magma.material_id, magma.lvf), (2, 1));
    assert_eq!(magma.texture_slots[0].len(), 30);
    assert_eq!(
        magma.texture_slots[1..4],
        [vec![340_156], vec![340_157], vec![340_158]]
    );
    assert_eq!(magma.ints[2], 0x00ff_6600);
    let slime = catalog.liquid_material(4, 4).expect("Slime");
    assert_eq!(slime.shader, LiquidShader::Magma);
    assert_eq!(slime.texture_slots[1], [374_986]);
}

/// Searing Gorge tile azeroth_34_46 (FDID 778417): its magma LiquidObjects are LVF 1, so
/// each layer re-reads its vertices as a height array followed by a UV array.
#[test]
fn searing_gorge_magma_layers_reread_as_lvf1() {
    let catalog = catalog();
    let bytes = std::fs::read(data_root().join("terrain/778417.adt")).expect("cached ADT");
    let mut water = parse_root(&bytes).expect("parse ADT").water.expect("MH2O");
    let layers: Vec<_> = water
        .chunks
        .iter_mut()
        .flat_map(|chunk| &mut chunk.layers)
        .collect();
    assert_eq!(layers.len(), 53);
    for layer in layers {
        let material = catalog
            .liquid_material(layer.liquid_type, layer.liquid_object)
            .expect("magma material");
        assert_eq!((material.shader, material.lvf), (LiquidShader::Magma, 1));
        layer.decode_object_vertices(material.lvf).expect("LVF 1");
        let vertices = (layer.width as usize + 1) * (layer.height as usize + 1);
        assert_eq!(layer.vertex_heights.len(), vertices);
        assert_eq!(layer.vertex_uvs.len(), vertices);
        assert!(layer.vertex_depths.is_empty());
    }
}
