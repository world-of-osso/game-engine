use super::*;

const TAGS: [(u32, u8); 3] = [
    (METAL_CORNERS, 1),
    (METAL_SIDE_EDGES, 2),
    (METAL_TOP_BOTTOM_EDGES, 3),
];

/// 512×512 fake texture whose pixels are `[tag, x, y, 255]` (coordinates mod 256).
fn tagged_source(fdid: u32) -> Result<(Vec<u8>, u32), String> {
    let tag = TAGS
        .iter()
        .find(|(id, _)| *id == fdid)
        .expect("metal fdid")
        .1;
    let size = 512u32;
    let mut pixels = Vec::with_capacity((size * size * 4) as usize);
    for y in 0..size {
        for x in 0..size {
            pixels.extend_from_slice(&[tag, x as u8, y as u8, 255]);
        }
    }
    Ok((pixels, size))
}

fn pixel(sheet: &[u8], x: u32, y: u32) -> [u8; 4] {
    let i = ((y * METAL_SHEET.0 + x) * 4) as usize;
    sheet[i..i + 4].try_into().unwrap()
}

#[test]
fn uv_rects_resolve_the_three_by_three_metal_grid() {
    let uv = metal_frame_uv_rects();
    let (w, h) = (364.0, 246.0);
    // TL portrait corner 150×150, top edge 64 wide, bottom row 64 tall.
    assert_eq!(uv[0], [0.0, 150.0 / w, 0.0, 150.0 / h]);
    assert_eq!(uv[1], [150.0 / w, 214.0 / w, 0.0, 150.0 / h]);
    assert_eq!(uv[4], [150.0 / w, 214.0 / w, 150.0 / h, 182.0 / h]);
    assert_eq!(uv[8], [214.0 / w, 1.0, 182.0 / h, 1.0]);
    let style = metal_frame_style(Handle::default());
    assert_eq!(style.edge_sizes, Some([75.0, 75.0, 75.0, 32.0]));
    assert_eq!(style.uv_rects, Some(uv));
}

#[test]
fn composed_sheet_takes_each_cell_from_its_retail_atlas_member() {
    let sheet = compose_metal_sheet(tagged_source).unwrap();
    // TL cell (10, 20) ← ui-frame-portraitmetal-cornertopleft-2x at (1, 153).
    assert_eq!(pixel(&sheet, 10, 20), [1, 11, 173, 255]);
    // Top edge (160, 5) ← _ui-frame-metal-edgetop-2x at (0, 1).
    assert_eq!(pixel(&sheet, 160, 5), [3, 10, 6, 255]);
    // Right edge (220, 160) ← !ui-frame-metal-edgeright-2x at (153, 0).
    assert_eq!(pixel(&sheet, 220, 160), [2, 159, 10, 255]);
    // Bottom-left corner (5, 190) ← cornerbottomleft-2x at (153, 153).
    assert_eq!(pixel(&sheet, 5, 190), [1, 158, 161, 255]);
    // The bottom-left cell continues with the tiled 32-px bottom edge.
    assert_eq!(pixel(&sheet, 100, 190), [3, (100 - 64) % 32, 161, 255]);
    // Bottom-right corner (310, 190) ← cornerbottomright-2x at (219, 153).
    assert_eq!(pixel(&sheet, 310, 190), [1, 229, 161, 255]);
    // The centre stays transparent: the window draws its own background.
    assert_eq!(pixel(&sheet, 180, 160), [0, 0, 0, 0]);
}

#[test]
fn real_metal_atlases_compose_with_opaque_border_art() {
    let sheet = compose_metal_sheet(load_blp_pixels).expect("metal frame atlases from CASC");
    let opaque_in = |x0: u32, y0: u32, w: u32, h: u32| {
        (y0..y0 + h)
            .flat_map(|y| (x0..x0 + w).map(move |x| (x, y)))
            .filter(|&(x, y)| pixel(&sheet, x, y)[3] > 200)
            .count()
    };
    assert!(opaque_in(0, 0, 150, 150) > 1000, "portrait ring corner");
    assert!(opaque_in(0, 150, 150, 32) > 50, "left edge line");
    assert!(opaque_in(150, 182, 64, 64) > 100, "bottom edge");
    assert_eq!(opaque_in(150, 150, 64, 32), 0, "transparent centre");
}
