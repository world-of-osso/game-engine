use std::path::Path;

const BORDER_BASE: &str = "/home/osso/Projects/wow/Interface/COMMON/Common-Input-Border-";
const NINE_SLICE_PARTS: [&str; 9] = ["TL", "T", "TR", "L", "M", "R", "BL", "B", "BR"];
const SCALE: u32 = 8;
const PART_SIZE: u32 = 8;
const CHECKER_TILE: u32 = 4;

fn fill_checkerboard(img: &mut image::RgbaImage) {
    for y in 0..img.height() {
        for x in 0..img.width() {
            let checker = if ((x / CHECKER_TILE) + (y / CHECKER_TILE)) % 2 == 0 {
                80
            } else {
                60
            };
            img.put_pixel(x, y, image::Rgba([checker, checker, checker, 255]));
        }
    }
}

fn tint_pixel(pixel: &[u8], tint: [f32; 3]) -> (u8, u8, u8, f32) {
    let r = (pixel[0] as f32 * tint[0]).round().min(255.0) as u8;
    let g = (pixel[1] as f32 * tint[1]).round().min(255.0) as u8;
    let b = (pixel[2] as f32 * tint[2]).round().min(255.0) as u8;
    let alpha = pixel[3] as f32 / 255.0;
    (r, g, b, alpha)
}

fn blit_scaled_pixel(
    img: &mut image::RgbaImage,
    base_x: u32,
    base_y: u32,
    r: u8,
    g: u8,
    b: u8,
    alpha: f32,
) {
    for sy in 0..SCALE {
        for sx in 0..SCALE {
            let dx = base_x + sx;
            let dy = base_y + sy;
            let bg = img.get_pixel(dx, dy);
            let nr = (r as f32 * alpha + bg[0] as f32 * (1.0 - alpha)) as u8;
            let ng = (g as f32 * alpha + bg[1] as f32 * (1.0 - alpha)) as u8;
            let nb = (b as f32 * alpha + bg[2] as f32 * (1.0 - alpha)) as u8;
            img.put_pixel(dx, dy, image::Rgba([nr, ng, nb, 255]));
        }
    }
}

fn blit_tinted_part(
    img: &mut image::RgbaImage,
    pixels: &[u8],
    w: u32,
    h: u32,
    grid_x: u32,
    grid_y: u32,
    tint: [f32; 3],
) {
    for y in 0..h {
        for x in 0..w {
            let src = ((y * w + x) * 4) as usize;
            let (r, g, b, alpha) = tint_pixel(&pixels[src..src + 4], tint);
            let base_x = grid_x * PART_SIZE * SCALE + x * SCALE;
            let base_y = grid_y * PART_SIZE * SCALE + y * SCALE;
            blit_scaled_pixel(img, base_x, base_y, r, g, b, alpha);
        }
    }
}

fn render_variant(variant: &str, tint: [f32; 3]) {
    let side = PART_SIZE * SCALE * 3;
    let mut img = image::RgbaImage::new(side, side);
    fill_checkerboard(&mut img);

    for (idx, part) in NINE_SLICE_PARTS.iter().enumerate() {
        let path = format!("{BORDER_BASE}{part}.blp");
        let (pixels, w, h) = game_engine::asset::blp::load_blp_rgba(Path::new(&path)).unwrap();
        let grid_x = (idx % 3) as u32;
        let grid_y = (idx / 3) as u32;
        blit_tinted_part(&mut img, &pixels, w, h, grid_x, grid_y, tint);
    }

    let out = format!("/tmp/editbox-{variant}-preview.png");
    img.save(&out).unwrap();
    eprintln!("Wrote {out} ({side}x{side})");
}

fn main() {
    render_variant("dark", [0.09, 0.07, 0.05]);
    render_variant("focused", [0.22, 0.16, 0.11]);
}
