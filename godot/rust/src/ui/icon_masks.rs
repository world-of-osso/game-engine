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

#[derive(Clone, Copy, PartialEq, Eq, Hash)]
enum Mask {
    Round,
    AuctionItem,
    PerksProduct,
    Glyph,
    TalentCircle,
    TalentChoice,
    TalentChoiceLeft,
    TalentChoiceRight,
    TalentChoiceFlyout,
}

#[derive(Default)]
pub struct IconMasks {
    /// Keyed by mask, FDID and the bits of its normalized crop.
    icons: HashMap<(Mask, u32, [u32; 4]), DynamicTextureId>,
    mask_alpha: Option<GrayImage>,
    perks_buttons: HashMap<(u8, &'static str), DynamicTextureId>,
}

impl IconMasks {
    /// Swap every round icon's (`is_round_icon`) and header glyph's FDID source for its
    /// masked image. Failures never fall back to the unmasked art.
    pub fn apply(&mut self, registry: &mut FrameRegistry) {
        self.apply_perks_template(registry);
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

    fn apply_perks_template(&mut self, registry: &mut FrameRegistry) {
        // ToyDetails.xml justifyV=TOP; move into RSX when it exposes vertical justification.
        if let Some(id) = registry.get_by_name("PerksProgramToyDescription")
            && let Some(frame) = registry.get_mut(id)
            && let Some(WidgetData::FontString(text)) = &mut frame.widget_data
        {
            text.justify_v = ui_toolkit::widgets::font_string::JustifyV::Top;
        }
        let names = [
            "PerksProgramLeaveButton",
            "PerksProgramPurchaseButton",
            "PerksProgramRefundButton",
        ];
        if !names
            .iter()
            .any(|name| registry.get_by_name(name).is_some())
        {
            return;
        }
        let skin = match ui_toolkit::atlas::thread_skin() {
            ui_toolkit::atlas::ActiveSkin::Modern => 0,
            ui_toolkit::atlas::ActiveSkin::Forever => 1,
        };
        let mut textures = Vec::new();
        for suffix in ["", "-Pressed", "-Disabled"] {
            let key = (skin, suffix);
            let source = if let Some(id) = self.perks_buttons.get(&key) {
                Ok(*id)
            } else {
                read_perks_button(suffix).and_then(|image| {
                    registry
                        .create_dynamic_texture(image.width(), image.height(), image.into_raw())
                        .map_err(str::to_owned)
                })
            };
            match source {
                Ok(id) => {
                    self.perks_buttons.insert(key, id);
                    textures.push(TextureSource::Dynamic(id));
                }
                Err(error) => {
                    godot_error!("Trading Post shared button art: {error}");
                    return;
                }
            }
        }
        for name in names {
            if let Some(id) = registry.get_by_name(name)
                && let Some(frame) = registry.get_mut(id)
                && let Some(WidgetData::Button(button)) = &mut frame.widget_data
            {
                button.normal_texture = Some(textures[0].clone());
                button.pushed_texture = Some(textures[1].clone());
                button.disabled_texture = Some(textures[2].clone());
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
            Mask::PerksProduct
            | Mask::TalentCircle
            | Mask::TalentChoice
            | Mask::TalentChoiceLeft
            | Mask::TalentChoiceRight
            | Mask::TalentChoiceFlyout => compose_masked_icon(source, &load_atlas_mask(mask)?),
            Mask::Round | Mask::AuctionItem => {
                let round = match &mut self.mask_alpha {
                    Some(round) => round,
                    empty => empty.insert(mask_alpha(&load_rgba(PORTRAIT_MASK_FDID, "mask")?)),
                };
                if mask == Mask::AuctionItem {
                    compose_auction_item_icon(source, round)
                } else {
                    compose_masked_icon(source, round)
                }
            }
        };
        let id =
            registry.create_dynamic_texture(masked.width(), masked.height(), masked.into_raw())?;
        self.icons.insert(key, id);
        Ok(id)
    }
}

/// SharedButtonLargeTemplate -> BigRedThreeSliceButtonTemplate; ThreeSliceButtonMixin UpdateScale.
fn read_perks_button(suffix: &str) -> Result<RgbaImage, String> {
    let names = [
        format!("128-RedButton-Left{suffix}"),
        format!("_128-RedButton-Center{suffix}"),
        format!("128-RedButton-Right{suffix}"),
    ];
    let pieces = names
        .iter()
        .map(|name| {
            let region = ui_toolkit::atlas::get_region(name)
                .ok_or_else(|| format!("missing shared button atlas {name}"))?;
            let ui_toolkit::atlas::AtlasSource::FileDataId(fdid) = region.source else {
                return Err(format!("shared button atlas {name} has no FDID"));
            };
            Ok(crop_normalized(
                load_rgba(fdid, "shared button")?,
                [region.left, region.right, region.top, region.bottom],
            ))
        })
        .collect::<Result<Vec<_>, String>>()?;
    // Retail atlas geometry fixes cap widths in both skins; Forever only changes pixels.
    let scale: f32 = 46. / 128.;
    let widths = [
        (114. * scale).round() as u32,
        180 - (114. * scale).round() as u32 - (292. * scale).round() as u32,
        (292. * scale).round() as u32,
    ];
    let mut result = RgbaImage::new(180, 46);
    let mut x = 0;
    for (piece, width) in pieces.iter().zip(widths) {
        let piece =
            image::imageops::resize(piece, width, 46, image::imageops::FilterType::Triangle);
        image::imageops::overlay(&mut result, &piece, x, 0);
        x += i64::from(width);
    }
    Ok(result)
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
                | "ProfessionsPortrait"
                | "CooldownSettingsPortrait"
                | "ProfessionsOutputIcon"
        );
    let talent = name.starts_with("TalentNode") && name.ends_with("RoundIcon");
    creation || bag || portrait || talent
}

fn load_atlas_mask(mask: Mask) -> Result<GrayImage, String> {
    let atlas = match mask {
        Mask::PerksProduct => "UI-HUD-ActionBar-IconFrame-Mask",
        Mask::TalentCircle => "talents-node-circle-mask",
        Mask::TalentChoiceLeft => "talents-node-choice-mask-half",
        Mask::TalentChoice | Mask::TalentChoiceRight => "talents-node-choice-mask",
        Mask::TalentChoiceFlyout => "talents-node-choiceflyout-mask",
        _ => unreachable!("atlas mask only"),
    };
    let region =
        ui_toolkit::atlas::get_region(atlas).ok_or_else(|| format!("missing icon mask {atlas}"))?;
    let ui_toolkit::atlas::AtlasSource::FileDataId(fdid) = region.source else {
        return Err(format!("icon mask {atlas} has no FDID"));
    };
    let crop = [region.left, region.right, region.top, region.bottom];
    let mut image = crop_normalized(load_rgba(fdid, "icon mask")?, crop);
    if mask == Mask::TalentChoiceRight {
        image = image::imageops::rotate180(&image);
    }
    Ok(mask_alpha(&image))
}
fn mask_of(name: &str) -> Option<Mask> {
    let talent_circle = name.starts_with("TalentNode") && name.ends_with("RoundIcon");
    let profession_circle = (name.starts_with("ProfessionSpecNode") && name.ends_with("Icon"))
        || name == "ProfessionSpecDetailedIcon";
    let perks = name == "PerksProgramFrozenIcon"
        || name
            .strip_prefix("PerksProgramIcon")
            .is_some_and(|id| id.parse::<u32>().is_ok());
    if perks {
        Some(Mask::PerksProduct)
    } else if talent_circle || profession_circle {
        Some(Mask::TalentCircle)
    } else if name.ends_with("ChoiceLeftIcon") {
        Some(Mask::TalentChoiceLeft)
    } else if name.ends_with("ChoiceRightIcon") {
        Some(Mask::TalentChoiceRight)
    } else if name.starts_with("TalentNode") && name.ends_with("ChoiceIcon") {
        Some(Mask::TalentChoice)
    } else if name.starts_with("TalentChoiceEntry") && name.ends_with("Icon") {
        Some(Mask::TalentChoiceFlyout)
    } else if name == "AuctionHouseFrameItemBuyFrameItemDisplayItemButtonIcon" {
        Some(Mask::AuctionItem)
    } else if is_round_icon(name) {
        Some(Mask::Round)
    } else if FOREVER_CHAT_HEADER_ICONS.contains(&name) {
        Some(Mask::Glyph)
    } else {
        None
    }
}

/// CircularGiantItemButtonTemplate:46px icon,42px mask inset2 (ItemButtonTemplate.xml:23-41).
fn compose_auction_item_icon(source: RgbaImage, round: &GrayImage) -> RgbaImage {
    use image::imageops::{FilterType, replace, resize};
    let source = resize(&source, 46, 46, FilterType::Triangle);
    let circle = resize(round, 42, 42, FilterType::Triangle);
    let mut mask = GrayImage::new(46, 46);
    replace(&mut mask, &circle, 2, 2);
    compose_masked_icon(source, &mask)
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
    let path = game_engine_ui_model::paths::resolve_data_path(format!("textures/{fdid}.blp"));
    let bytes =
        std::fs::read(&path).map_err(|error| format!("{role} {}: {error}", path.display()))?;
    let rgba = game_engine_core::blp::decode_rgba(&bytes)
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

    #[test]
    fn trading_post_shared_large_buttons_have_distinct_source_states() {
        game_engine_ui_model::paths::set_data_root(
            std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../data"),
        )
        .unwrap();
        ui_toolkit::atlas::set_thread_skin(ui_toolkit::atlas::ActiveSkin::Modern);
        let normal = super::read_perks_button("").unwrap();
        let pressed = super::read_perks_button("-Pressed").unwrap();
        let disabled = super::read_perks_button("-Disabled").unwrap();
        assert_eq!(normal.dimensions(), (180, 46));
        assert_eq!(pressed.dimensions(), (180, 46));
        assert_eq!(disabled.dimensions(), (180, 46));
        assert_ne!(normal.as_raw(), pressed.as_raw());
        assert_ne!(normal.as_raw(), disabled.as_raw());
    }

    #[test]
    fn trading_post_icons_use_actionbar_mask_in_both_skins() {
        use ui_toolkit::atlas::{ActiveSkin, set_thread_skin};
        game_engine_ui_model::paths::set_data_root(
            std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../data"),
        )
        .unwrap();
        for skin in [ActiveSkin::Modern, ActiveSkin::Forever] {
            set_thread_skin(skin);
            let mut registry = FrameRegistry::new(1920., 1080.);
            Screen::new(|_| ui_toolkit::rsx! {
                texture { name:"PerksProgramIcon4", texture_fdid:135729, width:32.0, height:32.0 }
                texture { name:"PerksProgramFrozenIcon", texture_fdid:135729, width:32.0, height:32.0 }
            }).sync(&SharedContext::new(), &mut registry);
            IconMasks::default().apply(&mut registry);
            for name in ["PerksProgramIcon4", "PerksProgramFrozenIcon"] {
                let frame = registry.get(registry.get_by_name(name).unwrap()).unwrap();
                let Some(WidgetData::Texture(icon)) = &frame.widget_data else {
                    panic!("icon");
                };
                let TextureSource::Dynamic(id) = icon.source else {
                    panic!("Trading Post icon must use its authored mask");
                };
                let pixels = registry.dynamic_texture(id).unwrap();
                let rgba =
                    RgbaImage::from_raw(pixels.width, pixels.height, pixels.rgba8.clone()).unwrap();
                assert_eq!(rgba.get_pixel(0, 0)[3], 0, "masked corner {skin:?}");
                assert!(
                    rgba.get_pixel(rgba.width() / 2, rgba.height() / 2)[3] > 200,
                    "opaque centre {skin:?}"
                );
            }
        }
    }

    #[test]
    fn trading_post_missing_icon_source_never_renders_a_substitute() {
        game_engine_ui_model::paths::set_data_root(
            std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../data"),
        )
        .unwrap();
        let mut registry = FrameRegistry::new(1920., 1080.);
        Screen::new(|_| ui_toolkit::rsx! {
            texture { name:"PerksProgramIcon4", texture_fdid:4294967295u32, width:32.0, height:32.0 }
        }).sync(&SharedContext::new(), &mut registry);
        IconMasks::default().apply(&mut registry);
        let frame = registry
            .get(registry.get_by_name("PerksProgramIcon4").unwrap())
            .unwrap();
        let Some(WidgetData::Texture(icon)) = &frame.widget_data else {
            panic!("icon");
        };
        assert!(matches!(icon.source, TextureSource::None));
    }

    #[test]
    fn professions_specialization_icons_cut_corners_with_authored_circle_mask() {
        use game_engine_ui_model::professions_frame::{
            ProfessionPage, ProfessionView, professions_screen,
        };
        use shared::protocol::*;
        game_engine_ui_model::paths::set_data_root(
            std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../data"),
        )
        .unwrap();
        let unlock = ProfessionSpecNodeSnapshot {
            node_id: 28546,
            entry_id: 36338,
            name: "Leather Armor Crafting".into(),
            description: String::new(),
            icon: 4635265,
            spell_id: 0,
            rank: 1,
            granted_rank: 0,
            max_rank: 1,
            pos_x: 0,
            pos_y: 0,
            kind: 1,
            available: false,
            unavailable_reason: None,
            costs: vec![],
            path: None,
            perk: None,
        };
        let mut knowledge = unlock.clone();
        knowledge.entry_id = 36327;
        knowledge.rank = 0;
        knowledge.max_rank = 30;
        knowledge.available = true;
        let mut view = ProfessionView {
            page: ProfessionPage::Specializations,
            profession_name: "Leatherworking".into(),
            ..Default::default()
        };
        view.book.visible = true;
        view.specializations.selected_tree = Some(350);
        view.specializations.selected_node = Some(28546);
        view.specializations.snapshot = Some(ProfessionSpecializationsSnapshot {
            skill_line: 2830,
            knowledge_currency_id: 2025,
            knowledge_available: 1,
            currencies: vec![ProfessionSpecCurrencySnapshot {
                trait_currency_id: 9,
                currency_id: 2025,
                available: 1,
            }],
            trees: vec![ProfessionSpecTreeSnapshot {
                tree_id: 350,
                name: "Leather Armor Crafting".into(),
                root_node_id: 28546,
                unlocked: true,
                nodes: vec![unlock, knowledge],
                edges: vec![],
            }],
        });
        let mut shared = SharedContext::new();
        shared.insert(view);
        let mut registry = FrameRegistry::new(1280.0, 900.0);
        Screen::new(professions_screen).sync(&shared, &mut registry);
        IconMasks::default().apply(&mut registry);
        for name in ["ProfessionSpecNode28546Icon", "ProfessionSpecDetailedIcon"] {
            let frame = registry.get(registry.get_by_name(name).unwrap()).unwrap();
            let Some(WidgetData::Texture(rendered)) = &frame.widget_data else {
                panic!("path icon texture")
            };
            let rgba = match rendered.source {
                TextureSource::FileDataId(fdid) => texture(fdid),
                TextureSource::Dynamic(id) => {
                    let pixels = registry.dynamic_texture(id).unwrap();
                    RgbaImage::from_raw(pixels.width, pixels.height, pixels.rgba8.clone()).unwrap()
                }
                _ => panic!("path icon must retain its authored pixels"),
            };
            assert_eq!(rgba.get_pixel(0, 0)[3], 0, "{name} top-left corner");
            assert_eq!(
                rgba.get_pixel(rgba.width() / 10, rgba.height() / 10)[3],
                0,
                "{name} diagonal outside circle"
            );
            assert_eq!(
                rgba.get_pixel(rgba.width() - 1, rgba.height() - 1)[3],
                0,
                "{name} bottom-right corner"
            );
            assert!(
                rgba.get_pixel(rgba.width() / 2, rgba.height() / 2)[3] > 200,
                "{name} opaque centre"
            );
        }
    }

    #[test]
    fn talent_tree3_choice_icons_use_authored_half_over_full_octagon_masks() {
        game_engine_ui_model::paths::set_data_root(
            std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../data"),
        )
        .unwrap();
        let mut registry = FrameRegistry::new(1920.0, 1080.0);
        Screen::new(|_| ui_toolkit::rsx! {
            texture { name:"TalentNode62087Entry80143ChoiceLeftIcon", texture_fdid:135729, width:36.0, height:36.0 }
            texture { name:"TalentNode62087Entry134199ChoiceRightIcon", texture_fdid:135729, width:36.0, height:36.0 }
        }).sync(&SharedContext::new(), &mut registry);
        IconMasks::default().apply(&mut registry);
        let images: Vec<_> = [
            "TalentNode62087Entry80143ChoiceLeftIcon",
            "TalentNode62087Entry134199ChoiceRightIcon",
        ]
        .into_iter()
        .map(|name| {
            let frame = registry.get(registry.get_by_name(name).unwrap()).unwrap();
            let Some(WidgetData::Texture(texture)) = &frame.widget_data else {
                panic!("choice icon")
            };
            let TextureSource::Dynamic(id) = texture.source else {
                panic!("choice icon must be masked")
            };
            let image = registry.dynamic_texture(id).unwrap();
            RgbaImage::from_raw(image.width, image.height, image.rgba8.clone()).unwrap()
        })
        .collect();
        for image in &images {
            assert_eq!(image.get_pixel(0, 0)[3], 0);
        }
        let (w, h) = images[0].dimensions();
        assert!(images[0].get_pixel(w / 4, h / 2)[3] > 200);
        assert_eq!(images[0].get_pixel(3 * w / 4, h / 2)[3], 0);
        // Retail Icon2Mask is a full octagon below IconSplitMask's left half.
        assert!(images[1].get_pixel(w / 4, h / 2)[3] > 200);
        assert!(images[1].get_pixel(3 * w / 4, h / 2)[3] > 200);
        // Preserve decoded authentic art for the visual audit, not generated art.
        if let Ok(directory) = std::env::var("TALENT_TREE3_DECODE_DIR") {
            for fdid in [8116735, 4641164, 4633071, 4638369] {
                let image = texture(fdid);
                let header = format!(
                    "P7\nWIDTH {}\nHEIGHT {}\nDEPTH 4\nMAXVAL 255\nTUPLTYPE RGB_ALPHA\nENDHDR\n",
                    image.width(),
                    image.height()
                );
                let bytes = [header.as_bytes(), image.as_raw()].concat();
                std::fs::write(
                    std::path::Path::new(&directory).join(format!("atlas-{fdid}.pam")),
                    bytes,
                )
                .unwrap();
            }
        }
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
