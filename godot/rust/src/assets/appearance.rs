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
use godot::{classes::ImageTexture, prelude::*};
use osso_asset_resolver::{AssetResolverConfig, CascListfileResolver};
use rusqlite::{Connection, OpenFlags};

use super::material::texture_from_rgba;
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
    pub(super) textures: HashMap<u32, TexturePixels>,
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
        resolve_armor: impl FnOnce(u8, u8) -> Result<ResolvedEquipmentAppearance, String>,
    ) -> Result<Option<PreparedNpc>, String> {
        let Some(appearance) = self.query_appearance(data_root, display_id)? else {
            return Ok(None);
        };
        let armor = resolve_armor(appearance.race, appearance.sex)?;
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
                textures,
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
    let (composed, mut decoded) = load_and_compose_selected_pixels(
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
    if let Some(fdid) = compositor.replacement_texture_fdid(&selected.materials, layout_id, 19) {
        let pixels = decoded
            .remove(&fdid)
            .ok_or_else(|| format!("missing NPC texture FDID {fdid}"))?;
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

fn make_texture((pixels, width, height): TexturePixels) -> Result<Gd<ImageTexture>, String> {
    texture_from_rgba(&pixels, width, height)
}
