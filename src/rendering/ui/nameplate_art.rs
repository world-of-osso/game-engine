//! Reference-derived nameplate skins; dimensions are unscaled screenshot pixels.
use bevy::image::ImageSampler;
use bevy::prelude::*;
use bevy::text::Font;

pub(crate) const NAMEPLATE_SCALE: f32 = 0.5;
pub(crate) const BAR_PIXEL_WIDTH: f32 = 376.0 * NAMEPLATE_SCALE;
pub(crate) const CAST_FILL_RECT: Rect = Rect::new(268.0, 124.0, 477.0, 135.0);
pub(crate) const CAST_BACKGROUND_RECT: Rect = Rect::new(57.0, 85.0, 266.0, 96.0);

#[derive(Clone)]
pub(crate) struct NameplateArt {
    pub health_fill_thin: Handle<Image>,
    pub health_fill_thick: Handle<Image>,
    pub health_thick: Handle<Image>,
    pub health_thin: Handle<Image>,
    pub casting: Handle<Image>,
    /// `casting` with its colour removed, tinted per cast type by `NameplateStyle`.
    pub cast_fill: Handle<Image>,
    pub cast_thick: Handle<Image>,
    pub cast_thin: Handle<Image>,
    pub font: Handle<Font>,
}

#[derive(Resource, Default)]
pub(crate) struct NameplateArtCache(Option<NameplateArt>);

impl NameplateArtCache {
    pub fn load(
        &mut self,
        images: &mut Assets<Image>,
        fonts: &mut Assets<Font>,
    ) -> Result<NameplateArt, String> {
        if let Some(art) = &self.0 {
            return Ok(art.clone());
        }
        let health_fill_thin =
            load_fill(include_bytes!("nameplate_skins/health-fill.png"), images)?;
        let health_fill_thick = load_fill(
            include_bytes!("nameplate_skins/health-fill-thick.png"),
            images,
        )?;
        let health_thick = load_skin(include_bytes!("nameplate_skins/health-thick.png"), images)?;
        let health_thin = load_skin(include_bytes!("nameplate_skins/health-thin.png"), images)?;
        let casting = load_fdid_texture(4505182, images)?;
        let cast_fill = desaturated_copy(&casting, images)?;
        let cast_thick = load_skin(include_bytes!("nameplate_skins/cast-thick.png"), images)?;
        let cast_thin = load_skin(include_bytes!("nameplate_skins/cast-thin.png"), images)?;
        let path = "data/fonts/FRIZQT__.TTF";
        let bytes =
            std::fs::read(path).map_err(|error| format!("Nameplate font {path}: {error}"))?;
        ab_glyph::FontRef::try_from_slice(&bytes)
            .map_err(|error| format!("Nameplate font {path}: {error}"))?;
        let art = NameplateArt {
            health_fill_thin,
            health_fill_thick,
            health_thick,
            health_thin,
            casting,
            cast_fill,
            cast_thick,
            cast_thin,
            font: fonts.add(Font::from_bytes(bytes)),
        };
        self.0 = Some(art.clone());
        Ok(art)
    }

    pub fn art(&self) -> &NameplateArt {
        self.0
            .as_ref()
            .expect("art loaded before nameplate visuals spawn")
    }

    #[cfg(test)]
    pub fn fixture(images: &mut Assets<Image>, fonts: &mut Assets<Font>) -> Self {
        use bevy::asset::RenderAssetUsages;
        use bevy::render::render_resource::{Extent3d, TextureDimension, TextureFormat};
        let image = images.add(Image::new_fill(
            Extent3d {
                width: 512,
                height: 256,
                depth_or_array_layers: 1,
            },
            TextureDimension::D2,
            &[255, 255, 255, 255],
            TextureFormat::Rgba8UnormSrgb,
            RenderAssetUsages::default(),
        ));
        Self(Some(NameplateArt {
            health_fill_thin: image.clone(),
            health_fill_thick: image.clone(),
            health_thick: image.clone(),
            health_thin: image.clone(),
            casting: image.clone(),
            cast_fill: image.clone(),
            cast_thick: image.clone(),
            cast_thin: image,
            font: fonts.add(Font::from_bytes(bevy::text::DEFAULT_FONT_DATA.to_vec())),
        }))
    }
}

fn load_skin(bytes: &[u8], images: &mut Assets<Image>) -> Result<Handle<Image>, String> {
    Ok(images.add(decode_skin(bytes)?))
}

/// Fills carry only shading; `NameplateStyle` supplies their colour as the sprite tint.
fn load_fill(bytes: &[u8], images: &mut Assets<Image>) -> Result<Handle<Image>, String> {
    let mut image = decode_skin(bytes)?;
    desaturate(&mut image)?;
    Ok(images.add(image))
}

fn desaturated_copy(
    source: &Handle<Image>,
    images: &mut Assets<Image>,
) -> Result<Handle<Image>, String> {
    let mut image = images
        .get(source)
        .ok_or("Nameplate cast atlas missing after load")?
        .clone();
    desaturate(&mut image)?;
    Ok(images.add(image))
}

/// Replaces each RGBA8 pixel's colour by its HSV value (max channel), so a pure tint such as
/// red reproduces the original red channel exactly.
fn desaturate(image: &mut Image) -> Result<(), String> {
    use bevy::render::render_resource::TextureFormat;
    if !matches!(
        image.texture_descriptor.format,
        TextureFormat::Rgba8UnormSrgb | TextureFormat::Rgba8Unorm
    ) {
        return Err(format!(
            "Nameplate fill format {:?} is not RGBA8",
            image.texture_descriptor.format
        ));
    }
    let pixels = image
        .data
        .as_mut()
        .ok_or("Nameplate fill has no pixel data")?;
    for pixel in pixels.chunks_exact_mut(4) {
        let value = pixel[0].max(pixel[1]).max(pixel[2]);
        pixel[..3].fill(value);
    }
    Ok(())
}

fn decode_skin(bytes: &[u8]) -> Result<Image, String> {
    use bevy::asset::RenderAssetUsages;
    use bevy::render::render_resource::{Extent3d, TextureDimension, TextureFormat};
    let rgba = image::load_from_memory(bytes)
        .map_err(|error| format!("Nameplate skin: {error}"))?
        .to_rgba8();
    let mut image = Image::new(
        Extent3d {
            width: rgba.width(),
            height: rgba.height(),
            depth_or_array_layers: 1,
        },
        TextureDimension::D2,
        rgba.into_raw(),
        TextureFormat::Rgba8UnormSrgb,
        RenderAssetUsages::default(),
    );
    image.sampler = ImageSampler::linear();
    Ok(image)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn health_fill_keeps_reference_shading_as_grey_so_tints_recolour_it() {
        let mut images = Assets::<Image>::default();
        let reference =
            decode_skin(include_bytes!("nameplate_skins/health-fill-thick.png")).unwrap();
        let handle = load_fill(
            include_bytes!("nameplate_skins/health-fill-thick.png"),
            &mut images,
        )
        .unwrap();
        let fill = images.get(&handle).unwrap();
        let (source, grey) = (reference.data.unwrap(), fill.data.as_ref().unwrap());
        // Row 12, column 143: the red body of the reference (195, 43, 41).
        let at = ((12 * fill.width() + 143) * 4) as usize;
        assert_eq!(&source[at..at + 4], &[195, 43, 41, 255]);
        assert_eq!(&grey[at..at + 4], &[195, 195, 195, 255]);
        for (grey, source) in grey.chunks_exact(4).zip(source.chunks_exact(4)) {
            assert_eq!(grey[0], source[..3].iter().copied().max().unwrap());
            assert_eq!((grey[1], grey[2], grey[3]), (grey[0], grey[0], source[3]));
        }
    }

    #[test]
    fn reference_frames_leave_live_values_and_labels_uncovered() {
        let cases: &[(&[u8], [u32; 4])] = &[
            (
                include_bytes!("nameplate_skins/health-thick.png"),
                [8, 6, 384, 44],
            ),
            (
                include_bytes!("nameplate_skins/health-thin.png"),
                [8, 6, 384, 25],
            ),
            (
                include_bytes!("nameplate_skins/cast-thin.png"),
                [28, 6, 400, 17],
            ),
            (
                include_bytes!("nameplate_skins/cast-thick.png"),
                [29, 7, 401, 25],
            ),
        ];
        let mut images = Assets::<Image>::default();
        for (bytes, [left, top, right, bottom]) in cases {
            let handle = load_skin(bytes, &mut images).unwrap();
            let image = images.get(&handle).unwrap();
            let pixels = image.data.as_ref().unwrap();
            assert!(pixels.chunks_exact(4).any(|pixel| pixel[3] > 0));
            for y in *top..*bottom {
                for x in *left..*right {
                    let alpha = pixels[((y * image.width() + x) * 4 + 3) as usize];
                    assert_eq!(alpha, 0, "frame covers live content at {x},{y}");
                }
            }
        }
    }
}

pub(crate) fn load_fdid_texture(
    fdid: u32,
    images: &mut Assets<Image>,
) -> Result<Handle<Image>, String> {
    let path = crate::asset::asset_cache::texture(fdid)
        .ok_or_else(|| format!("Nameplate atlas {fdid} unavailable in local CASC"))?;
    let mut image = crate::asset::blp::load_blp_to_image(&path)
        .map_err(|error| format!("Nameplate atlas {fdid}: {error}"))?;
    image.sampler = ImageSampler::linear();
    Ok(images.add(image))
}
