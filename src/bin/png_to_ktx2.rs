use std::env;
use std::path::Path;

use image::ImageReader;
use ktx2_rw::{Ktx2Texture, VkFormat};

fn box_filter_mip(src: &[u8], src_w: u32, src_h: u32) -> Vec<u8> {
    let dst_w = (src_w / 2).max(1);
    let dst_h = (src_h / 2).max(1);
    let mut dst = Vec::with_capacity((dst_w * dst_h * 4) as usize);

    for y in 0..dst_h {
        for x in 0..dst_w {
            let mut rgba = [0u32; 4];
            let src_x = x * 2;
            let src_y = y * 2;

            for dy in 0..2u32 {
                for dx in 0..2u32 {
                    let px = (src_x + dx).min(src_w - 1);
                    let py = (src_y + dy).min(src_h - 1);
                    let offset = ((py * src_w + px) * 4) as usize;
                    for c in 0..4 {
                        rgba[c] += src[offset + c] as u32;
                    }
                }
            }

            for c in 0..4 {
                dst.push((rgba[c] / 4) as u8);
            }
        }
    }

    dst
}

fn generate_mipmaps(base: &[u8], width: u32, height: u32) -> Vec<Vec<u8>> {
    let mut levels: Vec<Vec<u8>> = vec![base.to_vec()];
    let mut w = width;
    let mut h = height;

    while w > 1 || h > 1 {
        let prev = levels.last().unwrap();
        let mip = box_filter_mip(prev, w, h);
        w = (w / 2).max(1);
        h = (h / 2).max(1);
        levels.push(mip);
    }

    levels
}

fn load_png_rgba(path: &str) -> Result<(Vec<u8>, u32, u32), String> {
    let img = ImageReader::open(path)
        .map_err(|err| format!("Failed to open PNG: {err}"))?
        .decode()
        .map_err(|err| format!("Failed to decode PNG: {err}"))?;
    let rgba = img.to_rgba8();
    let (width, height) = rgba.dimensions();
    Ok((rgba.into_raw(), width, height))
}

fn build_ktx2(mips: &[Vec<u8>], width: u32, height: u32) -> Result<Ktx2Texture, String> {
    let level_count = mips.len() as u32;
    let mut texture =
        Ktx2Texture::create(width, height, 1, 1, 1, level_count, VkFormat::R8G8B8A8Srgb)
            .map_err(|err| format!("Failed to create KTX2 texture: {err}"))?;
    for (level, mip_data) in mips.iter().enumerate() {
        texture
            .set_image_data(level as u32, 0, 0, mip_data)
            .map_err(|err| format!("Failed to set image data for level {level}: {err}"))?;
    }
    Ok(texture)
}

fn write_ktx2(texture: Ktx2Texture, input_path: &str, output_path: &str) -> Result<(), String> {
    let mut texture = texture;
    texture.set_metadata("OriginalFormat", b"PNG").map_err(|e| format!("metadata: {e}"))?;
    texture
        .set_metadata("SourceFile", input_path.as_bytes())
        .map_err(|e| format!("metadata: {e}"))?;

    let bytes = texture.write_to_memory().map_err(|e| format!("Failed to encode KTX2: {e}"))?;

    if let Some(parent) = Path::new(output_path).parent()
        && !parent.as_os_str().is_empty()
    {
        std::fs::create_dir_all(parent)
            .map_err(|e| format!("Failed to create output directory: {e}"))?;
    }

    std::fs::write(output_path, bytes).map_err(|e| format!("Failed to write KTX2 file: {e}"))
}

fn main() -> Result<(), String> {
    let args: Vec<String> = env::args().collect();
    if args.len() != 3 {
        return Err("usage: png_to_ktx2 <input.png> <output.ktx2>".to_string());
    }

    let input_path = &args[1];
    let output_path = &args[2];

    let (image_data, width, height) = load_png_rgba(input_path)?;
    let mips = generate_mipmaps(&image_data, width, height);
    let texture = build_ktx2(&mips, width, height)?;
    write_ktx2(texture, input_path, output_path)
}
