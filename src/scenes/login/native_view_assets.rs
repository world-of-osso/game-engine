//! Login artwork loading without toolkit frame/render state.

use std::collections::{HashMap, HashSet};

use bevy::ecs::system::SystemParam;
use bevy::prelude::*;
use ui_toolkit::font_registry::FontRegistry;
use ui_toolkit::render::LoadedTexture;
use ui_toolkit::render_texture::{BlpLoaderRes, load_texture_source};
use ui_toolkit::widgets::{font_string::GameFont, texture::TextureSource};

#[derive(SystemParam)]
pub(crate) struct LoginViewAssets<'w, 's> {
    images: Option<ResMut<'w, Assets<Image>>>,
    fonts: ResMut<'w, Assets<Font>>,
    font_registry: ResMut<'w, FontRegistry>,
    blp_loader: Option<Res<'w, BlpLoaderRes>>,
    textures: Local<'s, HashMap<u32, Handle<Image>>>,
    files: Local<'s, HashMap<String, Handle<Image>>>,
    missing_textures: Local<'s, HashSet<u32>>,
    missing_files: Local<'s, HashSet<String>>,
}

pub(super) struct LoginArtwork {
    pub background: LoadedTexture,
    pub logo: LoadedTexture,
    pub blizzard_logo: LoadedTexture,
    pub input: [LoadedTexture; 9],
    pub primary: ButtonArtwork,
    pub secondary: ButtonArtwork,
    pub friz: Handle<Font>,
    pub arial: Handle<Font>,
}

#[derive(Clone)]
pub(super) struct ButtonArtwork {
    pub states: [Vec<ImageNode>; 4],
    pub display_edges: [f32; 4],
}

impl LoginViewAssets<'_, '_> {
    fn texture(&mut self, source: TextureSource) -> Result<LoadedTexture, String> {
        load_texture_source(
            &source,
            &mut self.images,
            &mut self.textures,
            &mut self.files,
            &mut self.missing_textures,
            &mut self.missing_files,
            self.blp_loader.as_deref(),
        )
        .ok_or_else(|| format!("Unable to load native login texture {source:?}"))
    }

    fn file(&mut self, path: &str) -> Result<LoadedTexture, String> {
        self.texture(TextureSource::File(path.to_owned()))
    }

    pub(super) fn load(&mut self) -> Result<LoginArtwork, String> {
        let mut input = Vec::with_capacity(9);
        for part in ["TL", "T", "TR", "L", "M", "R", "BL", "B", "BR"] {
            input.push(self.file(&format!(
                "data/ui/login-input/Common-Input-Border-{part}.blp"
            ))?);
        }
        let variant = match std::env::var("LOGIN_BUTTON_VARIANT").ok().as_deref() {
            Some("regular") => Some("output/imagegen/button-dark-bronze-regular.ktx2"),
            Some("knotwork") => Some("output/imagegen/button-carved-bronze-knotwork.ktx2"),
            Some("walnut") => Some("output/imagegen/button-walnut-bronze-framed.ktx2"),
            _ => None,
        };
        Ok(LoginArtwork {
            background: self.file("data/glues/common/world-of-osso-background.ktx2")?,
            logo: self.file("data/glues/common/world-of-osso-logo.ktx2")?,
            blizzard_logo: self.file("data/glues/mainmenu/Glues-BlizzardLogo.blp")?,
            input: input
                .try_into()
                .map_err(|_| "Expected nine input textures")?,
            primary: self.button_artwork(250.0, 66.0, variant)?,
            secondary: self.button_artwork(200.0, 32.0, None)?,
            friz: self
                .font_registry
                .get(GameFont::FrizQuadrata, &mut self.fonts),
            arial: self
                .font_registry
                .get(GameFont::ArialNarrow, &mut self.fonts),
        })
    }

    fn button_artwork(
        &mut self,
        width: f32,
        height: f32,
        file: Option<&str>,
    ) -> Result<ButtonArtwork, String> {
        let mut states = Vec::with_capacity(4);
        let mut display_edges = [4.0; 4];
        for state in ["up", "pressed", "highlight", "disabled"] {
            let name = format!("defaultbutton-nineslice-{state}");
            let source = file.map_or_else(
                || TextureSource::Atlas(name.clone()),
                |path| TextureSource::File(path.to_owned()),
            );
            let loaded = self.texture(source)?;
            let image = self
                .images
                .as_ref()
                .and_then(|images| images.get(&loaded.handle))
                .ok_or_else(|| format!("Native login texture {name} has no image"))?;
            let rect = loaded.rect.unwrap_or(Rect::from_corners(
                Vec2::ZERO,
                Vec2::new(image.width() as f32, image.height() as f32),
            ));
            let uv = if file.is_some() {
                [4.0; 4]
            } else {
                ui_toolkit::atlas::nine_slice_margins(&name)
                    .ok_or_else(|| format!("Missing login button slice margins: {name}"))?
            };
            if file.is_none() {
                let region = ui_toolkit::atlas::get_region(&name)
                    .ok_or_else(|| format!("Missing login button atlas: {name}"))?;
                display_edges = [
                    uv[0] * width / region.width,
                    uv[1] * height / region.height,
                    uv[2] * width / region.width,
                    uv[3] * height / region.height,
                ];
            }
            let xs = [
                rect.min.x,
                rect.min.x + uv[0],
                rect.max.x - uv[2],
                rect.max.x,
            ];
            let ys = [
                rect.min.y,
                rect.min.y + uv[1],
                rect.max.y - uv[3],
                rect.max.y,
            ];
            let mut pieces = Vec::with_capacity(9);
            for row in 0..3 {
                for column in 0..3 {
                    pieces.push(ImageNode {
                        image: loaded.handle.clone(),
                        rect: Some(Rect::new(xs[column], ys[row], xs[column + 1], ys[row + 1])),
                        image_mode: NodeImageMode::Stretch,
                        ..default()
                    });
                }
            }
            states.push(pieces);
        }
        Ok(ButtonArtwork {
            states: states
                .try_into()
                .map_err(|_| "Expected four button states")?,
            display_edges,
        })
    }
}
