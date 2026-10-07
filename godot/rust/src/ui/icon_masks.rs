//! Round icon masking with `TempPortraitAlphaMask`: character-creation race/class icons
//! (`src/scenes/char_create/icon_masks.rs`) and the bag bar's `CircularItemButtonTemplate`
//! bag icons (`CircleMask`, ItemButtonTemplate.xml:5-21) and container portraits; and
//! the Forever chat header glyphs, drawn as white masks the vertex colour tints.

use std::collections::HashMap;

use game_engine_core::character_creation_icon_mask_data::{
    PORTRAIT_MASK_FDID, compose_masked_icon, mask_alpha,
};
use game_engine_ui_model::chat_frame_component::FOREVER_CHAT_HEADER_ICONS;
use game_engine_ui_model::flare_panel::flare_glyph_mask;
use godot::global::godot_error;
use image::{GrayImage, RgbaImage};
use ui_toolkit::frame::WidgetData;
use ui_toolkit::registry::FrameRegistry;
use ui_toolkit::widgets::texture::{DynamicTextureId, TextureSource};

use super::assets;

#[derive(Clone, Copy, PartialEq, Eq, Hash)]
enum Mask {
    Round,
    Glyph,
}

#[derive(Default)]
pub struct IconMasks {
    /// Keyed by mask, FDID and the bits of its normalized crop.
    icons: HashMap<(Mask, u32, [u32; 4]), DynamicTextureId>,
    mask_alpha: Option<GrayImage>,
}

impl IconMasks {
    /// Swap every round icon's (`is_round_icon`) and header glyph's FDID source for its
    /// masked image. Failures never fall back to the unmasked art.
    pub fn apply(&mut self, registry: &mut FrameRegistry) {
        let pending: Vec<_> = registry
            .frames_iter()
            .filter_map(|frame| {
                let mask = mask_of(frame.name.as_deref()?)?;
                let Some(WidgetData::Texture(texture)) = &frame.widget_data else {
                    return None;
                };
                let TextureSource::FileDataId(fdid) = texture.source else {
                    return None;
                };
                Some((frame.id, mask, fdid, texture.tex_coords))
            })
            .collect();
        for (id, mask, fdid, crop) in pending {
            // FDID 0 is no icon: an empty portrait ring (`SpellbookFrameState::portrait_fdid`).
            let source = match (fdid != 0).then(|| self.masked(mask, fdid, crop, registry)) {
                None => TextureSource::None,
                Some(Ok(texture)) => TextureSource::Dynamic(texture),
                Some(Err(error)) => {
                    godot_error!("Icon {fdid} cannot be masked: {error}");
                    TextureSource::None
                }
            };
            if let Some(frame) = registry.get_mut(id)
                && let Some(WidgetData::Texture(texture)) = &mut frame.widget_data
            {
                texture.source = source;
                // The masked image is already the cropped region.
                texture.tex_coords = [0.0, 1.0, 0.0, 1.0];
            }
        }
    }

    fn masked(
        &mut self,
        mask: Mask,
        fdid: u32,
        crop: [f32; 4],
        registry: &mut FrameRegistry,
    ) -> Result<DynamicTextureId, String> {
        let key = (mask, fdid, crop.map(f32::to_bits));
        if let Some(id) = self.icons.get(&key) {
            return Ok(*id);
        }
        let mut source = crop_normalized(load_rgba(fdid, "icon")?, crop);
        let masked = match mask {
            Mask::Glyph => {
                flare_glyph_mask(&mut source);
                source
            }
            Mask::Round => {
                let round = match &mut self.mask_alpha {
                    Some(round) => round,
                    empty => empty.insert(mask_alpha(&load_rgba(PORTRAIT_MASK_FDID, "mask")?)),
                };
                compose_masked_icon(source, round)
            }
        };
        let id =
            registry.create_dynamic_texture(masked.width(), masked.height(), masked.into_raw())?;
        self.icons.insert(key, id);
        Ok(id)
    }
}

/// `Race_*_Icon` / `Class_*_Icon` / `Form_*_Icon`, the bag bar's
/// `CharacterBag{n}SlotIconTexture` / `CharacterReagentBag0SlotIconTexture`, and the
/// `ContainerFrame{n}Portrait` / `SpellBookPortrait` window portraits
/// (`PortraitFrameBaseTemplate` `CircleMask`, SharedUIPanelTemplates.xml:564-572).
fn is_round_icon(name: &str) -> bool {
    let creation = name.ends_with("_Icon")
        && (name.starts_with("Race_") || name.starts_with("Class_") || name.starts_with("Form_"));
    let bag = name.ends_with("SlotIconTexture")
        && (name.starts_with("CharacterBag") || name.starts_with("CharacterReagentBag"));
    let portrait = (name.starts_with("ContainerFrame") && name.ends_with("Portrait"))
        || matches!(
            name,
            "SpellBookPortrait"
                | "MailFramePortrait"
                | "OpenMailFramePortrait"
                | "FlightMapPortrait"
        );
    creation || bag || portrait
}

fn mask_of(name: &str) -> Option<Mask> {
    if is_round_icon(name) {
        Some(Mask::Round)
    } else if FOREVER_CHAT_HEADER_ICONS.contains(&name) {
        Some(Mask::Glyph)
    } else {
        None
    }
}

/// The (left, right, top, bottom) normalized region of `image`.
fn crop_normalized(image: RgbaImage, crop: [f32; 4]) -> RgbaImage {
    if crop == [0.0, 1.0, 0.0, 1.0] {
        return image;
    }
    let (width, height) = (image.width() as f32, image.height() as f32);
    let x = (crop[0] * width).round() as u32;
    let y = (crop[2] * height).round() as u32;
    let w = ((crop[1] - crop[0]) * width).round() as u32;
    let h = ((crop[3] - crop[2]) * height).round() as u32;
    image::imageops::crop_imm(&image, x, y, w, h).to_image()
}

fn load_rgba(fdid: u32, role: &str) -> Result<RgbaImage, String> {
    let rgba = assets::decode_blp(&format!("data/textures/{fdid}.blp"))
        .map_err(|error| format!("{role} FDID {fdid}: {error}"))?;
    if rgba.width == 0 || rgba.height == 0 {
        return Err(format!("{role} FDID {fdid} has zero dimensions"));
    }
    RgbaImage::from_raw(rgba.width, rgba.height, rgba.pixels)
        .ok_or_else(|| format!("{role} FDID {fdid} has invalid RGBA size"))
}

#[cfg(test)]
mod tests {
    use game_engine_core::character_creation_icon_mask_data::{
        PORTRAIT_MASK_FDID, compose_masked_icon, mask_alpha,
    };
    use game_engine_ui_model::bag_frame_component::{
        BACKPACK_PORTRAIT, BagContainerState, BagFrameState, bag_frame_screen,
    };
    use game_engine_ui_model::spellbook_frame_component::{
        SpellbookFrameState, spellbook_frame_screen,
    };
    use image::RgbaImage;
    use ui_toolkit::frame::WidgetData;
    use ui_toolkit::registry::FrameRegistry;
    use ui_toolkit::screen::{Screen, SharedContext};
    use ui_toolkit::widgets::texture::TextureSource;

    use super::{IconMasks, is_round_icon};

    fn texture(fdid: u32) -> RgbaImage {
        let path = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join(format!("../../data/textures/{fdid}.blp"));
        let bytes = std::fs::read(&path).unwrap_or_else(|error| panic!("{path:?}: {error}"));
        let image = game_engine_core::blp::decode_rgba(&bytes).unwrap();
        RgbaImage::from_raw(image.width, image.height, image.pixels).unwrap()
    }

    /// The open backpack's portrait is its bag icon, rounded by `TempPortraitAlphaMask`:
    /// transparent in the corners, opaque in the middle.
    #[test]
    fn container_portrait_is_the_bag_icon_masked_round() {
        game_engine_ui_model::paths::set_data_root(
            std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../data"),
        )
        .unwrap();
        let mut ctx = SharedContext::new();
        ctx.insert(BagFrameState {
            bags: vec![BagContainerState {
                bag_index: 0,
                title: "Backpack".into(),
                portrait_fdid: BACKPACK_PORTRAIT,
                slots: Vec::new(),
                visible: true,
            }],
            ..Default::default()
        });
        let mut registry = FrameRegistry::new(1920.0, 1080.0);
        Screen::new(bag_frame_screen).sync(&ctx, &mut registry);
        let round: Vec<_> = registry
            .frames_iter()
            .filter(|frame| frame.name.as_deref().is_some_and(is_round_icon))
            .collect();
        assert_eq!(round.len(), 1, "one round icon: the portrait");
        let Some(WidgetData::Texture(portrait)) = &round[0].widget_data else {
            panic!("portrait is not a texture");
        };
        let TextureSource::FileDataId(fdid) = portrait.source else {
            panic!("portrait has no icon");
        };
        let masked = compose_masked_icon(texture(fdid), &mask_alpha(&texture(PORTRAIT_MASK_FDID)));
        let (w, h) = masked.dimensions();
        assert_eq!(masked.get_pixel(0, 0)[3], 0, "corner cut away");
        assert_eq!(masked.get_pixel(w - 1, h - 1)[3], 0, "corner cut away");
        assert!(masked.get_pixel(w / 2, h / 2)[3] > 200, "centre kept");
    }

    #[test]
    fn capturepolish_flight_map_portrait_keeps_art_and_cuts_corners() {
        use game_engine_ui_model::quest_art::{window_portrait_slot, window_portrait_texture};
        game_engine_ui_model::paths::set_data_root(
            std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../data"),
        )
        .unwrap();
        let mut registry = FrameRegistry::new(1920.0, 1080.0);
        Screen::new(|_| {
            window_portrait_texture(&window_portrait_slot("FlightMapPortrait"), 618_976)
        })
        .sync(&SharedContext::new(), &mut registry);
        let portrait = registry
            .get(registry.get_by_name("FlightMapPortrait").unwrap())
            .unwrap();
        let Some(WidgetData::Texture(source)) = &portrait.widget_data else {
            panic!()
        };
        let TextureSource::FileDataId(fdid) = source.source else {
            panic!("portrait has no authored art")
        };
        let rgba = compose_masked_icon(texture(fdid), &mask_alpha(&texture(PORTRAIT_MASK_FDID)));
        let (width, height) = rgba.dimensions();
        assert_eq!(rgba.get_pixel(0, 0)[3], 0);
        assert_eq!(rgba.get_pixel(width - 1, height - 1)[3], 0);
        let middle = rgba.get_pixel(width / 2, height / 2);
        assert!(middle[3] > 200);
        assert!(
            middle.0[0..3].iter().any(|channel| *channel > 30),
            "authored art is not black"
        );
    }

    /// Without a specialization the spellbook portrait's FDID is 0, an empty ring
    /// (`SpellbookFrameState::portrait_fdid`): nothing to mask, not a missing file.
    #[test]
    fn spellbook_portrait_without_a_specialization_draws_no_icon() {
        game_engine_ui_model::paths::set_data_root(
            std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../data"),
        )
        .unwrap();
        let mut ctx = SharedContext::new();
        ctx.insert(SpellbookFrameState {
            viewport: [1920.0, 1080.0],
            ..Default::default()
        });
        let mut registry = FrameRegistry::new(1920.0, 1080.0);
        Screen::new(spellbook_frame_screen).sync(&ctx, &mut registry);
        IconMasks::default().apply(&mut registry);
        let portrait = registry
            .frames_iter()
            .find(|frame| frame.name.as_deref() == Some("SpellBookPortrait"))
            .expect("spellbook portrait");
        let Some(WidgetData::Texture(texture)) = &portrait.widget_data else {
            panic!("portrait is not a texture");
        };
        assert!(matches!(texture.source, TextureSource::None));
    }
}
