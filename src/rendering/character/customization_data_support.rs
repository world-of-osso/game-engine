use crate::asset::{asset_cache::texture, blp::load_blp_rgba};

pub(super) fn sample_swatch_color(materials: &[(u16, u32)]) -> Option<[u8; 3]> {
    let &(_, fdid) = materials.first()?;
    let path = texture(fdid)?;
    let (rgba, w, h) = load_blp_rgba(&path).ok()?;
    let cx = w / 2;
    let cy = h / 2;
    let idx = ((cy * w + cx) * 4) as usize;
    (idx + 2 < rgba.len()).then_some([rgba[idx], rgba[idx + 1], rgba[idx + 2]])
}
