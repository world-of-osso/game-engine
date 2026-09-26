//! Shared BLP decode quirks for mipmap headers and 1-bit alpha.

pub fn strip_mipmaps(bytes: &mut [u8]) {
    if bytes.starts_with(b"BLP2") {
        if let Some(has_mipmaps) = bytes.get_mut(11) {
            *has_mipmaps = 0;
        }
        return;
    }
    if (bytes.starts_with(b"BLP0") || bytes.starts_with(b"BLP1")) && bytes.len() >= 28 {
        bytes[24..28].copy_from_slice(&0u32.to_le_bytes());
    }
}

pub fn fix_1bit_alpha(pixels: &mut [u8]) {
    let max_alpha = pixels.iter().skip(3).step_by(4).copied().max().unwrap_or(0);
    if max_alpha == 0 {
        // No alpha channel — set all pixels fully opaque.
        for alpha in pixels.iter_mut().skip(3).step_by(4) {
            *alpha = 255;
        }
    } else if max_alpha == 1 {
        // 1-bit alpha — expand 1 → 255.
        for alpha in pixels.iter_mut().skip(3).step_by(4) {
            if *alpha > 0 {
                *alpha = 255;
            }
        }
    }
}
