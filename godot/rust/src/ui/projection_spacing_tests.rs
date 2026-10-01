use super::{JustifyV, multiline_line_spacing, shadow_reserved_height};

#[test]
fn centered_money_keeps_three_original_font_height_lines_inside_shadow_bounds() {
    let available = 38.0 - shadow_reserved_height(JustifyV::Middle, 1.0);
    assert_eq!(available, 36.0);
    let spacing = multiline_line_spacing(available, 15.0, 3, 3);
    assert_eq!(spacing, -5);
    // Independent fixture geometry: three 15px lines, two gaps, centered +1px shadow.
    let text_height = 45.0 + 2.0 * spacing as f32;
    assert_eq!(text_height, 35.0);
    assert_eq!((38.0 - text_height) / 2.0 + text_height + 1.0, 37.5);
}

#[test]
fn fractional_negative_spacing_rounds_down_to_keep_all_lines_inside() {
    assert_eq!(multiline_line_spacing(38.0, 15.0, 3, 3), -4);
    assert_eq!(multiline_line_spacing(35.5, 15.0, 3, 3), -5);
}

#[test]
fn roomy_rectangles_never_increase_original_themed_spacing() {
    assert_eq!(multiline_line_spacing(80.0, 15.0, 3, 3), 3);
    assert_eq!(multiline_line_spacing(80.0, 15.0, 3, -2), -2);
}

#[test]
fn single_line_retains_themed_spacing() {
    assert_eq!(multiline_line_spacing(10.0, 15.0, 1, 3), 3);
    assert_eq!(multiline_line_spacing(10.0, 15.0, 0, 3), 3);
}

#[test]
fn shadow_reservation_depends_on_vertical_alignment_and_native_offset() {
    assert_eq!(shadow_reserved_height(JustifyV::Top, 2.0), 2.0);
    assert_eq!(shadow_reserved_height(JustifyV::Top, -2.0), 0.0);
    assert_eq!(shadow_reserved_height(JustifyV::Middle, 2.0), 4.0);
    assert_eq!(shadow_reserved_height(JustifyV::Middle, -2.0), 4.0);
    assert_eq!(shadow_reserved_height(JustifyV::Bottom, 2.0), 0.0);
    assert_eq!(shadow_reserved_height(JustifyV::Bottom, -2.0), 2.0);
    assert_eq!(shadow_reserved_height(JustifyV::Middle, 0.0), 0.0);
}
