//! Authored nameplate artwork; all dimensions are reference screenshot pixels.
use bevy::image::ImageSampler;
use bevy::prelude::*;
use bevy::text::Font;

pub(crate) const NAMEPLATE_SCALE: f32 = 0.5;
pub(crate) const BAR_PIXEL_WIDTH: f32 = 384.0 * NAMEPLATE_SCALE;
pub(crate) const NAME_FONT_SIZE: f32 = 26.0 * NAMEPLATE_SCALE;
pub(crate) const CAST_FONT_SIZE: f32 = 20.0 * NAMEPLATE_SCALE;
pub(crate) const HEALTH_FILL_RECT: Rect = Rect::new(89.0, 22.0, 213.0, 32.0);
pub(crate) const HEALTH_BACKGROUND_RECT: Rect = Rect::new(89.0, 1.0, 221.0, 20.0);
pub(crate) const CAST_FILL_RECT: Rect = Rect::new(268.0, 124.0, 477.0, 135.0);
pub(crate) const CAST_BACKGROUND_RECT: Rect = Rect::new(57.0, 85.0, 266.0, 96.0);
pub(crate) const CAST_INDICATOR_RECT: Rect = Rect::new(1.0, 63.0, 265.0, 79.0);
pub(crate) const CAST_PIP_RECT: Rect = Rect::new(1.0, 151.0, 7.0, 181.0);

#[derive(Clone)]
pub(crate) struct NameplateArt {
    pub health: Handle<Image>,
    pub casting: Handle<Image>,
    pub indicator: Handle<Image>,
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
        let health = load_atlas(6704514, images)?;
        let casting = load_atlas(4505182, images)?;
        let indicator = load_atlas(7241122, images)?;
        let path = "data/fonts/FRIZQT__.TTF";
        let bytes =
            std::fs::read(path).map_err(|error| format!("Nameplate font {path}: {error}"))?;
        ab_glyph::FontRef::try_from_slice(&bytes)
            .map_err(|error| format!("Nameplate font {path}: {error}"))?;
        let art = NameplateArt {
            health,
            casting,
            indicator,
            font: fonts.add(Font::from_bytes(bytes)),
        };
        self.0 = Some(art.clone());
        Ok(art)
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
            health: image.clone(),
            casting: image.clone(),
            indicator: image,
            font: fonts.add(Font::from_bytes(bevy::text::DEFAULT_FONT_DATA.to_vec())),
        }))
    }
}

fn load_atlas(fdid: u32, images: &mut Assets<Image>) -> Result<Handle<Image>, String> {
    let path = crate::asset::asset_cache::texture(fdid)
        .ok_or_else(|| format!("Nameplate atlas {fdid} unavailable in local CASC"))?;
    let mut image = crate::asset::blp::load_blp_to_image(&path)
        .map_err(|error| format!("Nameplate atlas {fdid}: {error}"))?;
    image.sampler = ImageSampler::linear();
    Ok(images.add(image))
}
