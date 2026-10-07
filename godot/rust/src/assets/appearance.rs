//! Prepare authored NPC replacement textures and geosets before allocating visual nodes.

use std::{
    collections::{HashMap, HashSet},
    path::Path,
};

use game_engine_core::{
    asset::m2_texture,
    blp,
    char_texture_data::{CharTextureData, CompositedModelTextures},
    customization_data::CustomizationDb,
    npc_appearance_assets::{load_compositor, load_customization_db},
    npc_appearance_data::{AuthoredNpcAppearance, query_authored_npc_appearance},
    npc_appearance_selection_data::{NpcSelections, select_npc_choices, select_npc_type6_texture},
};
use godot::{
    classes::{Image, ImageTexture, image},
    prelude::*,
};
use osso_asset_resolver::{AssetResolverConfig, CascListfileResolver};
use rusqlite::{Connection, OpenFlags};

use crate::equipment_appearance_data::ResolvedEquipmentAppearance;

pub(super) type TexturePixels = (Vec<u8>, u32, u32);

#[derive(Default)]
pub(crate) struct NpcAppearances {
    profiles: Option<Connection>,
    customization: Option<CustomizationDb>,
    compositor: Option<CharTextureData>,
}

pub(crate) struct PreparedAppearance {
    pub(super) source: &'static str,
    pub(super) textures: HashMap<u32, Gd<ImageTexture>>,
    pub(super) selected_geosets: Vec<(u16, u16)>,
    pub(super) authored_geosets: Vec<(u16, u16)>,
    pub(super) equipment_geosets: Vec<(u16, u16)>,
    pub(super) hidden_geoset_ids: HashSet<u16>,
}

/// A `PreparedAppearance` with its replacement textures still pixels, so a worker can
/// prepare it; `into_prepared` makes the textures on the main thread.
pub(crate) struct AppearanceParts {
    pub(super) source: &'static str,
    pub(super) textures: HashMap<u32, MipChain>,
    pub(super) selected_geosets: Vec<(u16, u16)>,
    pub(super) authored_geosets: Vec<(u16, u16)>,
    pub(super) equipment_geosets: Vec<(u16, u16)>,
    pub(super) hidden_geoset_ids: HashSet<u16>,
}

impl AppearanceParts {
    pub(crate) fn into_prepared(self) -> Result<PreparedAppearance, String> {
        let textures = self
            .textures
            .into_iter()
            .map(|(kind, pixels)| make_texture(pixels).map(|texture| (kind, texture)))
            .collect::<Result<_, _>>()?;
        Ok(PreparedAppearance {
            source: self.source,
            textures,
            selected_geosets: self.selected_geosets,
            authored_geosets: self.authored_geosets,
            equipment_geosets: self.equipment_geosets,
            hidden_geoset_ids: self.hidden_geoset_ids,
        })
    }
}

/// An authored NPC body with its display's armor (`NPCModelItemSlotDisplayInfo`).
pub(crate) struct PreparedNpc {
    pub(crate) appearance: AppearanceParts,
    /// Item models, textures and geosets of the armor; its textures are in the bake.
    pub(crate) armor: ResolvedEquipmentAppearance,
    pub(crate) race: u8,
    pub(crate) sex: u8,
}

impl NpcAppearances {
    /// The display's authored body, dressed in the armor `resolve_armor` resolves for its
    /// race and sex; `None` for a display without a `CreatureDisplayInfoExtra`.
    pub(crate) fn prepare(
        &mut self,
        data_root: &Path,
        display_id: u32,
        resolve_armor: impl FnOnce(
            &AuthoredNpcAppearance,
        ) -> Result<ResolvedEquipmentAppearance, String>,
    ) -> Result<Option<PreparedNpc>, String> {
        let Some(appearance) = self.query_appearance(data_root, display_id)? else {
            return Ok(None);
        };
        let armor = resolve_armor(&appearance)?;
        let (mut selected, layout_id) = self.select_choices_and_layout(data_root, &appearance)?;
        let db = self
            .customization
            .as_ref()
            .expect("loaded customization db");
        hide_armor_geoset_groups(&mut selected.geosets, &armor, db, &appearance);
        if self.compositor.is_none() {
            self.compositor = Some(load_compositor(data_root)?);
        }
        let compositor = self.compositor.as_ref().expect("loaded compositor");
        let resolver = CascListfileResolver::new(
            AssetResolverConfig::new()
                .with_data_root(data_root)
                .with_shared_data_root(data_root),
        );
        let textures = compose_replacement_textures(
            compositor,
            &appearance,
            &selected,
            layout_id,
            &resolver,
            data_root,
            display_id,
        )?;
        Ok(Some(PreparedNpc {
            appearance: AppearanceParts {
                source: "NPC",
                textures: mip_chains(textures),
                selected_geosets: selected.geosets,
                authored_geosets: appearance.geosets,
                equipment_geosets: armor.outfit.geoset_overrides.clone(),
                hidden_geoset_ids: armor.hidden_character_geoset_ids.clone(),
            },
            armor,
            race: appearance.race,
            sex: appearance.sex,
        }))
    }

    fn query_appearance(
        &mut self,
        data_root: &Path,
        display_id: u32,
    ) -> Result<Option<AuthoredNpcAppearance>, String> {
        if self.profiles.is_none() {
            let path = data_root.join("cache/npc_appearance.sqlite");
            self.profiles = Some(
                Connection::open_with_flags(&path, OpenFlags::SQLITE_OPEN_READ_ONLY).map_err(
                    |error| {
                        format!(
                            "open authored NPC appearance cache {} for display {display_id}: {error}",
                            path.display()
                        )
                    },
                )?,
            );
        }
        query_authored_npc_appearance(
            self.profiles.as_ref().expect("opened NPC appearance cache"),
            display_id,
        )
    }

    fn select_choices_and_layout(
        &mut self,
        data_root: &Path,
        appearance: &AuthoredNpcAppearance,
    ) -> Result<(NpcSelections, u32), String> {
        if self.customization.is_none() {
            self.customization = Some(load_customization_db(data_root)?);
        }
        let db = self
            .customization
            .as_ref()
            .expect("loaded customization db");
        let selected = select_npc_choices(appearance, db)?;
        let layout_id = db
            .layout_id(appearance.race, appearance.sex)
            .ok_or_else(|| {
                format!(
                    "missing texture layout for race {} sex {}",
                    appearance.race, appearance.sex
                )
            })?;
        Ok((selected, layout_id))
    }
}

/// Replace each customization geoset group the armor hides: hair (0) by the scalp,
/// others by variant 1 (Bevy `apply_hidden_geoset_groups`).
fn hide_armor_geoset_groups(
    geosets: &mut Vec<(u16, u16)>,
    armor: &ResolvedEquipmentAppearance,
    db: &CustomizationDb,
    appearance: &AuthoredNpcAppearance,
) {
    for &group in &armor.hidden_character_geoset_groups {
        geosets.retain(|(active, _)| *active != group);
        let variant = if group == 0 {
            db.scalp_fallback_hair_geoset(appearance.race, appearance.sex)
                .unwrap_or(1)
        } else {
            1
        };
        geosets.push((group, variant));
    }
}

fn compose_replacement_textures(
    compositor: &CharTextureData,
    appearance: &AuthoredNpcAppearance,
    selected: &NpcSelections,
    layout_id: u32,
    resolver: &CascListfileResolver,
    data_root: &Path,
    display_id: u32,
) -> Result<HashMap<u32, TexturePixels>, String> {
    let (composed, decoded) = load_and_compose_selected_pixels(
        compositor, selected, layout_id, resolver, data_root, display_id,
    )?;
    let body = match appearance.baked_texture_fdid {
        Some(fdid) => load_npc_texture(resolver, data_root, fdid)?,
        None => composed.body,
    };
    let mut textures = HashMap::from([(1, body)]);
    if let Some(type6) = select_npc_type6_texture(
        compositor.declares_hair(&selected.materials, layout_id),
        composed.hair,
        composed.head,
    )? {
        textures.insert(6, type6);
    }
    // Eyes (19) compose every selected layer on their own canvas: the Eyesight overlay
    // (target 44) over the eye colour (target 25).
    if let Some(pixels) =
        compositor.composite_texture_type(&selected.materials, layout_id, 19, |fdid| {
            decoded.get(&fdid).cloned()
        })
    {
        textures.insert(19, pixels);
    }
    Ok(textures)
}

fn load_and_compose_selected_pixels(
    compositor: &CharTextureData,
    selected: &NpcSelections,
    layout_id: u32,
    resolver: &CascListfileResolver,
    data_root: &Path,
    display_id: u32,
) -> Result<(CompositedModelTextures, HashMap<u32, TexturePixels>), String> {
    let layout = compositor
        .layout(layout_id)
        .ok_or_else(|| format!("cannot composite NPC texture layout {layout_id}"))?;
    let default_fdid = m2_texture::default_fdid_for_type(
        1,
        layout.width == 2048 && layout.height == 1024,
        &[0, 0, 0],
    )
    .ok_or_else(|| format!("missing default NPC body texture for layout {layout_id}"))?;
    let decoded =
        load_selected_and_default_pixels(selected, default_fdid, resolver, data_root, display_id)?;
    let composed = compositor
        .composite_model_textures_with(&selected.materials, &[], layout_id, default_fdid, |fdid| {
            decoded.get(&fdid).cloned()
        })
        .ok_or_else(|| format!("cannot composite NPC texture layout {layout_id}"))?;
    Ok((composed, decoded))
}

fn load_selected_and_default_pixels(
    selected: &NpcSelections,
    default_fdid: u32,
    resolver: &CascListfileResolver,
    data_root: &Path,
    display_id: u32,
) -> Result<HashMap<u32, TexturePixels>, String> {
    let mut decoded = HashMap::new();
    for fdid in selected
        .materials
        .iter()
        .map(|(_, fdid)| *fdid)
        .collect::<HashSet<_>>()
    {
        decoded.insert(fdid, load_npc_texture(resolver, data_root, fdid)?);
    }
    // The default atlas is optional in the original compositor, but failures must be visible.
    if !decoded.contains_key(&default_fdid) {
        match load_npc_texture(resolver, data_root, default_fdid) {
            Ok(pixels) => {
                decoded.insert(default_fdid, pixels);
            }
            Err(error) => eprintln!("NPC display {display_id}: {error}"),
        }
    }
    Ok(decoded)
}

fn load_npc_texture(
    resolver: &CascListfileResolver,
    data_root: &Path,
    fdid: u32,
) -> Result<TexturePixels, String> {
    load_appearance_texture(resolver, data_root, fdid, "NPC")
}

pub(super) fn load_appearance_texture(
    resolver: &CascListfileResolver,
    data_root: &Path,
    fdid: u32,
    source: &str,
) -> Result<TexturePixels, String> {
    let destination = data_root.join("textures").join(format!("{fdid}.blp"));
    let path = resolver.ensure_cached(fdid, &destination).ok_or_else(|| {
        format!(
            "missing {source} texture FDID {fdid} at {}",
            destination.display()
        )
    })?;
    let bytes = std::fs::read(&path).map_err(|error| format!("{}: {error}", path.display()))?;
    let rgba = blp::decode_rgba(&bytes).map_err(|error| {
        format!(
            "decode {source} texture FDID {fdid} at {}: {error}",
            path.display()
        )
    })?;
    Ok((rgba.pixels, rgba.width, rgba.height))
}

/// A composited character texture with its whole mip chain, as stock composes the atlas
/// (solarityclient composer.rs `compose`); without mips it aliases when minified.
fn make_texture(chain: MipChain) -> Result<Gd<ImageTexture>, String> {
    let MipChain {
        data,
        width,
        height,
    } = chain;
    let _span = crate::profile::span(|| format!("appearance.make_texture {width}x{height}"));
    let image_span = crate::profile::span(|| "appearance.make_texture.image".to_owned());
    let image = Image::create_from_data(
        width as i32,
        height as i32,
        true,
        image::Format::RGBA8,
        &PackedByteArray::from(data.as_slice()),
    )
    .ok_or_else(|| format!("Godot rejected {width}x{height} character texture"))?;
    drop(image_span);
    ImageTexture::create_from_image(&image).ok_or_else(|| "Godot rejected character texture".into())
}

/// An RGBA8 texture with its mipmaps, level 0 first and each level half the previous
/// down to 1x1, made on a worker so the main thread only uploads it.
pub(crate) struct MipChain {
    data: Vec<u8>,
    width: u32,
    height: u32,
}

pub(super) fn mip_chains(textures: HashMap<u32, TexturePixels>) -> HashMap<u32, MipChain> {
    textures
        .into_iter()
        .map(|(kind, pixels)| (kind, mip_chain(pixels)))
        .collect()
}

/// Godot's `Image::generate_mipmaps` for RGBA8 (`_generate_po2_mipmap` with
/// `average_4_uint8`, godot core/io/image.cpp): each texel of a level is the rounded mean of
/// the 2x2 block above it; a 1-texel-wide or -high level repeats its edge.
fn mip_chain((pixels, width, height): TexturePixels) -> MipChain {
    let mut data = pixels;
    let (mut level_start, mut w, mut h) = (0, width as usize, height as usize);
    while w > 1 || h > 1 {
        let (next_w, next_h) = ((w >> 1).max(1), (h >> 1).max(1));
        let right = if w == 1 { 0 } else { 4 };
        let down = if h == 1 { 0 } else { w * 4 };
        let next_start = data.len();
        data.reserve(next_w * next_h * 4);
        for y in 0..next_h {
            for x in 0..next_w {
                let up = level_start + y * 2 * down + x * 2 * right;
                for channel in 0..4 {
                    let texel = |offset: usize| u32::from(data[up + offset + channel]);
                    let sum = texel(0) + texel(right) + texel(down) + texel(down + right);
                    data.push(((sum + 2) >> 2) as u8);
                }
            }
        }
        (level_start, w, h) = (next_start, next_w, next_h);
    }
    MipChain {
        data,
        width,
        height,
    }
}

#[cfg(test)]
mod mip_chain_tests {
    use super::*;

    /// Each level averages 2x2 blocks with rounding, down to 1x1.
    #[test]
    fn levels_average_two_by_two_blocks_down_to_one_texel() {
        let texel = |value: u8| [value, 255 - value, value / 2, 255];
        let pixels: Vec<u8> = [
            10, 20, 30, 41, 50, 60, 70, 80, 90, 100, 110, 120, 130, 140, 150, 161,
        ]
        .into_iter()
        .flat_map(texel)
        .collect();
        let chain = mip_chain((pixels.clone(), 4, 4));
        assert_eq!((chain.width, chain.height), (4, 4));
        assert_eq!(chain.data.len(), (16 + 4 + 1) * 4);
        assert_eq!(&chain.data[..64], pixels.as_slice());
        // Level 1, top left: (10 + 20 + 50 + 60 + 2) >> 2 = 35 in red, 220 in green,
        // (5 + 10 + 25 + 30 + 2) >> 2 = 18 in blue.
        assert_eq!(&chain.data[64..68], &[35, 220, 18, 255]);
        // Top right block 30, 41, 70, 80: (221 + 2) >> 2 = 55.
        assert_eq!(chain.data[68], 55);
        // Bottom left block 90, 100, 130, 140: (460 + 2) >> 2 = 115.
        assert_eq!(chain.data[72], 115);
        // Bottom right block 110, 120, 150, 161: (541 + 2) >> 2 = 135.
        assert_eq!(chain.data[76], 135);
        // Level 2 averages level 1's red 35, 55, 115, 135: (340 + 2) >> 2 = 85.
        assert_eq!(chain.data[80], 85);
    }

    /// A level one texel high repeats its row, as Godot's down step of 0 does.
    #[test]
    fn a_single_row_halves_only_its_width() {
        let pixels: Vec<u8> = [8_u8, 16, 33, 64]
            .into_iter()
            .flat_map(|v| [v; 4])
            .collect();
        let chain = mip_chain((pixels, 4, 1));
        // Levels 4x1, 2x1, 1x1.
        assert_eq!(chain.data.len(), (4 + 2 + 1) * 4);
        // (8 + 16 + 8 + 16 + 2) >> 2 = 12; (33 + 64 + 33 + 64 + 2) >> 2 = 49.
        assert_eq!(chain.data[16], 12);
        assert_eq!(chain.data[20], 49);
        // (12 + 49 + 12 + 49 + 2) >> 2 = 31.
        assert_eq!(chain.data[24], 31);
    }
}
