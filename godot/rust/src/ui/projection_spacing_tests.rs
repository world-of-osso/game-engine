use ui_toolkit::font_line_metrics::{FontLineRequest, resolve_font_lines};

// These replace the old15UI/negative-gap squeezing fixtures. Native process
// coverage in fontstring_raster_lines.gd proves the actual consumer uses this rule.
#[test]
fn centered_money_allocates_three_raster_lines_without_shadow_padding_or_squeezing() {
    let metrics = resolve_font_lines(FontLineRequest {
        nominal_height: 12.0,
        pixels_per_ui: 1080.0 / 768.0,
        spacing: 0.0,
        line_count: 3,
        face_ascent: 965.0,
        face_descent: -250.0,
    })
    .unwrap();
    assert!((metrics.block_height_ui - 36.266666).abs() < 0.00001);
    assert_eq!(metrics.spacing_ui, 0.0);
    // MIDDLE positions the whole allocated block in the authored38UI region.
    let first_baseline = (38.0 - metrics.block_height_ui) / 2.0 + metrics.baseline_ui;
    assert!((first_baseline - 10.822223).abs() < 0.00001);
}

#[test]
fn positive_spacing_allocation_retains_the_named_native_truncation_gap() {
    let metrics = resolve_font_lines(FontLineRequest {
        nominal_height: 12.0,
        pixels_per_ui: 1080.0 / 768.0,
        spacing: 1.0,
        line_count: 2,
        face_ascent: 965.0,
        face_descent: -250.0,
    })
    .unwrap();
    assert!((metrics.block_height_ui - 25.6).abs() < 0.00001);
    let native_integer_spacing = metrics.spacing_ui as i32;
    assert_eq!(native_integer_spacing, 1);
    assert!((native_integer_spacing as f32 - metrics.spacing_ui + 0.422222).abs() < 0.00001);
}
