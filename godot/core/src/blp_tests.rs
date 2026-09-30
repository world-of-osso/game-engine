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

fn bc5_blp(width: u32, height: u32, blocks: &[u8]) -> Vec<u8> {
    let mut bytes = vec![0; 148];
    bytes[..4].copy_from_slice(b"BLP2");
    bytes[4..8].copy_from_slice(&1u32.to_le_bytes());
    bytes[8] = 2;
    bytes[10] = 11;
    bytes[12..16].copy_from_slice(&width.to_le_bytes());
    bytes[16..20].copy_from_slice(&height.to_le_bytes());
    bytes[20..24].copy_from_slice(&148u32.to_le_bytes());
    bytes[84..88].copy_from_slice(&(blocks.len() as u32).to_le_bytes());
    bytes.extend_from_slice(blocks);
    bytes
}

/// Two BC4 channels with indices 0..7 twice, independently known UNORM palettes.
#[test]
fn bc5_decodes_both_endpoint_orders_and_all_indices() {
    // 48 little-endian index bits encode [0,1,2,3,4,5,6,7] twice.
    let indices = [0x88, 0xc6, 0xfa, 0x88, 0xc6, 0xfa];
    let mut block = vec![240, 30];
    block.extend_from_slice(&indices);
    block.extend_from_slice(&[20, 220]);
    block.extend_from_slice(&indices);
    let image = decode_rgba(&bc5_blp(4, 4, &block)).unwrap();
    let red = [240, 30, 210, 180, 150, 120, 90, 60];
    let green = [20, 220, 60, 100, 140, 180, 0, 255];
    assert_eq!((image.width, image.height), (4, 4));
    let expected: Vec<u8> = (0..16)
        .flat_map(|i| [red[i % 8], green[i % 8], 0, 255])
        .collect();
    assert_eq!(image.pixels, expected);
}

#[test]
fn bc5_clips_partial_blocks_and_uses_authored_mip_offset() {
    let mut blocks = vec![0; 32];
    blocks[0] = 64;
    blocks[8] = 128;
    blocks[16] = 192;
    blocks[24] = 32;
    let mut bytes = bc5_blp(5, 3, &blocks);
    bytes.splice(148..148, [99; 12]);
    bytes[20..24].copy_from_slice(&160u32.to_le_bytes());
    let image = decode_rgba(&bytes).unwrap();
    let expected: Vec<u8> = (0..3)
        .flat_map(|_| {
            (0..5).flat_map(|x| {
                if x < 4 {
                    [64, 128, 0, 255]
                } else {
                    [192, 32, 0, 255]
                }
            })
        })
        .collect();
    assert_eq!((image.width, image.height), (5, 3));
    assert_eq!(image.pixels, expected);
}

#[test]
fn bc5_rejects_incomplete_header_or_mip_payload() {
    let bytes = bc5_blp(4, 4, &[0; 16]);
    assert!(decode_rgba(&bytes[..100]).is_err());
    assert!(decode_rgba(&bytes[..163]).is_err());
    assert!(decode_rgba(&bc5_blp(5, 4, &[0; 16])).is_err());
    let mut overlap = bytes;
    overlap[20..24].copy_from_slice(&20u32.to_le_bytes());
    assert!(
        decode_rgba(&overlap).is_err(),
        "mip must not read header bytes"
    );
}

#[test]
fn bc5_rejects_zero_and_overflowing_dimensions() {
    assert!(decode_rgba(&bc5_blp(0, 4, &[])).is_err());
    assert!(decode_rgba(&bc5_blp(4, 0, &[])).is_err());
    assert!(decode_rgba(&bc5_blp(u32::MAX, u32::MAX, &[])).is_err());
}

/// Azerite liquid normal map FDID 1886758 is BLP2 pixel format 11 (BC5).
#[test]
fn bc5_normal_map_decodes_red_green_channels() {
    let image = decode_rgba(&texture(1_886_758)).unwrap();
    assert_eq!((image.width, image.height), (1024, 1024));
    let texels: Vec<_> = image.pixels.chunks_exact(4).collect();
    assert!(texels.iter().all(|texel| texel[2] == 0 && texel[3] == 255));
    let mean = |channel: usize| {
        texels
            .iter()
            .map(|texel| f64::from(texel[channel]))
            .sum::<f64>()
            / texels.len() as f64
    };
    // Tangent-space normal XY centre near 0.5.
    assert!((100.0..156.0).contains(&mean(0)), "red mean {}", mean(0));
    assert!((100.0..156.0).contains(&mean(1)), "green mean {}", mean(1));
    assert!(texels.iter().any(|texel| texel[0] != texels[0][0]));
}
