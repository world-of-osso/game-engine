//! Byte-exact M2 secondary-texture and overlay composition, independent of rendering and IO.

#[path = "rgba_blit.rs"]
mod rgba_blit;
use rgba_blit::{blit_region, scale_2x as scale_2x_pixels};

pub fn composite_second_texture_pixels(
    base_pixels: &mut [u8],
    base_width: u32,
    base_height: u32,
    overlay_pixels: &[u8],
    overlay_width: u32,
    overlay_height: u32,
    shader_id: u16,
) {
    for y in 0..base_height {
        for x in 0..base_width {
            let base_idx = ((y * base_width + x) * 4) as usize;
            let ox = x.rem_euclid(overlay_width);
            let oy = y.rem_euclid(overlay_height);
            let overlay_idx = ((oy * overlay_width + ox) * 4) as usize;
            let base = &mut base_pixels[base_idx..base_idx + 4];
            let overlay = &overlay_pixels[overlay_idx..overlay_idx + 4];
            apply_m2_multitexture_shader(base, overlay, shader_id);
        }
    }
}

pub fn composite_overlay_pixels(
    base: &mut [u8],
    base_width: u32,
    overlay: &[u8],
    overlay_width: u32,
    overlay_height: u32,
    x: u32,
    y: u32,
    scale_2x: bool,
) {
    if scale_2x {
        let (scaled, width, height) = scale_2x_pixels(overlay, overlay_width, overlay_height);
        blit_region(base, base_width, &scaled, width, height, x, y);
    } else {
        blit_region(
            base,
            base_width,
            overlay,
            overlay_width,
            overlay_height,
            x,
            y,
        );
    }
}

const M2_SHADER_ALPHA_MASK: u16 = 0x8000;
const M2_SHADER_MOD_2X_ALPHA: u16 = 0x4014;

pub fn apply_m2_multitexture_shader(base: &mut [u8], overlay: &[u8], shader_id: u16) {
    let base_rgb = [
        base[0] as f32 / 255.0,
        base[1] as f32 / 255.0,
        base[2] as f32 / 255.0,
    ];
    let base_a = base[3] as f32 / 255.0;
    let overlay_rgb = [
        overlay[0] as f32 / 255.0,
        overlay[1] as f32 / 255.0,
        overlay[2] as f32 / 255.0,
    ];
    let overlay_a = overlay[3] as f32 / 255.0;

    let (rgb, a) = shader_blend(base_rgb, base_a, overlay_rgb, overlay_a, shader_id);

    base[0] = (rgb[0] * 255.0).round() as u8;
    base[1] = (rgb[1] * 255.0).round() as u8;
    base[2] = (rgb[2] * 255.0).round() as u8;
    base[3] = (a * 255.0).round() as u8;
}

fn shader_blend(
    base_rgb: [f32; 3],
    base_a: f32,
    overlay_rgb: [f32; 3],
    overlay_a: f32,
    shader_id: u16,
) -> ([f32; 3], f32) {
    match shader_id {
        M2_SHADER_ALPHA_MASK => (base_rgb, (base_a * overlay_a).clamp(0.0, 1.0)),
        M2_SHADER_MOD_2X_ALPHA => (
            mul_2x_rgb(base_rgb, overlay_rgb),
            (base_a * overlay_a * 2.0).clamp(0.0, 1.0),
        ),
        0x0010 => (mul_rgb(base_rgb, overlay_rgb), base_a),
        0x0011 => (
            mul_rgb(base_rgb, overlay_rgb),
            (base_a * overlay_a).clamp(0.0, 1.0),
        ),
        0x4016 => (mul_2x_rgb(base_rgb, overlay_rgb), base_a),
        0x8015 => (add_overlay_rgb(base_rgb, overlay_rgb, overlay_a, 1.0), 1.0),
        0x8001 => (shader_8001_rgb(base_rgb, base_a, overlay_rgb), 1.0),
        0x8002 => (add_overlay_rgb(base_rgb, overlay_rgb, overlay_a, 1.0), 1.0),
        0x8003 => (
            add_overlay_rgb(base_rgb, overlay_rgb, overlay_a, base_a),
            1.0,
        ),
        _ => (base_rgb, base_a),
    }
}

fn mul_rgb(base_rgb: [f32; 3], overlay_rgb: [f32; 3]) -> [f32; 3] {
    [
        (base_rgb[0] * overlay_rgb[0]).clamp(0.0, 1.0),
        (base_rgb[1] * overlay_rgb[1]).clamp(0.0, 1.0),
        (base_rgb[2] * overlay_rgb[2]).clamp(0.0, 1.0),
    ]
}

fn mul_2x_rgb(base_rgb: [f32; 3], overlay_rgb: [f32; 3]) -> [f32; 3] {
    [
        (base_rgb[0] * overlay_rgb[0] * 2.0).clamp(0.0, 1.0),
        (base_rgb[1] * overlay_rgb[1] * 2.0).clamp(0.0, 1.0),
        (base_rgb[2] * overlay_rgb[2] * 2.0).clamp(0.0, 1.0),
    ]
}

fn add_overlay_rgb(
    base_rgb: [f32; 3],
    overlay_rgb: [f32; 3],
    overlay_a: f32,
    weight: f32,
) -> [f32; 3] {
    [
        (base_rgb[0] + overlay_rgb[0] * overlay_a * weight).clamp(0.0, 1.0),
        (base_rgb[1] + overlay_rgb[1] * overlay_a * weight).clamp(0.0, 1.0),
        (base_rgb[2] + overlay_rgb[2] * overlay_a * weight).clamp(0.0, 1.0),
    ]
}

fn shader_8001_rgb(base_rgb: [f32; 3], base_a: f32, overlay_rgb: [f32; 3]) -> [f32; 3] {
    [
        (base_rgb[0] * ((overlay_rgb[0] * 2.0) * (1.0 - base_a) + base_a)).clamp(0.0, 1.0),
        (base_rgb[1] * ((overlay_rgb[1] * 2.0) * (1.0 - base_a) + base_a)).clamp(0.0, 1.0),
        (base_rgb[2] * ((overlay_rgb[2] * 2.0) * (1.0 - base_a) + base_a)).clamp(0.0, 1.0),
    ]
}

#[cfg(test)]
mod tests {
    use super::{composite_overlay_pixels, composite_second_texture_pixels};

    #[test]
    fn shader_ids_preserve_original_byte_rounding_and_alpha_rules() {
        let cases = [
            (0x8000, [128, 64, 32, 32]),
            (0x4014, [0, 64, 16, 64]),
            (0x0010, [0, 32, 8, 128]),
            (0x0011, [0, 32, 8, 32]),
            (0x4016, [0, 64, 16, 128]),
            (0x8015, [128, 96, 48, 255]),
            (0x8001, [64, 64, 24, 255]),
            (0x8002, [128, 96, 48, 255]),
            (0x8003, [128, 80, 40, 255]),
            (0xffff, [128, 64, 32, 128]),
        ];
        for (shader, expected) in cases {
            let mut base = [128, 64, 32, 128];
            let overlay = [0, 128, 64, 64];
            composite_second_texture_pixels(&mut base, 1, 1, &overlay, 1, 1, shader);
            assert_eq!(base, expected, "shader {shader:#06x}");
        }
    }

    #[test]
    fn secondary_texture_tiles_across_both_axes() {
        let mut base = vec![255; 3 * 3 * 4];
        let overlay = [
            10, 20, 30, 40, 50, 60, 70, 80, // row 0
            90, 100, 110, 120, 130, 140, 150, 160, // row 1
        ];
        composite_second_texture_pixels(&mut base, 3, 3, &overlay, 2, 2, 0x0011);
        let pixels: Vec<_> = base.chunks_exact(4).map(|pixel| pixel.to_vec()).collect();
        assert_eq!(
            pixels,
            [
                vec![10, 20, 30, 40],
                vec![50, 60, 70, 80],
                vec![10, 20, 30, 40],
                vec![90, 100, 110, 120],
                vec![130, 140, 150, 160],
                vec![90, 100, 110, 120],
                vec![10, 20, 30, 40],
                vec![50, 60, 70, 80],
                vec![10, 20, 30, 40],
            ]
        );
    }

    #[test]
    fn overlay_blit_uses_integer_truncation_and_max_alpha() {
        let mut base = [10, 20, 30, 200, 99, 88, 77, 66];
        let overlay = [100, 120, 140, 128, 4, 5, 6, 0];
        composite_overlay_pixels(&mut base, 2, &overlay, 2, 1, 0, 0, false);
        assert_eq!(base, [55, 70, 85, 200, 99, 88, 77, 66]);
    }

    #[test]
    fn scaled_overlay_repeats_nearest_pixels_and_clips_at_base_end() {
        let mut base = [0; 3 * 2 * 4];
        let overlay = [10, 20, 30, 255, 90, 80, 70, 255];
        composite_overlay_pixels(&mut base, 3, &overlay, 2, 1, 1, 0, true);
        assert_eq!(
            &base[0..12],
            &[0, 0, 0, 0, 10, 20, 30, 255, 10, 20, 30, 255]
        );
        assert_eq!(
            &base[12..24],
            &[0, 0, 0, 0, 10, 20, 30, 255, 10, 20, 30, 255]
        );
    }
}
