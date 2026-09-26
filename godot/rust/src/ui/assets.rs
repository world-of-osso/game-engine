use std::fs;
use std::path::PathBuf;

use godot::classes::{AtlasTexture, FontFile, Image, ImageTexture, ProjectSettings, Texture2D};
use godot::prelude::*;
use ktx2_rw::{Ktx2Texture, VkFormat};
use ui_toolkit::atlas::{self, AtlasSource};
use ui_toolkit::registry::FrameRegistry;
use ui_toolkit::widgets::font_string::GameFont;
use ui_toolkit::widgets::texture::TextureSource;

fn asset_path(path: &str) -> PathBuf {
    let source = if path.starts_with("data/") {
        &path[5..]
    } else {
        path
    };
    let resource_path = format!("res://../data/{source}");
    PathBuf::from(
        ProjectSettings::singleton()
            .globalize_path(&resource_path)
            .to_string(),
    )
}

fn load_bytes(path: &str) -> Result<Vec<u8>, String> {
    fs::read(asset_path(path)).map_err(|error| format!("Read authored UI asset {path}: {error}"))
}

pub fn load_font(font: GameFont) -> Result<Gd<FontFile>, String> {
    let path = match font {
        GameFont::FrizQuadrata => "data/fonts/FRIZQT__.TTF",
        GameFont::ArialNarrow => "data/fonts/ARIALN.ttf",
    };
    let bytes = load_bytes(path)?;
    let mut resource = FontFile::new_gd();
    resource.set_data(&PackedByteArray::from(bytes.as_slice()));
    Ok(resource)
}

fn image_from_rgba(width: u32, height: u32, bytes: &[u8]) -> Result<Gd<ImageTexture>, String> {
    if bytes.len() != width as usize * height as usize * 4 {
        return Err(format!(
            "RGBA image {width}x{height} has {} bytes",
            bytes.len()
        ));
    }
    let image = Image::create_from_data(
        width as i32,
        height as i32,
        false,
        godot::classes::image::Format::RGBA8,
        &PackedByteArray::from(bytes),
    )
    .ok_or_else(|| format!("Godot rejected {width}x{height} authored image"))?;
    ImageTexture::create_from_image(&image)
        .ok_or_else(|| "Godot rejected authored image texture".into())
}

fn decode_ktx(path: &str, bytes: &[u8]) -> Result<Gd<ImageTexture>, String> {
    let texture =
        Ktx2Texture::from_memory(bytes).map_err(|error| format!("Decode KTX2 {path}: {error}"))?;
    if !matches!(
        texture.vk_format(),
        VkFormat::R8G8B8A8Unorm | VkFormat::R8G8B8A8Srgb
    ) {
        return Err(format!(
            "Unsupported authored KTX2 format {:?} in {path}",
            texture.vk_format()
        ));
    }
    let pixels = texture
        .get_image_data(0, 0, 0)
        .map_err(|error| format!("Read KTX2 image {path}: {error}"))?;
    image_from_rgba(texture.width(), texture.height(), &pixels)
}

fn decode_png(path: &str, bytes: &[u8]) -> Result<Gd<ImageTexture>, String> {
    let mut image = Image::new_gd();
    let error = image.load_png_from_buffer(&PackedByteArray::from(bytes));
    if error != godot::global::Error::OK {
        return Err(format!("Decode PNG {path}: {error:?}"));
    }
    ImageTexture::create_from_image(&image)
        .ok_or_else(|| format!("Create PNG texture {path}: Godot rejected decoded image"))
}

fn load_file(path: &str) -> Result<Gd<ImageTexture>, String> {
    let bytes = load_bytes(path)?;
    if path.to_ascii_lowercase().ends_with(".ktx2") {
        return decode_ktx(path, &bytes);
    }
    if path.to_ascii_lowercase().ends_with(".png") {
        return decode_png(path, &bytes);
    }
    if path.to_ascii_lowercase().ends_with(".blp") {
        let rgba = game_engine_core::blp::decode_rgba(&bytes)
            .map_err(|error| format!("Decode BLP {path}: {error}"))?;
        return image_from_rgba(rgba.width, rgba.height, &rgba.pixels);
    }
    Err(format!("Unsupported authored UI texture file {path}"))
}

pub fn load_texture(
    source: &TextureSource,
    registry: &FrameRegistry,
) -> Result<Gd<Texture2D>, String> {
    match source {
        TextureSource::File(path) => load_file(path).map(Gd::upcast),
        TextureSource::FileDataId(id) => {
            load_file(&format!("data/textures/{id}.blp")).map(Gd::upcast)
        }
        TextureSource::Atlas(name) => {
            let region =
                atlas::get_region(name).ok_or_else(|| format!("Unknown UI atlas region {name}"))?;
            let source = match region.source {
                AtlasSource::File(path) => TextureSource::File(path.into()),
                AtlasSource::FileDataId(id) => TextureSource::FileDataId(id),
            };
            let full = load_texture(&source, registry)?;
            let width = full.get_width();
            let height = full.get_height();
            let pixels = region.rect_pixels(width as u32, height as u32);
            let mut atlas = AtlasTexture::new_gd();
            atlas.set_atlas(&full);
            atlas.set_region(Rect2::new(
                Vector2::new(pixels.min[0], pixels.min[1]),
                Vector2::new(pixels.max[0] - pixels.min[0], pixels.max[1] - pixels.min[1]),
            ));
            Ok(atlas.upcast())
        }
        TextureSource::None => Err("No authored texture source".into()),
        TextureSource::SolidColor(_) => Err("Solid color must be drawn natively".into()),
        TextureSource::Dynamic(id) => {
            let image = registry
                .dynamic_texture(*id)
                .ok_or_else(|| format!("Dynamic UI texture {} not found", id.0))?;
            image_from_rgba(image.width, image.height, &image.rgba8).map(Gd::upcast)
        }
    }
}
