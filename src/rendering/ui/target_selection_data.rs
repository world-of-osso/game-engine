//! Engine-free unit selection rules shared by the Bevy and Godot clients.

/// Tab targeting: the target after `current` in the nearest-first `sorted` list,
/// wrapping; the nearest when nothing (or an unlisted unit) is targeted.
pub fn next_target<T: Copy + PartialEq>(sorted: &[T], current: Option<T>) -> Option<T> {
    let first = *sorted.first()?;
    let Some(current) = current else {
        return Some(first);
    };
    match sorted.iter().position(|&unit| unit == current) {
        Some(index) => Some(sorted[(index + 1) % sorted.len()]),
        None => Some(first),
    }
}

/// One surface the pick ray through the cursor crosses, nearest first.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PickHit<T> {
    /// A unit's pick shape; `visible` is false for a hidden unit.
    Unit { id: T, visible: bool },
    /// World geometry (terrain, WMO); `visible` is false for culled geometry.
    World { visible: bool },
}

/// The unit a click selects: the nearest visible surface on the ray, when it is a
/// unit. Visible world geometry in front occludes every unit behind it; hidden
/// units and culled geometry let the ray through. Hits after the first visible
/// one are not consumed.
pub fn first_picked_unit<T>(hits: impl IntoIterator<Item = PickHit<T>>) -> Option<T> {
    for hit in hits {
        match hit {
            PickHit::Unit { id, visible: true } => return Some(id),
            PickHit::World { visible: true } => return None,
            PickHit::Unit { visible: false, .. } | PickHit::World { visible: false } => {}
        }
    }
    None
}

/// Selection-ring textures without authored alpha draw their intensity as alpha,
/// so the ring's black background is transparent. RGBA8 pixels.
pub fn opaque_to_alpha_mask(rgba: &mut [u8]) {
    if !rgba.chunks_exact(4).all(|pixel| pixel[3] == 255) {
        return;
    }
    for pixel in rgba.chunks_exact_mut(4) {
        let intensity = pixel[0].max(pixel[1]).max(pixel[2]);
        pixel.copy_from_slice(&[intensity; 4]);
    }
}
