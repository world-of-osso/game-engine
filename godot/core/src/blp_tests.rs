use std::path::Path;

use super::*;

fn texture(fdid: u32) -> Vec<u8> {
    let path = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../data/textures")
        .join(format!("{fdid}.blp"));
    std::fs::read(&path).unwrap_or_else(|error| panic!("{}: {error}", path.display()))
}

fn compressed(fdid: u32) -> GpuImage {
    let image = decode_gpu(&texture(fdid)).unwrap();
    assert_ne!(image.format, GpuFormat::Rgba8, "{fdid} decoded to RGBA8");
    image
}

/// Stormwind crenellation 464811: a 256x256 DXT1 BLP with its full mip chain.
#[test]
fn a_dxt1_blp_stays_block_compressed_with_every_mip_level() {
    let image = compressed(464811);

    assert_eq!(image.format, GpuFormat::Dxt1);
    assert_eq!((image.width, image.height), (256, 256));
    assert!(image.mipmaps);
    // 256..4: 64x64..1x1 blocks of 8 bytes, then 2x2 and 1x1 still take one block each.
    let expected: usize = [4096, 1024, 256, 64, 16, 4, 1, 1, 1].iter().sum::<usize>() * 8;
    assert_eq!(image.data.len(), expected);
    let rgba = decode_rgba(&texture(464811)).unwrap();
    assert_eq!(rgba.pixels.len(), 256 * 256 * 4);
    // With its whole mip chain it still takes under a quarter of mip 0 as RGBA8.
    assert!(image.data.len() * 4 < rgba.pixels.len());
}

/// 464043: a DXT5 BLP whose alpha is zero everywhere; `decode_rgba` makes it opaque, so
/// the uploaded blocks must too.
#[test]
fn a_zero_alpha_dxt5_blp_uploads_opaque_like_the_rgba_decode() {
    let rgba = decode_rgba(&texture(464043)).unwrap();
    assert!(
        rgba.pixels
            .iter()
            .skip(3)
            .step_by(4)
            .all(|&alpha| alpha == 255)
    );

    let image = compressed(464043);

    assert_eq!(image.format, GpuFormat::Dxt5);
    assert!(
        image
            .data
            .chunks_exact(16)
            .all(|block| block[..8] == [255, 255, 0, 0, 0, 0, 0, 0])
    );
}

/// Torch 198077: a 32x32 DXT5 BLP with authored alpha, kept as authored.
#[test]
fn authored_dxt5_alpha_is_kept() {
    let bytes = texture(198077);
    let blp = load_blp_from_buf(&bytes).unwrap();
    let BlpContent::Dxt5(dxtn) = &blp.content else {
        panic!("198077 is DXT5");
    };

    let image = compressed(198077);

    assert_eq!(image.format, GpuFormat::Dxt5);
    assert_eq!(
        &image.data[..dxtn.images[0].content.len()],
        &dxtn.images[0].content[..]
    );
}

/// 1022933 is palettized: there are no blocks to upload, so it decodes to RGBA8.
#[test]
fn a_palettized_blp_decodes_to_rgba() {
    let image = decode_gpu(&texture(1022933)).unwrap();

    assert_eq!(image.format, GpuFormat::Rgba8);
    assert_eq!(image.data.len(), (image.width * image.height * 4) as usize);
}

/// Elwynn bush 189700's leaf cards, 189937: a 128x128 DXT1 BLP with 1-bit alpha and its
/// full chain. Godot uploads `FORMAT_DXT1` as BC1 RGB, where the punch-through texels are
/// opaque black, so it decodes every level to RGBA8 and keeps them transparent.
#[test]
fn a_one_bit_alpha_dxt1_blp_decodes_every_level_with_its_transparent_texels() {
    let image = decode_gpu(&texture(189937)).unwrap();

    assert_eq!(image.format, GpuFormat::Rgba8);
    assert_eq!((image.width, image.height), (128, 128));
    assert!(image.mipmaps);
    let levels: usize = (0..8)
        .map(|level| (128 >> level) * (128 >> level) * 4)
        .sum();
    assert_eq!(image.data.len(), levels);
    let rgba = decode_rgba(&texture(189937)).unwrap();
    assert_eq!(&image.data[..rgba.pixels.len()], &rgba.pixels[..]);
    let transparent = rgba.pixels.iter().skip(3).step_by(4).filter(|&&a| a == 0);
    assert!(transparent.count() > 128 * 128 / 4);
}
