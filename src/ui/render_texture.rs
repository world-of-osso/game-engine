use bevy::asset::RenderAssetUsages;
use bevy::image::{CompressedImageFormats, ImageSampler, ImageType};
use bevy::prelude::*;
use std::collections::{HashMap, HashSet};
use std::fs;

use crate::asset;
use crate::ui::atlas;
use crate::ui::render::LoadedTexture;
use crate::ui::widgets::texture::TextureSource;

pub fn load_texture_source_pub(
    source: &TextureSource,
    images: &mut Option<ResMut<Assets<Image>>>,
    texture_cache: &mut HashMap<u32, Handle<Image>>,
    file_texture_cache: &mut HashMap<String, Handle<Image>>,
    missing_textures: &mut HashSet<u32>,
    missing_file_textures: &mut HashSet<String>,
) -> Option<LoadedTexture> {
    load_texture_source(
        source,
        images,
        texture_cache,
        file_texture_cache,
        missing_textures,
        missing_file_textures,
    )
}

pub fn load_texture_source(
    source: &TextureSource,
    images: &mut Option<ResMut<Assets<Image>>>,
    texture_cache: &mut HashMap<u32, Handle<Image>>,
    file_texture_cache: &mut HashMap<String, Handle<Image>>,
    missing_textures: &mut HashSet<u32>,
    missing_file_textures: &mut HashSet<String>,
) -> Option<LoadedTexture> {
    match source {
        TextureSource::FileDataId(fdid) => {
            load_fdid_texture(*fdid, images, texture_cache, missing_textures)
                .map(|handle| LoadedTexture { handle, rect: None })
        }
        TextureSource::File(path) => {
            load_file_texture(path, images, file_texture_cache, missing_file_textures)
                .map(|handle| LoadedTexture { handle, rect: None })
        }
        TextureSource::Atlas(name) => {
            load_atlas_texture(name, images, file_texture_cache, missing_file_textures)
        }
        _ => None,
    }
}

fn load_atlas_texture(
    name: &str,
    images: &mut Option<ResMut<Assets<Image>>>,
    file_texture_cache: &mut HashMap<String, Handle<Image>>,
    missing_file_textures: &mut HashSet<String>,
) -> Option<LoadedTexture> {
    let region = atlas::get_region(name)?;
    let handle = load_file_texture(
        region.path,
        images,
        file_texture_cache,
        missing_file_textures,
    )?;
    let rect = images
        .as_ref()
        .and_then(|assets| assets.get(&handle))
        .map(|image| region.rect_pixels(image));
    Some(LoadedTexture { handle, rect })
}

pub fn load_file_texture(
    path: &str,
    images: &mut Option<ResMut<Assets<Image>>>,
    file_texture_cache: &mut HashMap<String, Handle<Image>>,
    missing_file_textures: &mut HashSet<String>,
) -> Option<Handle<Image>> {
    if let Some(handle) = file_texture_cache.get(path) {
        return Some(handle.clone());
    }
    if missing_file_textures.contains(path) {
        return None;
    }
    let assets = images.as_mut().map(|images| &mut **images)?;
    let image = match load_ui_file_texture(path) {
        Ok(image) => image,
        Err(_) => {
            missing_file_textures.insert(path.to_string());
            return None;
        }
    };
    let handle = assets.add(image);
    file_texture_cache.insert(path.to_string(), handle.clone());
    Some(handle)
}

fn load_image_from_buffer(path: &str, ext: &str) -> Result<Image, String> {
    let bytes = fs::read(path).map_err(|err| format!("Failed to read {ext}: {err}"))?;
    Image::from_buffer(
        &bytes,
        ImageType::Extension(ext),
        CompressedImageFormats::NONE,
        true,
        ImageSampler::default(),
        RenderAssetUsages::default(),
    )
    .map_err(|err| format!("Failed to decode {ext}: {err}"))
}

fn load_ui_file_texture(path: &str) -> Result<Image, String> {
    let lower = path.to_ascii_lowercase();
    if lower.ends_with(".ktx2") {
        load_image_from_buffer(path, "ktx2")
    } else if lower.ends_with(".png") {
        load_image_from_buffer(path, "png")
    } else if should_cpu_decode_ui_texture(path) {
        asset::blp::load_blp_to_image(std::path::Path::new(path))
    } else {
        asset::blp::load_blp_gpu_image(std::path::Path::new(path))
    }
}

fn should_cpu_decode_ui_texture(path: &str) -> bool {
    path.ends_with("Glues-BlizzardLogo.blp")
}

pub fn load_fdid_texture(
    fdid: u32,
    images: &mut Option<ResMut<Assets<Image>>>,
    texture_cache: &mut HashMap<u32, Handle<Image>>,
    missing_textures: &mut HashSet<u32>,
) -> Option<Handle<Image>> {
    if let Some(handle) = texture_cache.get(&fdid) {
        return Some(handle.clone());
    }
    if missing_textures.contains(&fdid) {
        return None;
    }
    let assets = images.as_mut().map(|images| &mut **images)?;
    let path = asset::casc_resolver::ensure_texture(fdid)?;
    let image = match asset::blp::load_blp_gpu_image(&path) {
        Ok(image) => image,
        Err(_) => {
            missing_textures.insert(fdid);
            return None;
        }
    };
    let handle = assets.add(image);
    texture_cache.insert(fdid, handle.clone());
    Some(handle)
}
