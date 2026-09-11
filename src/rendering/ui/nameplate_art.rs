//! Reference-derived nameplate skins; dimensions are unscaled screenshot pixels.
use bevy::image::ImageSampler;
use bevy::prelude::*;
use bevy::text::Font;

pub(crate) const NAMEPLATE_SCALE: f32 = 0.5;
pub(crate) const BAR_PIXEL_WIDTH: f32 = 376.0 * NAMEPLATE_SCALE;
pub(crate) const NAME_FONT_SIZE: f32 = 26.0 * NAMEPLATE_SCALE;
pub(crate) const CAST_FONT_SIZE: f32 = 20.0 * NAMEPLATE_SCALE;
pub(crate) const CAST_FILL_RECT: Rect = Rect::new(268.0, 124.0, 477.0, 135.0);
pub(crate) const CAST_BACKGROUND_RECT: Rect = Rect::new(57.0, 85.0, 266.0, 96.0);

#[derive(Clone)]
pub(crate) struct NameplateArt {
    pub health_fill_thin: Handle<Image>,
    pub health_fill_thick: Handle<Image>,
    pub health_thick: Handle<Image>,
    pub health_thin: Handle<Image>,
    pub casting: Handle<Image>,
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
            load_skin(include_bytes!("nameplate_skins/health-fill.png"), images)?;
        let health_fill_thick = load_skin(
            include_bytes!("nameplate_skins/health-fill-thick.png"),
            images,
        )?;
        let health_thick = load_skin(include_bytes!("nameplate_skins/health-thick.png"), images)?;
        let health_thin = load_skin(include_bytes!("nameplate_skins/health-thin.png"), images)?;
        let casting = load_atlas(4505182, images)?;
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
            cast_thick: image.clone(),
            cast_thin: image,
            font: fonts.add(Font::from_bytes(bevy::text::DEFAULT_FONT_DATA.to_vec())),
        }))
    }
}

fn load_skin(bytes: &[u8], images: &mut Assets<Image>) -> Result<Handle<Image>, String> {
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
    Ok(images.add(image))
}

#[cfg(test)]
mod tests {
    use super::*;

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

fn load_atlas(fdid: u32, images: &mut Assets<Image>) -> Result<Handle<Image>, String> {
    let path = crate::asset::asset_cache::texture(fdid)
        .ok_or_else(|| format!("Nameplate atlas {fdid} unavailable in local CASC"))?;
    let mut image = crate::asset::blp::load_blp_to_image(&path)
        .map_err(|error| format!("Nameplate atlas {fdid}: {error}"))?;
    image.sampler = ImageSampler::linear();
    Ok(images.add(image))
}
