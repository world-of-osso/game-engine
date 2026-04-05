use super::*;

pub(super) struct WmoMaterialProps {
    pub(super) texture_fdid: u32,
    pub(super) texture_2_fdid: u32,
    pub(super) texture_3_fdid: u32,
    pub(super) blend_mode: u32,
    pub(super) unculled: bool,
    pub(super) shader: u32,
    pub(super) sidn_glow: Option<WmoSidnGlow>,
}

pub(super) fn wmo_material_props(root: &wmo::WmoRootData, material_index: u16) -> WmoMaterialProps {
    let mat_def = root.materials.get(material_index as usize);
    WmoMaterialProps {
        texture_fdid: mat_def.map(|m| m.texture_fdid).unwrap_or(0),
        texture_2_fdid: mat_def.map(|m| m.texture_2_fdid).unwrap_or(0),
        texture_3_fdid: mat_def.map(|m| m.texture_3_fdid).unwrap_or(0),
        blend_mode: mat_def.map(|m| m.blend_mode).unwrap_or(0),
        unculled: mat_def.map(|m| m.material_flags.unculled).unwrap_or(false),
        shader: mat_def.map(|m| m.shader).unwrap_or(0),
        sidn_glow: mat_def.and_then(build_wmo_sidn_glow),
    }
}

pub(super) fn build_wmo_sidn_glow(mat_def: &wmo::WmoMaterialDef) -> Option<WmoSidnGlow> {
    let rgb = &mat_def.sidn_color[..3];
    (mat_def.material_flags.sidn && rgb.iter().any(|channel| *channel > 0.0)).then_some(
        WmoSidnGlow {
            base_sidn_color: mat_def.sidn_color,
        },
    )
}

pub(super) fn load_wmo_batch_material_image(
    images: &mut Assets<Image>,
    material_index: u16,
    material_props: &WmoMaterialProps,
) -> Option<Handle<Image>> {
    if material_props.texture_fdid == 0 {
        return None;
    }
    let Some(blp_path) = crate::asset::asset_cache::texture(material_props.texture_fdid) else {
        log_wmo_texture_extract_failure(material_props.texture_fdid);
        return None;
    };
    match load_wmo_material_image(
        &blp_path,
        material_props.shader,
        material_props.texture_2_fdid,
        material_props.texture_3_fdid,
        images,
    ) {
        Ok(image) => Some(image),
        Err(err) => {
            log_wmo_texture_decode_failure(material_index, material_props, &err);
            None
        }
    }
}

pub(super) fn log_wmo_texture_decode_failure(
    material_index: u16,
    material_props: &WmoMaterialProps,
    err: &str,
) {
    eprintln!(
        "WMO texture decode failed for material {material_index} shader {} FDID {}: {err}",
        material_props.shader, material_props.texture_fdid
    );
}

pub(super) fn log_wmo_texture_extract_failure(texture_fdid: u32) {
    let label = game_engine::listfile::lookup_fdid(texture_fdid).unwrap_or("unknown");
    eprintln!("WMO texture extract failed for FDID {texture_fdid}: {label}");
}

pub(super) fn load_wmo_material_image(
    base_path: &Path,
    shader: u32,
    texture_2_fdid: u32,
    texture_3_fdid: u32,
    images: &mut Assets<Image>,
) -> Result<Handle<Image>, String> {
    let key = WmoTextureCacheKey {
        base_path: base_path.to_path_buf(),
        shader,
        texture_2_fdid,
        texture_3_fdid,
    };
    let cache = wmo_texture_cache();
    if let Some(cached) = lookup_cached_wmo_material_image(cache, &key) {
        return cached;
    }
    let (mut pixels, w, h) = blp::load_blp_rgba(base_path)?;
    composite_wmo_overlay_layers(&mut pixels, w, h, shader, [texture_2_fdid, texture_3_fdid]);
    let handle = images.add(build_wmo_material_image(pixels, w, h));
    cache.lock().unwrap().insert(key, Ok(handle.clone()));
    Ok(handle)
}

pub(super) fn wmo_texture_cache()
-> &'static Mutex<std::collections::HashMap<WmoTextureCacheKey, Result<Handle<Image>, String>>> {
    WMO_TEXTURE_CACHE.get_or_init(|| Mutex::new(std::collections::HashMap::new()))
}

pub(super) fn lookup_cached_wmo_material_image(
    cache: &Mutex<std::collections::HashMap<WmoTextureCacheKey, Result<Handle<Image>, String>>>,
    key: &WmoTextureCacheKey,
) -> Option<Result<Handle<Image>, String>> {
    cache.lock().unwrap().get(key).cloned()
}

pub(super) fn composite_wmo_overlay_layers(
    pixels: &mut [u8],
    width: u32,
    height: u32,
    shader: u32,
    overlay_fdids: [u32; 2],
) {
    let descriptor = describe_wmo_shader(shader);
    let combine_modes = [descriptor.second_layer, descriptor.third_layer];

    for (overlay_fdid, combine_mode) in overlay_fdids.into_iter().zip(combine_modes) {
        if overlay_fdid == 0 {
            continue;
        }
        let Some(overlay_path) = crate::asset::asset_cache::texture(overlay_fdid) else {
            continue;
        };
        let Ok((overlay_pixels, ov_w, ov_h)) = blp::load_blp_rgba(&overlay_path) else {
            continue;
        };
        if ov_w == width && ov_h == height {
            composite_wmo_shader_layer(pixels, &overlay_pixels, combine_mode);
        }
    }
}

pub(super) fn build_wmo_material_image(pixels: Vec<u8>, width: u32, height: u32) -> Image {
    let mut image = Image::new(
        bevy::render::render_resource::Extent3d {
            width,
            height,
            depth_or_array_layers: 1,
        },
        bevy::render::render_resource::TextureDimension::D2,
        pixels,
        bevy::render::render_resource::TextureFormat::Rgba8UnormSrgb,
        bevy::asset::RenderAssetUsages::default(),
    );
    image.sampler = bevy::image::ImageSampler::Descriptor(bevy::image::ImageSamplerDescriptor {
        address_mode_u: bevy::image::ImageAddressMode::Repeat,
        address_mode_v: bevy::image::ImageAddressMode::Repeat,
        ..bevy::image::ImageSamplerDescriptor::linear()
    });
    image
}

pub(crate) fn wmo_standard_material(
    texture: Option<Handle<Image>>,
    blend_mode: u32,
    unculled: bool,
    shader: u32,
    interior_ambient: Option<[f32; 4]>,
    has_vertex_color: bool,
    sidn_glow: Option<WmoSidnGlow>,
) -> StandardMaterial {
    let alpha_mode = match blend_mode {
        2 | 3 => AlphaMode::Blend,
        _ if texture.is_some() => AlphaMode::Mask(0.5),
        _ => AlphaMode::Opaque,
    };
    let double_sided = unculled;
    let prop_like_surface = double_sided || !matches!(alpha_mode, AlphaMode::Opaque);
    let surface = wmo_surface_params(texture.is_some(), prop_like_surface, shader);
    StandardMaterial {
        base_color: wmo_base_color(interior_ambient, texture.is_some()),
        base_color_texture: texture,
        perceptual_roughness: surface.roughness,
        reflectance: surface.reflectance,
        metallic: surface.metallic,
        emissive: wmo_emissive(shader, sidn_glow),
        unlit: has_vertex_color,
        double_sided,
        cull_mode: wmo_cull_mode(double_sided),
        alpha_mode,
        ..default()
    }
}

fn wmo_base_color(interior_ambient: Option<[f32; 4]>, has_texture: bool) -> Color {
    if let Some(ambient) = interior_ambient {
        Color::linear_rgba(ambient[0], ambient[1], ambient[2], 1.0)
    } else if !has_texture {
        Color::srgb(0.6, 0.6, 0.6)
    } else {
        Color::WHITE
    }
}

fn wmo_emissive(shader: u32, sidn_glow: Option<WmoSidnGlow>) -> LinearRgba {
    sidn_glow
        .map(|glow| sidn_emissive_color(glow.base_sidn_color, 0.0))
        .unwrap_or_else(|| shader_emissive(shader))
}

fn shader_emissive(shader: u32) -> LinearRgba {
    if describe_wmo_shader(shader).emissive {
        LinearRgba::rgb(0.05, 0.05, 0.05)
    } else {
        LinearRgba::BLACK
    }
}

fn wmo_cull_mode(double_sided: bool) -> Option<bevy::render::render_resource::Face> {
    if double_sided {
        None
    } else {
        Some(bevy::render::render_resource::Face::Back)
    }
}

pub(crate) fn sync_wmo_sidn_emissive(
    game_time: Res<GameTime>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    query: Query<(&MeshMaterial3d<StandardMaterial>, &WmoSidnGlow)>,
    new_glow_query: Query<
        (&MeshMaterial3d<StandardMaterial>, &WmoSidnGlow),
        Or<(
            Added<WmoSidnGlow>,
            Changed<MeshMaterial3d<StandardMaterial>>,
        )>,
    >,
    mut last_strength: Local<Option<f32>>,
) {
    let strength = sidn_glow_strength(game_time.minutes);
    if last_strength.is_some_and(|last| (last - strength).abs() < 0.001) {
        apply_sidn_emissive_updates(&mut materials, &new_glow_query, strength);
        return;
    }
    *last_strength = Some(strength);

    apply_sidn_emissive_updates(&mut materials, &query, strength);
}

pub(super) fn apply_sidn_emissive_updates<F: QueryFilter>(
    materials: &mut Assets<StandardMaterial>,
    query: &Query<(&MeshMaterial3d<StandardMaterial>, &WmoSidnGlow), F>,
    strength: f32,
) {
    for (material_handle, glow) in query.iter() {
        let Some(material) = materials.get_mut(material_handle) else {
            continue;
        };
        material.emissive = sidn_emissive_color(glow.base_sidn_color, strength);
    }
}

pub(crate) fn sidn_glow_strength(minutes: f32) -> f32 {
    let sun_cycle = ((minutes.rem_euclid(2880.0) / 2880.0) * std::f32::consts::TAU
        - std::f32::consts::FRAC_PI_2)
        .sin();
    (-sun_cycle).max(0.0).powf(1.25)
}

pub(super) fn sidn_emissive_color(base_sidn_color: [f32; 4], strength: f32) -> LinearRgba {
    let alpha = base_sidn_color[3];
    let scale = alpha * strength;
    let linear =
        Color::srgb(base_sidn_color[0], base_sidn_color[1], base_sidn_color[2]).to_linear();
    LinearRgba::rgb(
        linear.red * scale,
        linear.green * scale,
        linear.blue * scale,
    )
}
