use super::blp_format::{fix_1bit_alpha, strip_mipmaps};
use std::path::Path;
use std::sync::OnceLock;
use std::time::Instant;

use bevy::asset::RenderAssetUsages;
use bevy::image::Image;
use bevy::render::render_resource::{Extent3d, TextureDimension, TextureFormat};
use image_blp::convert::blp_to_image;
use image_blp::parser::load_blp_from_buf;
use image_blp::types::BlpContent;

pub fn load_blp_to_image(path: &Path) -> Result<Image, String> {
    let (pixels, width, height, timings) = load_blp_rgba_with_timing(path)?;
    log_blp_perf(path, width, height, timings);

    Ok(Image::new(
        Extent3d {
            width,
            height,
            depth_or_array_layers: 1,
        },
        TextureDimension::D2,
        pixels,
        TextureFormat::Rgba8UnormSrgb,
        RenderAssetUsages::default(),
    ))
}

/// Load a BLP as a GPU-compressed Image when possible (BC1/BC2/BC3).
/// Falls back to RGBA8 decompression for JPEG/Raw BLPs.
/// This skips CPU-side DXT decompression entirely for DXT BLPs.
pub fn load_blp_gpu_image(path: &Path) -> Result<Image, String> {
    let blp = load_blp(path)?;
    let (w, h) = (blp.header.width, blp.header.height);
    match &blp.content {
        BlpContent::Dxt1(dxtn) => gpu_image_from_dxtn(dxtn, w, h, TextureFormat::Bc1RgbaUnormSrgb),
        BlpContent::Dxt3(dxtn) => gpu_image_from_dxtn(dxtn, w, h, TextureFormat::Bc2RgbaUnormSrgb),
        BlpContent::Dxt5(dxtn) => gpu_image_from_dxtn(dxtn, w, h, TextureFormat::Bc3RgbaUnormSrgb),
        _ => {
            // Non-DXT: fall back to CPU decode
            load_blp_to_image(path)
        }
    }
}

fn gpu_image_from_dxtn(
    dxtn: &image_blp::types::BlpDxtn,
    width: u32,
    height: u32,
    format: TextureFormat,
) -> Result<Image, String> {
    let data = dxtn
        .images
        .first()
        .ok_or_else(|| "BLP DXT has no mipmap level 0".to_string())?;
    let (w, h) = dxtn_actual_dimensions(width, height, data.content.len(), format);
    Ok(Image::new(
        Extent3d {
            width: w,
            height: h,
            depth_or_array_layers: 1,
        },
        TextureDimension::D2,
        data.content.clone(),
        format,
        RenderAssetUsages::default(),
    ))
}

/// Some BLPs have truncated mipmaps — header says 128×128 but mip0 data
/// only fits a smaller resolution. Infer actual dimensions from data size.
fn dxtn_actual_dimensions(w: u32, h: u32, data_len: usize, format: TextureFormat) -> (u32, u32) {
    let block_bytes = match format {
        TextureFormat::Bc1RgbaUnormSrgb => 8,
        _ => 16, // BC2, BC3
    };
    let expected = dxtn_size(w, h, block_bytes);
    if data_len >= expected {
        return (w, h);
    }
    let (mut mw, mut mh) = (w, h);
    while mw > 4 && mh > 4 {
        mw /= 2;
        mh /= 2;
        if data_len >= dxtn_size(mw, mh, block_bytes) {
            return (mw, mh);
        }
    }
    (4.max(mw), 4.max(mh))
}

fn dxtn_size(w: u32, h: u32, block_bytes: usize) -> usize {
    let bw = w.div_ceil(4) as usize;
    let bh = h.div_ceil(4) as usize;
    bw * bh * block_bytes
}

/// Load a BLP file and return raw RGBA pixels + dimensions.
pub fn load_blp_rgba(path: &Path) -> Result<(Vec<u8>, u32, u32), String> {
    let (pixels, width, height, timings) = load_blp_rgba_with_timing(path)?;
    log_blp_perf(path, width, height, timings);
    Ok((pixels, width, height))
}

#[derive(Clone, Copy)]
struct BlpDecodeTimings {
    read_us: u64,
    decode_us: u64,
    convert_us: u64,
    alpha_us: u64,
}

fn load_blp_rgba_with_timing(path: &Path) -> Result<(Vec<u8>, u32, u32, BlpDecodeTimings), String> {
    let measure = blp_perf_enabled();
    let (bytes, read_us) = measure_blp_stage(measure, || std::fs::read(path));
    let mut bytes = bytes.map_err(|e| format!("Failed to read BLP: {e}"))?;
    let (blp, decode_us) = measure_blp_stage(measure, || {
        strip_mipmaps(&mut bytes);
        load_blp_from_buf(&bytes)
    });
    let blp = blp.map_err(|e| format!("Failed to load BLP: {e}"))?;
    let (rgba, convert_us) = measure_blp_stage(measure, || {
        blp_to_image(&blp, 0).map(|image| image.to_rgba8())
    });
    let rgba = rgba.map_err(|e| format!("Failed to convert BLP: {e}"))?;
    let width = rgba.width();
    let height = rgba.height();
    let mut pixels = rgba.into_raw();
    let (_, alpha_us) = measure_blp_stage(measure, || fix_1bit_alpha(&mut pixels));

    Ok((
        pixels,
        width,
        height,
        BlpDecodeTimings {
            read_us,
            decode_us,
            convert_us,
            alpha_us,
        },
    ))
}

fn load_blp(path: &Path) -> Result<image_blp::types::BlpImage, String> {
    let mut bytes = std::fs::read(path).map_err(|e| format!("Failed to read BLP: {e}"))?;
    strip_mipmaps(&mut bytes);
    load_blp_from_buf(&bytes).map_err(|e| format!("Failed to load BLP: {e}"))
}

fn blp_perf_enabled() -> bool {
    static ENABLED: OnceLock<bool> = OnceLock::new();

    *ENABLED.get_or_init(|| std::env::var_os("WOO_PERF_MOVEMENT").is_some())
}

fn measure_blp_stage<T>(enabled: bool, operation: impl FnOnce() -> T) -> (T, u64) {
    if !enabled {
        return (operation(), 0);
    }

    let started_at = Instant::now();
    let result = operation();
    (result, started_at.elapsed().as_micros() as u64)
}

fn log_blp_perf(path: &Path, width: u32, height: u32, timings: BlpDecodeTimings) {
    if blp_perf_enabled() {
        eprintln!("{}", format_blp_perf(path, width, height, timings));
    }
}

fn format_blp_perf(path: &Path, width: u32, height: u32, timings: BlpDecodeTimings) -> String {
    format!(
        "blp_perf path={} width={width} height={height} read_us={} decode_us={} convert_us={} alpha_us={}",
        path.display(),
        timings.read_us,
        timings.decode_us,
        timings.convert_us,
        timings.alpha_us,
    )
}

#[path = "rgba_blit.rs"]
mod rgba_blit;
pub use rgba_blit::{blit_region, scale_2x};

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn formats_blp_decode_timings_as_key_value_diagnostics() {
        let timings = BlpDecodeTimings {
            read_us: 11,
            decode_us: 22,
            convert_us: 33,
            alpha_us: 44,
        };

        assert_eq!(
            format_blp_perf(Path::new("data/textures/example.blp"), 64, 32, timings),
            "blp_perf path=data/textures/example.blp width=64 height=32 read_us=11 decode_us=22 convert_us=33 alpha_us=44"
        );
    }
}
