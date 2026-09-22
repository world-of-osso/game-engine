//! Cached character-creation icons clipped by the authored portrait mask.

use std::collections::HashMap;

use bevy::asset::RenderAssetUsages;
use bevy::prelude::{Assets, Handle, Image, Resource};
use bevy::render::render_resource::{Extent3d, TextureDimension, TextureFormat};
use image::{GrayImage, Luma, RgbaImage};

const PORTRAIT_MASK_FDID: u32 = 130_924;

/// Owns masked images for one `Assets<Image>` collection.
#[derive(Resource, Default)]
pub struct CharacterCreationIconMasks {
    icons: HashMap<u32, Handle<Image>>,
    mask_alpha: Option<GrayImage>,
}

impl CharacterCreationIconMasks {
    /// Load the exact icon and authored mask through local CASC, or reuse its image.
    /// Errors never produce an unmasked substitute.
    pub fn get_or_load(
        &mut self,
        icon_fdid: u32,
        images: &mut Assets<Image>,
    ) -> Result<Handle<Image>, String> {
        self.get_or_load_with(icon_fdid, images, load_texture_rgba)
    }

    fn get_or_load_with(
        &mut self,
        icon_fdid: u32,
        images: &mut Assets<Image>,
        mut load: impl FnMut(u32) -> Result<RgbaImage, String>,
    ) -> Result<Handle<Image>, String> {
        if let Some(handle) = self.icons.get(&icon_fdid) {
            return Ok(handle.clone());
        }
        let source = load_nonempty_texture(icon_fdid, "icon", &mut load)?;
        let mask_alpha = match &mut self.mask_alpha {
            Some(mask) => mask,
            empty => {
                let mask = load_nonempty_texture(PORTRAIT_MASK_FDID, "mask", &mut load)?;
                empty.insert(GrayImage::from_fn(mask.width(), mask.height(), |x, y| {
                    Luma([mask.get_pixel(x, y)[3]])
                }))
            }
        };
        let handle = images.add(compose_masked_icon(source, mask_alpha));
        self.icons.insert(icon_fdid, handle.clone());
        Ok(handle)
    }
}

fn load_nonempty_texture(
    fdid: u32,
    role: &str,
    load: &mut impl FnMut(u32) -> Result<RgbaImage, String>,
) -> Result<RgbaImage, String> {
    let image =
        load(fdid).map_err(|error| format!("character-creation {role} FDID {fdid}: {error}"))?;
    if image.width() == 0 || image.height() == 0 {
        return Err(format!(
            "character-creation {role} FDID {fdid} has zero dimensions"
        ));
    }
    Ok(image)
}

fn compose_masked_icon(mut source: RgbaImage, mask_alpha: &GrayImage) -> Image {
    let alpha = image::imageops::resize(
        mask_alpha,
        source.width(),
        source.height(),
        image::imageops::FilterType::Triangle,
    );
    for (pixel, mask) in source.pixels_mut().zip(alpha.pixels()) {
        pixel[3] = (u16::from(pixel[3]) * u16::from(mask[0]) / 255) as u8;
    }
    Image::new(
        Extent3d {
            width: source.width(),
            height: source.height(),
            depth_or_array_layers: 1,
        },
        TextureDimension::D2,
        source.into_raw(),
        TextureFormat::Rgba8UnormSrgb,
        RenderAssetUsages::default(),
    )
}

fn load_texture_rgba(fdid: u32) -> Result<RgbaImage, String> {
    let path = crate::asset::asset_cache::texture(fdid)
        .ok_or_else(|| format!("texture FDID {fdid} could not be resolved from local CASC"))?;
    let (pixels, width, height) = crate::asset::blp::load_blp_rgba(&path)
        .map_err(|error| format!("{}: {error}", path.display()))?;
    RgbaImage::from_raw(width, height, pixels)
        .ok_or_else(|| format!("texture FDID {fdid} has invalid RGBA dimensions"))
}

#[cfg(test)]
mod tests {
    use super::*;
    use image::Rgba;

    const ICON: u32 = 236_448;

    #[test]
    fn authored_mask_uses_alpha_and_preserves_real_icon_colors() {
        let source = load_texture_rgba(ICON).unwrap();
        let mask = load_texture_rgba(PORTRAIT_MASK_FDID).unwrap();
        assert_eq!(mask.dimensions(), (128, 128));
        assert_eq!(mask.get_pixel(0, 0).0, [0, 0, 0, 0]);
        // The authored green channel is 251, but the opaque mask alpha is 255.
        // Multiplying RGB/luminance would incorrectly dim the portrait center.
        assert_eq!(mask.get_pixel(64, 64).0, [255, 251, 255, 255]);

        let mut cache = CharacterCreationIconMasks::default();
        let mut images = Assets::<Image>::default();
        let handle = cache.get_or_load(ICON, &mut images).unwrap();
        let actual = images.get(&handle).unwrap();
        assert_eq!((actual.width(), actual.height()), source.dimensions());
        let rgba = actual.data.as_ref().unwrap();
        for (actual, original) in rgba.chunks_exact(4).zip(source.pixels()) {
            assert_eq!(&actual[..3], &original.0[..3]);
        }
        for (x, y) in [
            (0, 0),
            (source.width() - 1, 0),
            (0, source.height() - 1),
            (source.width() - 1, source.height() - 1),
        ] {
            assert_eq!(rgba[((y * source.width() + x) * 4 + 3) as usize], 0);
        }
        let center = (source.width() / 2, source.height() / 2);
        assert_eq!(
            rgba[((center.1 * source.width() + center.0) * 4 + 3) as usize],
            source.get_pixel(center.0, center.1)[3]
        );
    }

    #[test]
    fn alpha_resizes_deterministically_without_resizing_or_tinting_source() {
        let source = RgbaImage::from_pixel(4, 1, Rgba([40, 80, 120, 128]));
        let mask = RgbaImage::from_fn(2, 1, |x, _| {
            Rgba([255, 0, 20, if x == 0 { 0 } else { 255 }])
        });
        let mut cache = CharacterCreationIconMasks::default();
        let mut images = Assets::<Image>::default();
        let handle = cache
            .get_or_load_with(ICON, &mut images, |id| {
                Ok(if id == ICON {
                    source.clone()
                } else {
                    mask.clone()
                })
            })
            .unwrap();
        let image = images.get(&handle).unwrap();
        assert_eq!((image.width(), image.height()), (4, 1));
        assert_eq!(
            image.data.as_ref().unwrap(),
            &vec![
                40, 80, 120, 0, 40, 80, 120, 32, 40, 80, 120, 95, 40, 80, 120, 128,
            ]
        );
    }

    #[test]
    fn repeated_icon_returns_same_image_without_reloading_assets() {
        let mut cache = CharacterCreationIconMasks::default();
        let mut images = Assets::<Image>::default();
        let first = cache.get_or_load(ICON, &mut images).unwrap();
        let again = cache
            .get_or_load_with(ICON, &mut images, |_| {
                Err("assets no longer available".into())
            })
            .unwrap();
        assert_eq!(first, again);
        assert_eq!(images.len(), 1);
    }

    #[test]
    fn source_failure_is_explicit_and_does_not_poison_retry() {
        let mut cache = CharacterCreationIconMasks::default();
        let mut images = Assets::<Image>::default();
        let error = cache
            .get_or_load_with(ICON, &mut images, |_| Err("invalid source BLP".into()))
            .unwrap_err();
        assert!(error.contains("icon FDID 236448"), "{error}");
        assert!(error.contains("invalid source BLP"), "{error}");
        assert!(images.is_empty());
        assert!(cache.get_or_load(ICON, &mut images).is_ok());
        assert_eq!(images.len(), 1);
    }

    #[test]
    fn mask_failure_is_explicit_without_unmasked_image_fallback() {
        let mut cache = CharacterCreationIconMasks::default();
        let mut images = Assets::<Image>::default();
        let error = cache
            .get_or_load_with(ICON, &mut images, |id| {
                if id == ICON {
                    Ok(RgbaImage::from_pixel(4, 4, Rgba([100, 120, 140, 255])))
                } else {
                    Err("invalid mask BLP".into())
                }
            })
            .unwrap_err();
        assert!(error.contains("mask FDID 130924"), "{error}");
        assert!(error.contains("invalid mask BLP"), "{error}");
        assert!(images.is_empty());
        assert!(cache.get_or_load(ICON, &mut images).is_ok());
    }

    #[test]
    fn zero_sized_source_and_mask_report_errors_without_images() {
        for empty_fdid in [ICON, PORTRAIT_MASK_FDID] {
            let mut cache = CharacterCreationIconMasks::default();
            let mut images = Assets::<Image>::default();
            let error = cache
                .get_or_load_with(ICON, &mut images, |id| {
                    Ok(if id == empty_fdid {
                        RgbaImage::new(0, 0)
                    } else {
                        RgbaImage::from_pixel(4, 4, Rgba([1, 2, 3, 255]))
                    })
                })
                .unwrap_err();
            assert!(error.contains(&empty_fdid.to_string()), "{error}");
            assert!(error.contains("zero"), "{error}");
            assert!(images.is_empty());
        }
    }
}
