use std::env;
use std::path::Path;

use image::ImageReader;
use ktx2_rw::{Ktx2Texture, VkFormat};

fn main() -> Result<(), String> {
    let args: Vec<String> = env::args().collect();
    if args.len() != 3 {
        return Err("usage: png_to_ktx2 <input.png> <output.ktx2>".to_string());
    }

    let input_path = &args[1];
    let output_path = &args[2];

    let img = ImageReader::open(input_path)
        .map_err(|err| format!("Failed to open PNG: {err}"))?
        .decode()
        .map_err(|err| format!("Failed to decode PNG: {err}"))?;

    let rgba = img.to_rgba8();
    let (width, height) = rgba.dimensions();
    let image_data = rgba.as_raw();

    let mut texture = Ktx2Texture::create(width, height, 1, 1, 1, 1, VkFormat::R8G8B8A8Srgb)
        .map_err(|err| format!("Failed to create KTX2 texture: {err}"))?;

    texture
        .set_image_data(0, 0, 0, image_data)
        .map_err(|err| format!("Failed to set KTX2 image data: {err}"))?;
    texture
        .set_metadata("OriginalFormat", b"PNG")
        .map_err(|err| format!("Failed to set metadata: {err}"))?;
    texture
        .set_metadata("SourceFile", input_path.as_bytes())
        .map_err(|err| format!("Failed to set metadata: {err}"))?;

    let bytes = texture
        .write_to_memory()
        .map_err(|err| format!("Failed to encode KTX2 texture: {err}"))?;

    if let Some(parent) = Path::new(output_path).parent()
        && !parent.as_os_str().is_empty()
    {
        std::fs::create_dir_all(parent)
            .map_err(|err| format!("Failed to create output directory: {err}"))?;
    }
    std::fs::write(output_path, bytes).map_err(|err| format!("Failed to write KTX2 file: {err}"))
}
