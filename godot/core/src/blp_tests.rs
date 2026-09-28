use std::path::Path;

use super::*;

fn texture(fdid: u32) -> Vec<u8> {
    let path = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../data/textures")
        .join(format!("{fdid}.blp"));
    std::fs::read(&path).unwrap_or_else(|error| panic!("{}: {error}", path.display()))
}

fn compressed(fdid: u32) -> CompressedImage {
    match decode_gpu(&texture(fdid)).unwrap() {
        GpuImage::Compressed(image) => image,
        GpuImage::Rgba(_) => panic!("{fdid} decoded to RGBA8"),
    }
}

/// Stormwind crenellation 464811: a 256x256 DXT1 BLP with its full mip chain.
#[test]
fn a_dxt1_blp_stays_block_compressed_with_every_mip_level() {
    let image = compressed(464811);

    assert_eq!(image.format, BlockFormat::Dxt1);
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

    assert_eq!(image.format, BlockFormat::Dxt5);
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

    assert_eq!(image.format, BlockFormat::Dxt5);
    assert_eq!(
        &image.data[..dxtn.images[0].content.len()],
        &dxtn.images[0].content[..]
    );
}

/// 1022933 is palettized: there are no blocks to upload, so it decodes to RGBA8.
#[test]
fn a_palettized_blp_decodes_to_rgba() {
    let GpuImage::Rgba(image) = decode_gpu(&texture(1022933)).unwrap() else {
        panic!("1022933 is palettized");
    };

    assert_eq!(
        image.pixels.len(),
        (image.width * image.height * 4) as usize
    );
}
