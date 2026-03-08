use std::borrow::Cow;

use bevy::image::Image;
use bevy::render::render_resource::TextureFormat;

pub fn encode_webp(img: &Image, quality: f32) -> Result<Vec<u8>, String> {
    let rgba = rgba_bytes(img)?;
    let size = img.size();
    let encoder = webp::Encoder::from_rgba(rgba.as_ref(), size.x, size.y);
    Ok(encoder.encode(quality).to_vec())
}

pub fn rgba_bytes(img: &Image) -> Result<Cow<'_, [u8]>, String> {
    let Some(data) = img.data.as_ref() else {
        return Err("screenshot has no pixel data".into());
    };
    match img.texture_descriptor.format {
        TextureFormat::Rgba8Unorm | TextureFormat::Rgba8UnormSrgb => {
            Ok(Cow::Borrowed(data.as_slice()))
        }
        TextureFormat::Bgra8Unorm | TextureFormat::Bgra8UnormSrgb => {
            let mut rgba = data.clone();
            for pixel in rgba.chunks_exact_mut(4) {
                pixel.swap(0, 2);
            }
            Ok(Cow::Owned(rgba))
        }
        format => Err(format!("unsupported screenshot texture format: {format:?}")),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use bevy::asset::RenderAssetUsages;
    use bevy::render::render_resource::{Extent3d, TextureDimension};

    fn image_with_format(pixels: Vec<u8>, format: TextureFormat) -> Image {
        Image::new(
            Extent3d {
                width: 1,
                height: 1,
                depth_or_array_layers: 1,
            },
            TextureDimension::D2,
            pixels,
            format,
            RenderAssetUsages::default(),
        )
    }

    #[test]
    fn rgba_bytes_passes_rgba_through() {
        let img = image_with_format(vec![1, 2, 3, 4], TextureFormat::Rgba8UnormSrgb);
        let bytes = rgba_bytes(&img).expect("rgba bytes");
        assert_eq!(bytes.as_ref(), &[1, 2, 3, 4]);
    }

    #[test]
    fn rgba_bytes_swaps_bgra_channels() {
        let img = image_with_format(vec![10, 20, 30, 40], TextureFormat::Bgra8UnormSrgb);
        let bytes = rgba_bytes(&img).expect("rgba bytes");
        assert_eq!(bytes.as_ref(), &[30, 20, 10, 40]);
    }
}
