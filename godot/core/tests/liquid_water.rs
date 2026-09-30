//! Retail ADT water inputs: MH2O LVF 0 vertex depths and the DB2 water material chain.
use std::path::PathBuf;

use game_engine_core::adt::parse_root;
use game_engine_core::asset::adt_format::adt_tex::parse_mh2o;
use game_engine_core::liquid_data::{LiquidCatalog, WaterColorSource};

fn data_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../data")
}

fn catalog() -> LiquidCatalog {
    LiquidCatalog::read(&data_root().join("db2/12.1.0.69933")).expect("liquid DB2 CSVs")
}

/// One chunk-0 SLiquidInstance of 1x1 cells: LiquidObject 427, 4 heights then 4 depths.
fn lvf0_payload() -> Vec<u8> {
    let header_size = 256 * 12;
    let mut payload = vec![0u8; header_size];
    payload[0..4].copy_from_slice(&(header_size as u32).to_le_bytes());
    payload[4..8].copy_from_slice(&1u32.to_le_bytes());
    let vertex_offset = header_size as u32 + 24;
    payload.extend_from_slice(&5u16.to_le_bytes());
    payload.extend_from_slice(&427u16.to_le_bytes());
    payload.extend_from_slice(&10.0f32.to_le_bytes());
    payload.extend_from_slice(&11.0f32.to_le_bytes());
    payload.extend_from_slice(&[0, 0, 1, 1]);
    payload.extend_from_slice(&0u32.to_le_bytes());
    payload.extend_from_slice(&vertex_offset.to_le_bytes());
    for height in [10.0f32, 10.25, 10.5, 11.0] {
        payload.extend_from_slice(&height.to_le_bytes());
    }
    payload.extend_from_slice(&[0, 40, 128, 255]);
    payload
}

#[test]
fn lvf0_layer_keeps_authored_depths_after_heights() {
    let water = parse_mh2o(&lvf0_payload()).expect("LVF 0 MH2O");
    let layer = &water.chunks[0].layers[0];
    assert_eq!(layer.vertex_heights, [10.0, 10.25, 10.5, 11.0]);
    assert_eq!(layer.vertex_depths, [0, 40, 128, 255]);
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
    let material = catalog().water_material(5, 427).expect("LiquidObject 427");
    assert_eq!(material.liquid_type, 5);
    assert_eq!((material.material_id, material.lvf), (1, 0));
    assert_eq!(material.color_source, WaterColorSource::River);
    assert_eq!(material.bump_frames, [463_849]);
    assert_eq!(material.foam_frames, [317_230]);
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
    let ocean = catalog.water_material(2, 2).expect("Ocean");
    assert_eq!(ocean.liquid_type, 2);
    assert_eq!(ocean.color_source, WaterColorSource::Ocean);
    let shallow = catalog.water_material(301, 0).expect("Shallow Water");
    assert_eq!(shallow.wave_periods, [1.0, 0.4]);
}

#[test]
fn unknown_liquid_object_is_an_error() {
    let error = catalog().water_material(5, 65_000).unwrap_err();
    assert_eq!(error, "LiquidObject 65000 has no DB2 row");
}
