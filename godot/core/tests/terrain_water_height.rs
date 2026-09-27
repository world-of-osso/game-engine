use game_engine_core::asset::adt_format::adt_tex::WaterLayer;
use game_engine_core::terrain_height_data::{
    WaterLayerSurface, layer_has_water, sample_water_layer_height,
};

const STEP: f32 = 100.0 / 3.0 / 8.0;

fn layer() -> WaterLayer {
    WaterLayer {
        liquid_type: 0,
        liquid_object: 0,
        min_height: 7.0,
        max_height: 40.0,
        x_offset: 5,
        y_offset: 2,
        width: 2,
        height: 2,
        exists: [0b10, 0, 0, 0, 0, 0, 0, 0],
        vertex_heights: vec![10.0, 14.0, 18.0, 20.0, 24.0, 28.0, 30.0, 34.0, 38.0],
        vertex_uvs: Vec::new(),
        vertex_depths: Vec::new(),
    }
}

fn sample(layer: &WaterLayer, row: f32, col: f32) -> Option<f32> {
    let surface = WaterLayerSurface::from_layer(layer, [-200.0, 100.0, 0.0]);
    sample_water_layer_height(&surface, 100.0 - row * STEP, 200.0 + col * STEP)
}

#[test]
fn existing_quad_interpolates_authored_vertex_heights_and_offsets() {
    let layer = layer();
    assert!(layer_has_water(&layer));
    let height = sample(&layer, 2.5, 6.5).expect("authored quad has water");
    assert!((height - 21.0).abs() < 0.001, "{height}");
    let corner = sample(&layer, 2.0, 6.0).expect("authored corner has water");
    assert!((corner - 14.0).abs() < 0.001, "{corner}");
}

#[test]
fn masked_quads_and_half_open_bounds_have_no_surface() {
    let layer = layer();
    assert_eq!(sample(&layer, 2.5, 5.5), None);
    assert_eq!(sample(&layer, 3.5, 6.5), None);
    assert_eq!(sample(&layer, 2.5, 7.0), None);
    assert_eq!(sample(&layer, 2.5, 4.9), None);
    assert_eq!(sample(&layer, 1.9, 6.5), None);
    assert_eq!(sample(&layer, 4.0, 6.5), None);
}

#[test]
fn empty_and_truncated_vertex_heights_use_min_height() {
    let mut layer = layer();
    layer.vertex_heights.clear();
    assert_eq!(sample(&layer, 2.5, 6.5), Some(7.0));
    layer.vertex_heights = vec![10.0];
    assert_eq!(sample(&layer, 2.5, 6.5), Some(7.0));
}

#[test]
fn selection_requires_existing_quad_within_layer_dimensions() {
    let mut layer = layer();
    layer.exists = [0; 8];
    assert!(!layer_has_water(&layer));
    layer.exists[7] = 0b1;
    assert!(!layer_has_water(&layer));
    layer.exists[0] = 0b10;
    assert!(layer_has_water(&layer));
    layer.width = 1;
    assert!(!layer_has_water(&layer));
}
