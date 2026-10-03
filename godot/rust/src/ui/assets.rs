use std::cell::RefCell;
use std::collections::HashMap;
use std::fs;
use std::path::PathBuf;

use game_engine_core::asset_loader::{AssetLoader, Priority};

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

thread_local! {
    /// Parsed font files shared by every UI: Godot caches glyphs per font and size, so
    /// one `FontFile` serves all labels.
    static FONTS: RefCell<HashMap<GameFont, Gd<FontFile>>> = RefCell::new(HashMap::new());
}

pub fn load_font(font: GameFont) -> Result<Gd<FontFile>, String> {
    if let Some(loaded) = FONTS.with_borrow(|fonts| fonts.get(&font).cloned()) {
        return Ok(loaded);
    }
    let _span = crate::profile::span(|| format!("ui.load_font {font:?}"));
    let path = match font {
        GameFont::FrizQuadrata => "data/fonts/FRIZQT__.TTF",
        GameFont::ArialNarrow => "data/fonts/ARIALN.ttf",
    };
    let bytes = load_bytes(path)?;
    let mut resource = FontFile::new_gd();
    resource.set_data(&PackedByteArray::from(bytes.as_slice()));
    FONTS.with_borrow_mut(|fonts| fonts.insert(font, resource.clone()));
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

/// An authored UI image file decoded to RGBA8, on a worker.
#[derive(Debug, PartialEq)]
struct DecodedFile {
    width: u32,
    height: u32,
    rgba: Vec<u8>,
}

/// Decode `bytes` of authored UI file `path` by its extension; touches no Godot object.
fn decode_file(path: &str, bytes: &[u8]) -> Result<DecodedFile, String> {
    let extension = path.rsplit('.').next().unwrap_or("").to_ascii_lowercase();
    match extension.as_str() {
        "ktx2" => decode_ktx(path, bytes),
        "png" => decode_png(path, bytes),
        "tga" => decode_tga(path, bytes),
        "blp" => decode_blp_bytes(path, bytes).map(|image| DecodedFile {
            width: image.width,
            height: image.height,
            rgba: image.pixels,
        }),
        _ => Err(format!("Unsupported authored UI texture file {path}")),
    }
}

fn decode_ktx(path: &str, bytes: &[u8]) -> Result<DecodedFile, String> {
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
    let rgba = texture
        .get_image_data(0, 0, 0)
        .map_err(|error| format!("Read KTX2 image {path}: {error}"))?
        .to_vec();
    Ok(DecodedFile {
        width: texture.width(),
        height: texture.height(),
        rgba,
    })
}

/// Any PNG colour type and depth as RGBA8, as Godot's `load_png_from_buffer` reads it.
fn decode_png(path: &str, bytes: &[u8]) -> Result<DecodedFile, String> {
    let mut decoder = png::Decoder::new(bytes);
    decoder.set_transformations(png::Transformations::normalize_to_color8());
    let mut reader = decoder
        .read_info()
        .map_err(|error| format!("Decode PNG {path}: {error}"))?;
    let mut pixels = vec![0; reader.output_buffer_size()];
    let frame = reader
        .next_frame(&mut pixels)
        .map_err(|error| format!("Decode PNG {path}: {error}"))?;
    pixels.truncate(frame.buffer_size());
    let rgba = match frame.color_type {
        png::ColorType::Rgba => pixels,
        png::ColorType::Rgb => pixels
            .chunks_exact(3)
            .flat_map(|rgb| [rgb[0], rgb[1], rgb[2], 255])
            .collect(),
        png::ColorType::GrayscaleAlpha => pixels
            .chunks_exact(2)
            .flat_map(|la| [la[0], la[0], la[0], la[1]])
            .collect(),
        png::ColorType::Grayscale => pixels.iter().flat_map(|&l| [l, l, l, 255]).collect(),
        png::ColorType::Indexed => {
            return Err(format!("Decode PNG {path}: palette left unexpanded"));
        }
    };
    Ok(DecodedFile {
        width: frame.width,
        height: frame.height,
        rgba,
    })
}

/// Addon art such as Plumber's `LoadingIndicator32.tga`.
fn decode_tga(path: &str, bytes: &[u8]) -> Result<DecodedFile, String> {
    let image = image::load_from_memory_with_format(bytes, image::ImageFormat::Tga)
        .map_err(|error| format!("Decode TGA {path}: {error}"))?
        .into_rgba8();
    Ok(DecodedFile {
        width: image.width(),
        height: image.height(),
        rgba: image.into_raw(),
    })
}

pub fn decode_blp(path: &str) -> Result<game_engine_core::blp::RgbaImage, String> {
    decode_blp_bytes(path, &load_bytes(path)?)
}

fn decode_blp_bytes(path: &str, bytes: &[u8]) -> Result<game_engine_core::blp::RgbaImage, String> {
    game_engine_core::blp::decode_rgba(bytes).map_err(|error| format!("Decode BLP {path}: {error}"))
}

/// An authored UI file: its `data/`-relative path and where it is on disk.
#[derive(Clone, PartialEq, Eq, Hash)]
struct UiFile {
    path: String,
    file: PathBuf,
}

/// Authored UI image files, read and decoded on worker threads and shared by every UI:
/// a frame shows its art from the frame it has arrived (retail UI textures load
/// asynchronously too) and no screen waits for a file.
struct FileTextures {
    loader: AssetLoader<UiFile, DecodedFile>,
    textures: HashMap<String, Result<Gd<ImageTexture>, String>>,
    /// Files that arrived so far; a projection redraws its waiting frames when it grows.
    arrived: u64,
}

impl FileTextures {
    fn new() -> Self {
        Self {
            loader: AssetLoader::new("ui-textures", 2, |file: &UiFile| {
                let bytes = fs::read(&file.file)
                    .map_err(|error| format!("Read authored UI asset {}: {error}", file.path))?;
                decode_file(&file.path, &bytes)
            }),
            textures: HashMap::new(),
            arrived: 0,
        }
    }

    /// Make the textures of the files decoded since the last poll.
    fn poll(&mut self) {
        for (file, decoded) in self.loader.poll() {
            let texture = decoded
                .and_then(|decoded| image_from_rgba(decoded.width, decoded.height, &decoded.rgba));
            self.textures.insert(file.path, texture);
            self.arrived += 1;
        }
    }

    /// File `path`'s texture; `Ok(None)` while it loads. Arrivals are taken once a frame
    /// (`arrived_file_textures`), so one layout pass sees one set of textures.
    fn get(&mut self, path: &str) -> Result<Option<Gd<ImageTexture>>, String> {
        if let Some(texture) = self.textures.get(path) {
            return texture.clone().map(Some);
        }
        self.loader.request(
            UiFile {
                path: path.to_owned(),
                file: asset_path(path),
            },
            Priority::Now,
        );
        Ok(None)
    }
}

thread_local! {
    static FILE_TEXTURES: RefCell<Option<FileTextures>> = const { RefCell::new(None) };
}

fn with_file_textures<T>(visit: impl FnOnce(&mut FileTextures) -> T) -> T {
    FILE_TEXTURES.with_borrow_mut(|textures| visit(textures.get_or_insert_with(FileTextures::new)))
}

/// FileDataID `id`'s art (`data/textures/<id>.blp`); `Ok(None)` while it loads.
pub fn load_file_data_id(id: u32) -> Result<Option<Gd<ImageTexture>>, String> {
    load_file(&format!("data/textures/{id}.blp"))
}

/// UI files whose textures arrived so far.
pub fn arrived_file_textures() -> u64 {
    with_file_textures(|textures| {
        textures.poll();
        textures.arrived
    })
}

/// Whether FileDataID `id`'s art has arrived (or failed to load); starts its load.
pub fn file_data_id_arrived(id: u32) -> bool {
    with_file_textures(|textures| {
        textures.poll();
        !matches!(textures.get(&format!("data/textures/{id}.blp")), Ok(None))
    })
}

fn load_file(path: &str) -> Result<Option<Gd<ImageTexture>>, String> {
    with_file_textures(|textures| textures.get(path))
}

/// Decoded source image and the pixel region `[x, y, w, h]` an atlas name selects;
/// `Ok(None)` while its file loads.
pub fn load_source(
    source: &TextureSource,
    registry: &FrameRegistry,
) -> Result<Option<(Gd<Texture2D>, [f32; 4])>, String> {
    let full = |image: Gd<ImageTexture>| {
        let region = [
            0.0,
            0.0,
            image.get_width() as f32,
            image.get_height() as f32,
        ];
        (image.upcast::<Texture2D>(), region)
    };
    match source {
        TextureSource::File(path) => load_file(path).map(|image| image.map(full)),
        TextureSource::FileDataId(id) => {
            load_file(&format!("data/textures/{id}.blp")).map(|image| image.map(full))
        }
        TextureSource::Atlas(name) => {
            let region =
                atlas::get_region(name).ok_or_else(|| format!("Unknown UI atlas region {name}"))?;
            let source = match region.source {
                AtlasSource::File(path) => TextureSource::File(path.into()),
                AtlasSource::FileDataId(id) => TextureSource::FileDataId(id),
            };
            let Some((image, _)) = load_source(&source, registry)? else {
                return Ok(None);
            };
            let pixels = region.rect_pixels(image.get_width() as u32, image.get_height() as u32);
            let rect = [
                pixels.min[0],
                pixels.min[1],
                pixels.max[0] - pixels.min[0],
                pixels.max[1] - pixels.min[1],
            ];
            Ok(Some((image, rect)))
        }
        TextureSource::None => Err("No authored texture source".into()),
        TextureSource::SolidColor(_) => Err("Solid color must be drawn natively".into()),
        TextureSource::Dynamic(id) => {
            let image = registry
                .dynamic_texture(*id)
                .ok_or_else(|| format!("Dynamic UI texture {} not found", id.0))?;
            image_from_rgba(image.width, image.height, &image.rgba8).map(|image| Some(full(image)))
        }
    }
}

/// `image` restricted to pixel `region`; whole images are returned unchanged.
pub fn sub_texture(image: &Gd<Texture2D>, region: [f32; 4]) -> Gd<Texture2D> {
    let [x, y, w, h] = region;
    if x == 0.0 && y == 0.0 && w == image.get_width() as f32 && h == image.get_height() as f32 {
        return image.clone();
    }
    let mut atlas = AtlasTexture::new_gd();
    atlas.set_atlas(image);
    atlas.set_region(Rect2::new(Vector2::new(x, y), Vector2::new(w, h)));
    atlas.upcast()
}

#[cfg(test)]
mod decode_tests {
    use super::*;

    fn png_bytes(color: png::ColorType, width: u32, pixels: &[u8]) -> Vec<u8> {
        let mut bytes = Vec::new();
        let channels = color.samples() as u32;
        let height = pixels.len() as u32 / (width * channels);
        let mut encoder = png::Encoder::new(&mut bytes, width, height);
        encoder.set_color(color);
        encoder.set_depth(png::BitDepth::Eight);
        let mut writer = encoder.write_header().unwrap();
        writer.write_image_data(pixels).unwrap();
        writer.finish().unwrap();
        bytes
    }

    #[test]
    fn png_colour_types_decode_to_rgba8() {
        let rgb = png_bytes(png::ColorType::Rgb, 2, &[200, 10, 20, 30, 40, 250]);
        assert_eq!(
            decode_file("data/ui/loading-screen.png", &rgb).unwrap(),
            DecodedFile {
                width: 2,
                height: 1,
                rgba: vec![200, 10, 20, 255, 30, 40, 250, 255],
            }
        );
        let gray_alpha = png_bytes(png::ColorType::GrayscaleAlpha, 1, &[90, 128]);
        assert_eq!(
            decode_file("art.PNG", &gray_alpha).unwrap().rgba,
            vec![90, 90, 90, 128]
        );
        let gray = png_bytes(png::ColorType::Grayscale, 1, &[7]);
        assert_eq!(
            decode_file("art.png", &gray).unwrap().rgba,
            vec![7, 7, 7, 255]
        );
    }

    /// An uncompressed 32-bit top-left TGA such as addon loading indicators.
    #[test]
    fn tga_decodes_to_rgba8() {
        let mut tga = vec![0, 0, 2, 0, 0, 0, 0, 0, 0, 0, 0, 0, 2, 0, 1, 0, 32, 0x28];
        // BGRA pixels.
        tga.extend_from_slice(&[30, 20, 10, 255, 3, 2, 1, 64]);
        assert_eq!(
            decode_file("Interface/AddOns/Plumber/LoadingIndicator32.tga", &tga).unwrap(),
            DecodedFile {
                width: 2,
                height: 1,
                rgba: vec![10, 20, 30, 255, 1, 2, 3, 64],
            }
        );
    }

    #[test]
    fn unknown_extensions_are_rejected() {
        assert_eq!(
            decode_file("data/ui/art.jpg", &[]),
            Err("Unsupported authored UI texture file data/ui/art.jpg".into())
        );
    }
}
