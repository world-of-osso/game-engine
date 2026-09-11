//! Synchronous loading artwork access; no toolkit frame/render state.

use std::collections::{HashMap, HashSet};

use bevy::ecs::system::SystemParam;
use bevy::prelude::*;
use game_engine::ui::screens::loading_component::*;
use ui_toolkit::font_registry::FontRegistry;
use ui_toolkit::render::LoadedTexture;
use ui_toolkit::render_texture::{BlpLoaderRes, load_texture_source};
use ui_toolkit::widgets::{font_string::GameFont, texture::TextureSource};

#[derive(SystemParam)]
pub(crate) struct LoadingViewAssets<'w, 's> {
    images: Option<ResMut<'w, Assets<Image>>>,
    fonts: ResMut<'w, Assets<Font>>,
    font_registry: ResMut<'w, FontRegistry>,
    blp_loader: Option<Res<'w, BlpLoaderRes>>,
    textures: Local<'s, HashMap<u32, Handle<Image>>>,
    files: Local<'s, HashMap<String, Handle<Image>>>,
    missing_textures: Local<'s, HashSet<u32>>,
    missing_files: Local<'s, HashSet<String>>,
}

pub(super) struct LoadingArtwork {
    pub art: LoadedTexture,
    pub top: LoadedTexture,
    pub bottom: LoadedTexture,
    pub logo: LoadedTexture,
    pub left: LoadedTexture,
    pub center: LoadedTexture,
    pub right: LoadedTexture,
    pub fill: LoadedTexture,
    pub font: Handle<Font>,
}

impl LoadingViewAssets<'_, '_> {
    fn load_file(&mut self, path: &str) -> Result<LoadedTexture, String> {
        load_texture_source(
            &TextureSource::File(path.to_owned()),
            &mut self.images,
            &mut self.textures,
            &mut self.files,
            &mut self.missing_textures,
            &mut self.missing_files,
            self.blp_loader.as_deref(),
        )
        .ok_or_else(|| format!("Unable to load native loading texture {path}"))
    }

    pub(super) fn load(&mut self) -> Result<LoadingArtwork, String> {
        Ok(LoadingArtwork {
            art: self.load_file(TEX_LOADING_ART)?,
            top: self.load_file(TEX_LOADING_FILLER_TOP)?,
            bottom: self.load_file(TEX_LOADING_FILLER_BOTTOM)?,
            logo: self.load_file(TEX_GAME_LOGO)?,
            left: self.load_file(TEX_LOADING_BAR_LEFT)?,
            center: self.load_file(TEX_LOADING_BAR_CENTER)?,
            right: self.load_file(TEX_LOADING_BAR_RIGHT)?,
            fill: self.load_file(TEX_LOADING_BAR_FILL)?,
            font: self
                .font_registry
                .get(GameFont::FrizQuadrata, &mut self.fonts),
        })
    }
}
