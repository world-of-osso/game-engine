//! Authored M2 batch material: shader variants plus the original CPU texture-composition route.

use std::{
    fs,
    path::{Path, PathBuf},
};

use game_engine_core::{
    blp,
    m2_batch_data::{OverlayScale, ResolvedBatch},
    m2_texture_composite_data,
};
use godot::{
    classes::{
        Image, ImageTexture, ProjectSettings, ResourceLoader, Shader, ShaderMaterial, image,
    },
    prelude::*,
};

const SHADER_PATH: &str = "res://shaders/m2.gdshader";
const RENDER_MODE: &str =
    "render_mode ambient_light_disabled, fog_disabled, specular_disabled, cull_back, blend_mix;";
type DecodedTexture = (Vec<u8>, u32, u32);

pub(super) fn is_effect(batch: &ResolvedBatch) -> bool {
    batch.texture_2_fdid.is_some() && batch.blend_mode >= 2 && batch.overlays.is_empty()
}

pub(super) fn load_material(
    batch: &ResolvedBatch,
    path: &GString,
    missing: &mut PackedInt32Array,
) -> Result<Gd<ShaderMaterial>, String> {
    let texture_dir = Path::new(
        &ProjectSettings::singleton()
            .globalize_path(path)
            .to_string(),
    )
    .parent()
    .and_then(Path::parent)
    .ok_or("Model path has no asset root")?
    .join("textures");
    let effect = is_effect(batch);
    let base = batch
        .texture_fdid
        .map(|fdid| load_texture(fdid, &texture_dir, missing))
        .transpose()?
        .flatten();
    let second = if effect {
        batch
            .texture_2_fdid
            .map(|fdid| load_texture(fdid, &texture_dir, missing))
            .transpose()?
            .flatten()
    } else {
        None
    };
    let base = if effect {
        base
    } else if let Some((mut pixels, width, height)) = base {
        compose_texture(&mut pixels, width, height, batch, &texture_dir, missing)?;
        Some((pixels, width, height))
    } else {
        None
    };

    let source = ResourceLoader::singleton()
        .load(SHADER_PATH)
        .ok_or_else(|| format!("Cannot load M2 shader {SHADER_PATH}"))?
        .try_cast::<Shader>()
        .map_err(|_| format!("M2 shader {SHADER_PATH} has wrong resource type"))?
        .get_code()
        .to_string();
    let variant = shader_variant(&source, batch, effect)?;
    let mut shader = Shader::new_gd();
    shader.set_code(&variant);
    let mut material = ShaderMaterial::new_gd();
    material.set_shader(&shader);
    bind_textures(&mut material, base, second)?;
    bind_uniforms(&mut material, batch, effect);
    Ok(material)
}

fn bind_textures(
    material: &mut Gd<ShaderMaterial>,
    base: Option<DecodedTexture>,
    second: Option<DecodedTexture>,
) -> Result<(), String> {
    if let Some((pixels, width, height)) = base {
        material.set_shader_parameter(
            "base_texture",
            &texture_from_rgba(&pixels, width, height)?.to_variant(),
        );
    }
    if let Some((pixels, width, height)) = second {
        material.set_shader_parameter(
            "second_texture",
            &texture_from_rgba(&pixels, width, height)?.to_variant(),
        );
    }
    Ok(())
}

fn bind_uniforms(material: &mut Gd<ShaderMaterial>, batch: &ResolvedBatch, effect: bool) {
    material.set_shader_parameter("effect_mode", &(effect as i32).to_variant());
    material.set_shader_parameter("shader_id", &(batch.shader_id as i32).to_variant());
    material.set_shader_parameter("render_flags", &(batch.render_flags as i32).to_variant());
    material.set_shader_parameter("gx_blend", &gx_blend(batch.blend_mode).to_variant());
    material.set_shader_parameter("uv_mode_1", &(batch.use_uv_2_1 as i32).to_variant());
    material.set_shader_parameter("uv_mode_2", &(batch.use_uv_2_2 as i32).to_variant());
    material.set_shader_parameter("transparency", &batch.transparency.to_variant());
    let alpha_test = match batch.blend_mode {
        1 => 224.0 / 255.0 * batch.transparency,
        2..=7 if effect => 1.0 / 255.0 * batch.transparency,
        _ => 0.0,
    };
    material.set_shader_parameter("alpha_test", &alpha_test.to_variant());
}

fn shader_variant(source: &str, batch: &ResolvedBatch, effect: bool) -> Result<String, String> {
    let blend = match batch.blend_mode {
        0 | 1 | 2 | 3 | 7 => "blend_mix",
        _ => "blend_add",
    };
    let cull = if !effect && batch.render_flags & 4 != 0 {
        "cull_disabled"
    } else {
        "cull_back"
    };
    let depth = if effect { ", depth_draw_never" } else { "" };
    let render_mode = format!(
        "render_mode ambient_light_disabled, fog_disabled, specular_disabled, {cull}, {blend}{depth};"
    );
    let (variant, count) = (
        source.replace(RENDER_MODE, &render_mode),
        source.matches(RENDER_MODE).count(),
    );
    if count != 1 {
        return Err(format!(
            "Expected one M2 render_mode declaration, found {count}"
        ));
    }
    if batch.blend_mode <= 1 {
        if variant.matches("ALPHA = color.a;").count() != 1 {
            return Err("Expected one M2 alpha output".into());
        }
        Ok(variant.replace("ALPHA = color.a;", ""))
    } else {
        Ok(variant)
    }
}

fn gx_blend(mode: u16) -> i32 {
    match mode {
        0 => 0,
        1 => 1,
        2 => 2,
        3 => 10,
        4 => 3,
        5 => 4,
        6 => 5,
        7 => 13,
        _ => 0,
    }
}

fn texture_path(fdid: u32, dir: &Path) -> PathBuf {
    dir.join(format!("{fdid}.blp"))
}

fn load_texture(
    fdid: u32,
    dir: &Path,
    missing: &mut PackedInt32Array,
) -> Result<Option<DecodedTexture>, String> {
    let path = texture_path(fdid, dir);
    let bytes = match fs::read(&path) {
        Ok(bytes) => bytes,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
            missing.push(fdid as i32);
            return Ok(None);
        }
        Err(error) => return Err(format!("Cannot read texture {fdid}: {error}")),
    };
    let rgba = blp::decode_rgba(&bytes).map_err(|error| format!("Texture {fdid}: {error}"))?;
    Ok(Some((rgba.pixels, rgba.width, rgba.height)))
}

fn compose_texture(
    pixels: &mut [u8],
    width: u32,
    height: u32,
    batch: &ResolvedBatch,
    dir: &Path,
    missing: &mut PackedInt32Array,
) -> Result<(), String> {
    if let Some(fdid) = batch.texture_2_fdid.filter(|_| !batch.use_env_map_2) {
        if let Some((second, w, h)) = load_texture(fdid, dir, missing)? {
            m2_texture_composite_data::composite_second_texture_pixels(
                pixels,
                width,
                height,
                &second,
                w,
                h,
                batch.shader_id,
            );
        }
    }
    for overlay in &batch.overlays {
        if let Some((bytes, w, h)) = load_texture(overlay.fdid, dir, missing)? {
            m2_texture_composite_data::composite_overlay_pixels(
                pixels,
                width,
                &bytes,
                w,
                h,
                overlay.x,
                overlay.y,
                overlay.scale == OverlayScale::Uniform2x,
            );
        }
    }
    Ok(())
}

fn texture_from_rgba(pixels: &[u8], width: u32, height: u32) -> Result<Gd<ImageTexture>, String> {
    let image = Image::create_from_data(
        width as i32,
        height as i32,
        false,
        image::Format::RGBA8,
        &PackedByteArray::from(pixels),
    )
    .ok_or_else(|| format!("Godot rejected {width}x{height} M2 image"))?;
    ImageTexture::create_from_image(&image).ok_or_else(|| "Godot rejected M2 texture".into())
}
