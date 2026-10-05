//! Engine-free unit selection rules shared by the Bevy and Godot clients.

/// Tab targeting: the target after `current` in the nearest-first `sorted` list,
/// wrapping; the nearest when nothing (or an unlisted unit) is targeted. With no
/// candidate, Retail's `TargetNearestEnemy` finds nothing and the target stays.
pub fn next_target<T: Copy + PartialEq>(sorted: &[T], current: Option<T>) -> Option<T> {
    let Some(&first) = sorted.first() else {
        return current;
    };
    let Some(current) = current else {
        return Some(first);
    };
    match sorted.iter().position(|&unit| unit == current) {
        Some(index) => Some(sorted[(index + 1) % sorted.len()]),
        None => Some(first),
    }
}

/// Shift-Tab, Retail `TARGETPREVIOUSENEMY` (`TargetNearestEnemy(true)`, "true means
/// reverse", Bindings_Standard.xml:998): Tab's cycle walked backwards, so it starts
/// at the farthest unit.
pub fn previous_target<T: Copy + PartialEq>(sorted: &[T], current: Option<T>) -> Option<T> {
    let reversed: Vec<T> = sorted.iter().rev().copied().collect();
    next_target(&reversed, current)
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
