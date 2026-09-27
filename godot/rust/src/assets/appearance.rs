//! Prepare authored NPC replacement textures and geosets before allocating visual nodes.

use std::{
    collections::{HashMap, HashSet},
    path::Path,
};

use game_engine_core::{
    asset::m2_texture,
    blp,
    char_texture_data::CharTextureData,
    customization_data::CustomizationDb,
    npc_appearance_assets::{load_compositor, load_customization_db},
    npc_appearance_data::query_authored_npc_appearance,
    npc_appearance_selection_data::{select_npc_choices, select_npc_type6_texture},
};
use godot::{classes::ImageTexture, prelude::*};
use osso_asset_resolver::{AssetResolverConfig, CascListfileResolver};
use rusqlite::{Connection, OpenFlags};

use super::material::texture_from_rgba;

type TexturePixels = (Vec<u8>, u32, u32);

#[derive(Default)]
pub(crate) struct NpcAppearances {
    profiles: Option<Connection>,
    customization: Option<CustomizationDb>,
    compositor: Option<CharTextureData>,
}

pub(crate) struct PreparedNpcAppearance {
    pub(super) textures: HashMap<u32, Gd<ImageTexture>>,
    pub(super) selected_geosets: Vec<(u16, u16)>,
    pub(super) authored_geosets: Vec<(u16, u16)>,
}

impl NpcAppearances {
    pub(crate) fn prepare(
        &mut self,
        data_root: &Path,
        cache_root: &Path,
        display_id: u32,
    ) -> Result<Option<PreparedNpcAppearance>, String> {
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
        let Some(appearance) = query_authored_npc_appearance(
            self.profiles.as_ref().expect("opened NPC appearance cache"),
            display_id,
        )?
        else {
            return Ok(None);
        };

        if self.customization.is_none() {
            self.customization = Some(load_customization_db(data_root)?);
        }
        let db = self
            .customization
            .as_ref()
            .expect("loaded customization db");
        let selected = select_npc_choices(&appearance, db)?;
        let layout_id = db
            .layout_id(appearance.race, appearance.sex)
            .ok_or_else(|| {
                format!(
                    "missing texture layout for race {} sex {}",
                    appearance.race, appearance.sex
                )
            })?;
        if self.compositor.is_none() {
            self.compositor = Some(load_compositor(data_root)?);
        }
        let compositor = self.compositor.as_ref().expect("loaded compositor");
        let layout = compositor
            .layout(layout_id)
            .ok_or_else(|| format!("cannot composite NPC texture layout {layout_id}"))?;
        let default_fdid = m2_texture::default_fdid_for_type(
            1,
            layout.width == 2048 && layout.height == 1024,
            &[0, 0, 0],
        )
        .ok_or_else(|| format!("missing default NPC body texture for layout {layout_id}"))?;
        let resolver = CascListfileResolver::new(
            AssetResolverConfig::new()
                .with_data_root(data_root)
                .with_shared_data_root(data_root)
                .with_cache_root(cache_root),
        );
        let mut decoded = HashMap::new();
        for fdid in selected
            .materials
            .iter()
            .map(|(_, fdid)| *fdid)
            .collect::<HashSet<_>>()
        {
            decoded.insert(fdid, load_npc_texture(&resolver, data_root, fdid)?);
        }
        // The default atlas is optional in the original compositor, but failures must be visible.
        if !decoded.contains_key(&default_fdid) {
            match load_npc_texture(&resolver, data_root, default_fdid) {
                Ok(pixels) => {
                    decoded.insert(default_fdid, pixels);
                }
                Err(error) => eprintln!("NPC display {display_id}: {error}"),
            }
        }
        let composed = compositor
            .composite_model_textures_with(
                &selected.materials,
                &[],
                layout_id,
                default_fdid,
                |fdid| decoded.get(&fdid).cloned(),
            )
            .ok_or_else(|| format!("cannot composite NPC texture layout {layout_id}"))?;

        let body = match appearance.baked_texture_fdid {
            Some(fdid) => load_npc_texture(&resolver, data_root, fdid)?,
            None => composed.body,
        };
        let mut textures = HashMap::from([(1, make_texture(body)?)]);
        if let Some(type6) = select_npc_type6_texture(
            compositor.declares_hair(&selected.materials, layout_id),
            composed.hair,
            composed.head,
        )? {
            textures.insert(6, make_texture(type6)?);
        }
        if let Some(fdid) = compositor.replacement_texture_fdid(&selected.materials, layout_id, 19)
        {
            let pixels = decoded
                .remove(&fdid)
                .ok_or_else(|| format!("missing NPC texture FDID {fdid}"))?;
            textures.insert(19, make_texture(pixels)?);
        }
        Ok(Some(PreparedNpcAppearance {
            textures,
            selected_geosets: selected.geosets,
            authored_geosets: appearance.geosets,
        }))
    }
}

fn load_npc_texture(
    resolver: &CascListfileResolver,
    data_root: &Path,
    fdid: u32,
) -> Result<TexturePixels, String> {
    let destination = data_root.join("textures").join(format!("{fdid}.blp"));
    let path = resolver.ensure_cached(fdid, &destination).ok_or_else(|| {
        format!(
            "missing NPC texture FDID {fdid} at {}",
            destination.display()
        )
    })?;
    let bytes = std::fs::read(&path).map_err(|error| format!("{}: {error}", path.display()))?;
    let rgba = blp::decode_rgba(&bytes).map_err(|error| {
        format!(
            "decode NPC texture FDID {fdid} at {}: {error}",
            path.display()
        )
    })?;
    Ok((rgba.pixels, rgba.width, rgba.height))
}

fn make_texture((pixels, width, height): TexturePixels) -> Result<Gd<ImageTexture>, String> {
    texture_from_rgba(&pixels, width, height)
}
