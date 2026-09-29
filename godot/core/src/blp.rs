//! Decode authored BLP bytes: mip 0 RGBA8 pixels, or the block-compressed data as authored.
use crate::asset::blp_format::{fix_1bit_alpha, strip_mipmaps};
use image_blp::{
    convert::blp_to_image,
    parser::load_blp_from_buf,
    types::{BlpContent, BlpDxtn},
};

pub struct RgbaImage {
    pub pixels: Vec<u8>,
    pub width: u32,
    pub height: u32,
}

pub fn decode_rgba(bytes: &[u8]) -> Result<RgbaImage, String> {
    let mut bytes = bytes.to_vec();
    strip_mipmaps(&mut bytes);
    let blp = load_blp_from_buf(&bytes).map_err(|e| format!("Failed to load BLP: {e}"))?;
    let rgba = blp_to_image(&blp, 0)
        .map_err(|e| format!("Failed to convert BLP: {e}"))?
        .to_rgba8();
    let width = rgba.width();
    let height = rgba.height();
    let mut pixels = rgba.into_raw();
    fix_1bit_alpha(&mut pixels);
    Ok(RgbaImage {
        pixels,
        width,
        height,
    })
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum GpuFormat {
    Dxt1,
    Dxt3,
    Dxt5,
    Rgba8,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum BlockFormat {
    Dxt1,
    Dxt3,
    Dxt5,
}

impl BlockFormat {
    fn block_bytes(self) -> usize {
        match self {
            Self::Dxt1 => 8,
            Self::Dxt3 | Self::Dxt5 => 16,
        }
    }

    fn gpu_format(self) -> GpuFormat {
        match self {
            Self::Dxt1 => GpuFormat::Dxt1,
            Self::Dxt3 => GpuFormat::Dxt3,
            Self::Dxt5 => GpuFormat::Dxt5,
        }
    }
}

/// Texture data ready for upload: mip 0, followed by every smaller level down to 1x1
/// when `mipmaps` is set.
pub struct GpuImage {
    pub format: GpuFormat,
    pub width: u32,
    pub height: u32,
    pub mipmaps: bool,
    pub data: Vec<u8>,
}

/// Keep DXT BLPs compressed as authored (4-8x smaller than RGBA8); other encodings decode
/// to RGBA8. Alpha matches `decode_rgba`: DXT3/DXT5 alpha that is zero everywhere is opaque.
///
/// DXT1 with alpha bits decodes every authored level to RGBA8. WebWowViewerCpp uploads it
/// as BC1 RGBA (`BlpTexture.cpp` `getTextureType`, `GBlpTextureVLK.cpp`), but Godot has no
/// such image format: it uploads `FORMAT_DXT1` as `DATA_FORMAT_BC1_RGB_UNORM_BLOCK`
/// (4.7.2 `texture_storage.cpp`), where punch-through texels are opaque black.
pub fn decode_gpu(bytes: &[u8]) -> Result<GpuImage, String> {
    let blp = load_blp_from_buf(bytes).map_err(|e| format!("Failed to load BLP: {e}"))?;
    let (width, height) = (blp.header.width, blp.header.height);
    let (format, dxtn) = match &blp.content {
        BlpContent::Dxt1(dxtn) => (BlockFormat::Dxt1, dxtn),
        BlpContent::Dxt3(dxtn) => (BlockFormat::Dxt3, dxtn),
        BlpContent::Dxt5(dxtn) => (BlockFormat::Dxt5, dxtn),
        _ => {
            let rgba = decode_rgba(bytes)?;
            return Ok(GpuImage {
                format: GpuFormat::Rgba8,
                width: rgba.width,
                height: rgba.height,
                mipmaps: false,
                data: rgba.pixels,
            });
        }
    };
    let image = compressed_image(format, dxtn, width, height)?;
    if format == BlockFormat::Dxt1 && blp.header.alpha_bits() > 0 {
        return Ok(decompress_bc1(image));
    }
    Ok(image)
}

fn compressed_image(
    format: BlockFormat,
    dxtn: &BlpDxtn,
    width: u32,
    height: u32,
) -> Result<GpuImage, String> {
    let first = dxtn.images.first().ok_or("BLP DXT has no mipmap level 0")?;
    let (width, height) = level_zero_size(width, height, first.content.len(), format);
    let levels = mip_level_sizes(width, height, format);
    let complete = dxtn.images.len() >= levels.len()
        && levels
            .iter()
            .zip(&dxtn.images)
            .all(|(size, image)| image.content.len() >= *size);
    let data = if complete {
        levels
            .iter()
            .zip(&dxtn.images)
            .flat_map(|(size, image)| image.content[..*size].iter().copied())
            .collect()
    } else {
        first.content[..levels[0]].to_vec()
    };
    let mut image = GpuImage {
        format: format.gpu_format(),
        width,
        height,
        mipmaps: complete,
        data,
    };
    make_zero_alpha_opaque(&mut image);
    Ok(image)
}

/// BC1 blocks of every level, decoded to RGBA8 with punch-through texels transparent.
fn decompress_bc1(image: GpuImage) -> GpuImage {
    let mut levels = mip_dimensions(image.width, image.height);
    if !image.mipmaps {
        levels.truncate(1);
    }
    let mut blocks = image.data.as_slice();
    let mut data = Vec::new();
    for (width, height) in levels {
        let (level, rest) = blocks.split_at(level_bytes(width, height, BlockFormat::Dxt1));
        let start = data.len();
        data.resize(start + (width * height * 4) as usize, 0);
        texpresso::Format::Bc1.decompress(
            level,
            width as usize,
            height as usize,
            &mut data[start..],
        );
        blocks = rest;
    }
    GpuImage {
        format: GpuFormat::Rgba8,
        data,
        ..image
    }
}

/// Some BLPs have truncated mip 0: the header says 128x128 but the data only fits a
/// smaller level. Take the largest level the data holds.
fn level_zero_size(width: u32, height: u32, len: usize, format: BlockFormat) -> (u32, u32) {
    let (mut w, mut h) = (width, height);
    while level_bytes(w, h, format) > len && w > 4 && h > 4 {
        w /= 2;
        h /= 2;
    }
    (w, h)
}

fn level_bytes(width: u32, height: u32, format: BlockFormat) -> usize {
    let blocks = |side: u32| side.div_ceil(4).max(1) as usize;
    blocks(width) * blocks(height) * format.block_bytes()
}

/// Size of every mip level from `width`x`height` down to 1x1.
fn mip_dimensions(width: u32, height: u32) -> Vec<(u32, u32)> {
    let (mut w, mut h) = (width, height);
    let mut levels = vec![(w, h)];
    while w > 1 || h > 1 {
        w = (w / 2).max(1);
        h = (h / 2).max(1);
        levels.push((w, h));
    }
    levels
}

/// Byte size of every mip level from `width`x`height` down to 1x1.
fn mip_level_sizes(width: u32, height: u32, format: BlockFormat) -> Vec<usize> {
    mip_dimensions(width, height)
        .into_iter()
        .map(|(w, h)| level_bytes(w, h, format))
        .collect()
}

/// `decode_rgba` turns alpha that is zero everywhere into opaque (`fix_1bit_alpha`);
/// uploaded blocks must decode the same. Only DXT3/DXT5 carry an alpha block.
fn make_zero_alpha_opaque(image: &mut GpuImage) {
    let mut blocks = image.data.chunks_exact(16);
    let zero = match image.format {
        GpuFormat::Dxt1 | GpuFormat::Rgba8 => return,
        GpuFormat::Dxt3 => blocks.all(|block| block[..8].iter().all(|&a| a == 0)),
        GpuFormat::Dxt5 => blocks.all(bc3_alpha_is_zero),
    };
    if !zero {
        return;
    }
    // DXT3: explicit 4-bit alpha 0xF. DXT5: endpoints 255/255, every index 0 -> 255.
    let opaque: [u8; 8] = match image.format {
        GpuFormat::Dxt3 => [0xFF; 8],
        _ => [255, 255, 0, 0, 0, 0, 0, 0],
    };
    for block in image.data.chunks_exact_mut(16) {
        block[..8].copy_from_slice(&opaque);
    }
}

/// Every texel of a DXT5 alpha block decodes to 0.
fn bc3_alpha_is_zero(block: &[u8]) -> bool {
    let (a0, a1) = (u32::from(block[0]), u32::from(block[1]));
    let mut palette = [a0, a1, 0, 0, 0, 0, 0, 0];
    if a0 > a1 {
        for i in 1..7 {
            palette[i + 1] = ((7 - i as u32) * a0 + i as u32 * a1) / 7;
        }
    } else {
        for i in 1..5 {
            palette[i + 1] = ((5 - i as u32) * a0 + i as u32 * a1) / 5;
        }
        palette[7] = 255;
    }
    let mut indices = [0u8; 8];
    indices[..6].copy_from_slice(&block[2..8]);
    let bits = u64::from_le_bytes(indices);
    (0..16).all(|texel| palette[((bits >> (3 * texel)) & 7) as usize] == 0)
}

#[cfg(test)]
#[path = "blp_tests.rs"]
mod tests;
