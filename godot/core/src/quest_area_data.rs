//! Quest objective areas (`QuestPOI` blobs) as an RGBA overlay for the minimap composite
//! and the world map canvas. Retail draws them with the `UI-QuestBlob-Inside` fill and
//! `UI-QuestBlob-Outside` rim art; this approximates both as a gold fill with a
//! brighter rim one and a half pixels wide.

/// Overlay colour (`QuestBlob` gold).
pub const QUEST_AREA_COLOR: [u8; 3] = [255, 209, 0];
const FILL_ALPHA: f32 = 0.25;
const RIM_ALPHA: f32 = 0.7;
const RIM_PX: f32 = 1.5;

/// `width`×`height` RGBA overlay of `polygons` (pixel coordinates, three or more
/// points each): transparent outside, gold fill inside, a stronger gold rim.
pub fn quest_area_overlay(width: u32, height: u32, polygons: &[Vec<[f32; 2]>]) -> Vec<u8> {
    let mut overlay = vec![0u8; (width * height * 4) as usize];
    for polygon in polygons.iter().filter(|polygon| polygon.len() >= 3) {
        let (min, max) = bounds(polygon);
        let rows = (min[1].floor().max(0.0) as u32)..(max[1].ceil().min(height as f32) as u32);
        for py in rows {
            let columns =
                (min[0].floor().max(0.0) as u32)..(max[0].ceil().min(width as f32) as u32);
            for px in columns {
                let point = [px as f32 + 0.5, py as f32 + 0.5];
                if !inside(polygon, point) {
                    continue;
                }
                let alpha = if edge_distance(polygon, point) <= RIM_PX {
                    RIM_ALPHA
                } else {
                    FILL_ALPHA
                };
                let offset = ((py * width + px) * 4) as usize;
                overlay[offset..offset + 3].copy_from_slice(&QUEST_AREA_COLOR);
                let alpha = (alpha * 255.0).round() as u8;
                overlay[offset + 3] = overlay[offset + 3].max(alpha);
            }
        }
    }
    overlay
}

/// Blends `overlay` into the opaque pixels of `base` (same size); returns how many
/// pixels it changed.
pub fn blend_overlay(base: &mut [u8], overlay: &[u8]) -> usize {
    let mut blended = 0;
    for (pixel, over) in base.chunks_exact_mut(4).zip(overlay.chunks_exact(4)) {
        if over[3] == 0 || pixel[3] == 0 {
            continue;
        }
        let strength = f32::from(over[3]) / 255.0;
        for channel in 0..3 {
            let value = f32::from(pixel[channel]);
            let target = f32::from(over[channel]);
            pixel[channel] = (value + (target - value) * strength).round() as u8;
        }
        blended += 1;
    }
    blended
}

fn bounds(polygon: &[[f32; 2]]) -> ([f32; 2], [f32; 2]) {
    polygon.iter().fold(
        ([f32::INFINITY; 2], [f32::NEG_INFINITY; 2]),
        |(min, max), &[x, y]| {
            (
                [min[0].min(x), min[1].min(y)],
                [max[0].max(x), max[1].max(y)],
            )
        },
    )
}

/// Even-odd point-in-polygon test.
fn inside(polygon: &[[f32; 2]], [x, y]: [f32; 2]) -> bool {
    let mut inside = false;
    let mut previous = polygon[polygon.len() - 1];
    for &[px, py] in polygon {
        let [qx, qy] = previous;
        if (py > y) != (qy > y) && x < (qx - px) * (y - py) / (qy - py) + px {
            inside = !inside;
        }
        previous = [px, py];
    }
    inside
}

/// Distance from `point` to the nearest polygon edge.
fn edge_distance(polygon: &[[f32; 2]], [x, y]: [f32; 2]) -> f32 {
    let mut previous = polygon[polygon.len() - 1];
    let mut nearest = f32::INFINITY;
    for &[px, py] in polygon {
        let [qx, qy] = previous;
        let (dx, dy) = (px - qx, py - qy);
        let length = dx * dx + dy * dy;
        let t = if length == 0.0 {
            0.0
        } else {
            (((x - qx) * dx + (y - qy) * dy) / length).clamp(0.0, 1.0)
        };
        nearest = nearest.min((x - qx - t * dx).hypot(y - qy - t * dy));
        previous = [px, py];
    }
    nearest
}

#[cfg(test)]
mod tests {
    use super::*;

    fn alpha(overlay: &[u8], width: u32, x: u32, y: u32) -> u8 {
        overlay[((y * width + x) * 4 + 3) as usize]
    }

    #[test]
    fn overlay_fills_inside_with_a_stronger_rim_and_leaves_outside_clear() {
        let square = vec![[10.0, 10.0], [30.0, 10.0], [30.0, 30.0], [10.0, 30.0]];
        let overlay = quest_area_overlay(40, 40, &[square]);
        assert_eq!(alpha(&overlay, 40, 5, 5), 0, "outside");
        assert_eq!(alpha(&overlay, 40, 20, 20), 64, "fill");
        assert_eq!(alpha(&overlay, 40, 10, 20), 179, "rim");
        let mut base = vec![100u8; 40 * 40 * 4];
        let blended = blend_overlay(&mut base, &overlay);
        assert_eq!(blended, 20 * 20);
        let centre = ((20 * 40 + 20) * 4) as usize;
        assert_eq!(&base[centre..centre + 4], &[139, 127, 75, 100]);
    }
}
