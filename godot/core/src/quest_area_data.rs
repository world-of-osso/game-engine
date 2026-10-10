//! Authored quest polygons rasterized with extracted Retail blue fill and rim art.
use crate::minimap_data::{TileImage, sample};
use std::path::Path;

const RIM_PX: f32 = 3.0;
// QuestBlobDataProvider.lua OnLoad; selection does not change these values.
const FILL_ALPHA: u16 = 128;
const BORDER_ALPHA: u16 = 192;

pub struct QuestAreaArt {
    fill: TileImage,
    rim: TileImage,
    selected_rim: TileImage,
}
impl QuestAreaArt {
    pub fn load(root: &Path, minimap: bool) -> Result<Self, String> {
        let read = |id| {
            let path = root.join("textures").join(format!("{id}.blp"));
            let bytes = std::fs::read(&path).map_err(|e| format!("{}: {e}", path.display()))?;
            let image = crate::blp::decode_rgba(&bytes)?;
            Ok::<_, String>(TileImage {
                pixels: image.pixels,
                width: image.width,
                height: image.height,
            })
        };
        Ok(Self {
            fill: read(342_529)?,
            rim: read(if minimap { 533_895 } else { 342_531 })?,
            selected_rim: read(if minimap { 1_083_696 } else { 342_531 })?,
        })
    }
}

/// Transparent RGBA overlay. Shapes come only from POI points; textures never
/// substitute a missing polygon. Selection uses authored minimap OutsideSelected
/// art only; the world map has no separate selected art. Opacity stays unchanged.
pub fn quest_area_overlay(
    width: u32,
    height: u32,
    polygons: &[Vec<[f32; 2]>],
    art: &QuestAreaArt,
    highlighted: bool,
) -> Vec<u8> {
    let mut overlay = vec![0u8; (width * height * 4) as usize];
    for polygon in polygons.iter().filter(|polygon| polygon.len() >= 3) {
        let (min, max) = bounds(polygon);
        let rows = ((min[1] - RIM_PX).floor().max(0.0) as u32)
            ..((max[1] + RIM_PX).ceil().min(height as f32) as u32);
        for py in rows {
            let columns = ((min[0] - RIM_PX).floor().max(0.0) as u32)
                ..((max[0] + RIM_PX).ceil().min(width as f32) as u32);
            for px in columns {
                let point = [px as f32 + 0.5, py as f32 + 0.5];
                let color = polygon_pixel(polygon, point, art, highlighted);
                let offset = ((py * width + px) * 4) as usize;
                if color[3] > overlay[offset + 3] {
                    overlay[offset..offset + 4].copy_from_slice(&color);
                }
            }
        }
    }
    overlay
}

fn polygon_pixel(
    polygon: &[[f32; 2]],
    point: [f32; 2],
    art: &QuestAreaArt,
    highlighted: bool,
) -> [u8; 4] {
    let mut color = [0; 4];
    if inside(polygon, point) {
        let [x, y] = point;
        color = sample(
            &art.fill,
            [
                (x.floor() as u32 % art.fill.width) as f32 / art.fill.width as f32,
                (y.floor() as u32 % art.fill.height) as f32 / art.fill.height as f32,
            ],
        );
        color[3] = (u16::from(color[3]) * FILL_ALPHA / 255) as u8;
    }
    let distance = edge_distance(polygon, point);
    if distance <= RIM_PX {
        let rim = if highlighted {
            &art.selected_rim
        } else {
            &art.rim
        };
        let mut border = sample(rim, [0.5, distance / RIM_PX]);
        border[3] = (u16::from(border[3]) * BORDER_ALPHA / 255) as u8;
        color = over(color, border);
    }
    color
}

fn over(base: [u8; 4], top: [u8; 4]) -> [u8; 4] {
    let a = f32::from(top[3]) / 255.0;
    let b = f32::from(base[3]) / 255.0 * (1.0 - a);
    let alpha = a + b;
    if alpha == 0.0 {
        return [0; 4];
    }
    let mut result = [0; 4];
    for c in 0..3 {
        result[c] = ((f32::from(top[c]) * a + f32::from(base[c]) * b) / alpha).round() as u8;
    }
    result[3] = (alpha * 255.0).round() as u8;
    result
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

    #[test]
    fn retail_objective_fill_is_blue() {
        let overlay = quest_area_overlay(
            40,
            40,
            &[vec![[10.0, 10.0], [30.0, 10.0], [30.0, 30.0], [10.0, 30.0]]],
            &art(),
            false,
        );
        let offset = ((20 * 40 + 20) * 4) as usize;
        assert!(
            overlay[offset + 2] > overlay[offset],
            "objective areas must use blue Retail fill"
        );
    }

    fn art() -> QuestAreaArt {
        QuestAreaArt::load(
            &Path::new(env!("CARGO_MANIFEST_DIR")).join("../../data"),
            false,
        )
        .unwrap()
    }

    fn alpha(overlay: &[u8], width: u32, x: u32, y: u32) -> u8 {
        overlay[((y * width + x) * 4 + 3) as usize]
    }

    #[test]
    fn opaque_art_uses_retail_fill_and_border_alpha_for_both_selection_states() {
        let opaque = || TileImage {
            pixels: vec![255; 4],
            width: 1,
            height: 1,
        };
        let art = QuestAreaArt {
            fill: opaque(),
            rim: opaque(),
            selected_rim: opaque(),
        };
        let polygon = [[10.0, 10.0], [30.0, 10.0], [30.0, 30.0], [10.0, 30.0]];
        for selected in [false, true] {
            assert_eq!(
                polygon_pixel(&polygon, [20.0, 20.0], &art, selected)[3],
                128
            );
            assert_eq!(polygon_pixel(&polygon, [9.5, 20.0], &art, selected)[3], 192);
        }
    }

    #[test]
    fn overlay_fills_inside_with_a_stronger_rim_and_leaves_outside_clear() {
        let square = vec![[10.0, 10.0], [30.0, 10.0], [30.0, 30.0], [10.0, 30.0]];
        let polygons = [square];
        let overlay = quest_area_overlay(40, 40, &polygons, &art(), false);
        let selected = quest_area_overlay(40, 40, &polygons, &art(), true);
        assert_eq!(alpha(&selected, 40, 20, 20), 128, "selected fill alpha");
        assert_eq!(
            selected, overlay,
            "world-map selection changes no opacity or art"
        );
        assert_eq!(alpha(&overlay, 40, 5, 5), 0, "outside");
        assert_eq!(alpha(&overlay, 40, 20, 20), 128, "Retail fill alpha");
        assert!(alpha(&overlay, 40, 10, 20) > 128, "rim over fill");
        let mut base = vec![100u8; 40 * 40 * 4];
        let blended = blend_overlay(&mut base, &overlay);
        assert!(
            blended > 20 * 20,
            "authored rim extends outside the exact polygon"
        );
        let centre = ((20 * 40 + 20) * 4) as usize;
        assert!(base[centre + 2] > base[centre]);
        assert_eq!(base[centre + 3], 100, "base mask preserved");
    }
}
