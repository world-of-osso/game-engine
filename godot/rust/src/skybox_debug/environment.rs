//! Original skyboxdebug environment, reference ground and camera-child procedural dome.
//!
//! Sources: src/scenes/skybox_debug/mod.rs, rendering/skybox/{mod.rs,cloud_texture.rs},
//! rendering/terrain/ground.rs and assets/shaders/{sky.wgsl,retail_m2.wgsl}.
//! No-server GameTime is fixed at 1440; cloud regeneration is not registered in the
//! original SkyPlugin. Only seed 0 is sampled, and cloud UV scroll depends on GameTime,
//! not elapsed milliseconds. Authored sky animation is deliberately outside this module.
//!
//! Linear distance fog is owned by the reference shader. Environment fog stays off:
//! stock depth/exponential fog is not substituted for original authored-space fog.
//! The existing fixture's Environment-only fog observation therefore needs expansion.
//! Original global sky updates can overwrite the debug camera's initial 15..45 fog;
//! this layer preserves the explicitly requested debug range, not that later overwrite.
//! Godot intensity=2500 and ambient energy=60 record original inputs, not established
//! Bevy/Godot physical-unit equivalence. Retail reference shading ignores both, as the
//! original M2 material does. Camera reflection/IBL stays disabled; the original gradient
//! cubemap is retained as a resource, not misrepresented as camera image-based lighting.

use std::{fs, path::Path};

use game_engine_core::{
    lighting_assets::{linear_to_authored_rgb, parse_light_data_csv},
    retail_light_data::{RetailLightColors, scene_light},
    sky_cubemap_data::{self, SKY_BAND_MAX, sky_dome_profile},
    sky_lightdata_data::{SkyColorSet, interpolate_colors},
};
use glam::{IVec2, Vec2};
use godot::{
    classes::{
        ArrayMesh, Camera3D, Cubemap, DirectionalLight3D, Environment, Image, MeshInstance3D,
        Node3D, PlaneMesh, Shader, ShaderMaterial, environment,
        geometry_instance_3d::ShadowCastingSetting, image, light_3d, mesh,
    },
    prelude::*,
};

use super::Composition;
use crate::assets::material::{shared_texture, texture_from_rgba};

const NOON: f32 = 1440.0;
const PRE_WORLD_LIGHT_PARAMS: u32 = 12;
const DOME_RADIUS: f32 = 500.0;
const LONGITUDE_SEGMENTS: usize = 32;
const GRASS_FDID: u32 = 187126;
const CLOUD_TEXTURE_WIDTH: u32 = 512;
const CLOUD_TEXTURE_HEIGHT: u32 = 1024;
const CLOUD_OCTAVES: usize = 6;
const CLOUD_RIDGE_SEED_MIX: u32 = 0x9E37_79B9;
const GRADIENT_HASH_X_MIX: u32 = 0x8DA6_B343;
const GRADIENT_HASH_Y_MIX: u32 = 0xD816_3841;
const GRADIENT_HASH_FINAL_MIX: u32 = 0x85EB_CA6B;

type SkyColors = SkyColorSet<[f32; 3]>;
type DomeVertex = ([f32; 3], [f32; 2]);

pub(super) struct DebugEnvironment {
    // Own the original gradient resource without enabling camera IBL.
    _environment_map: Gd<Cubemap>,
}

impl DebugEnvironment {
    /// Valid no-op: noon is fixed, original cloud seed is fixed, elapsed time affects
    /// neither. MAIN advances the separate authored sky's bone/material animation.
    pub(super) fn update_time_ms(&mut self, _time_ms: u32) -> Result<(), String> {
        Ok(())
    }
}

pub(super) fn create(
    root: &mut Gd<Node3D>,
    camera: &mut Gd<Camera3D>,
    data_root: &Path,
    composition: Composition,
) -> Result<DebugEnvironment, String> {
    // Complete fallible preparation before attaching any manual-lifetime nodes.
    let colors = read_noon_colors(data_root)?;
    let cube = create_cubemap(&colors)?;
    let dome = composition
        .procedural_baseline
        .then(|| create_dome_material(&colors))
        .transpose()?;
    let plane = composition
        .reference_objects
        .then(|| create_reference_material(data_root, &colors, composition.procedural_fog))
        .transpose()?;
    attach_environment(camera, composition.clear_color);
    attach_sun(root);
    if let Some(material) = plane {
        attach_reference_plane(root, &material);
    }
    if let Some(material) = dome {
        attach_dome(camera, &material);
    }
    Ok(DebugEnvironment {
        _environment_map: cube,
    })
}

fn read_noon_colors(data_root: &Path) -> Result<SkyColors, String> {
    let path = data_root.join("LightData.csv");
    let source = fs::read_to_string(&path)
        .map_err(|error| format!("SkyboxDebug read {}: {error}", path.display()))?;
    let keyframes = parse_light_data_csv(&source)
        .map_err(|error| format!("SkyboxDebug parse {}: {error}", path.display()))?;
    let rows = keyframes
        .get(&PRE_WORLD_LIGHT_PARAMS)
        .ok_or_else(|| format!("{} has no LightParams 12 keyframes", path.display()))?;
    interpolate_colors(rows, NOON, |a, b, t| {
        std::array::from_fn(|channel| a[channel] + (b[channel] - a[channel]) * t)
    })
    .ok_or_else(|| format!("{} has no noon LightParams 12 sample", path.display()))
}

fn attach_environment(camera: &mut Gd<Camera3D>, clear: Color) {
    let mut environment = Environment::new_gd();
    environment.set_background(environment::BGMode::COLOR);
    environment.set_bg_color(clear);
    environment.set_ambient_source(environment::AmbientSource::COLOR);
    environment.set_ambient_light_color(Color::WHITE);
    environment.set_ambient_light_energy(60.0);
    environment.set_ambient_light_sky_contribution(0.0);
    environment.set_reflection_source(environment::ReflectionSource::DISABLED);
    environment.set_tonemapper(environment::ToneMapper::LINEAR);
    environment.set_tonemap_exposure(1.0);
    environment.set_fog_enabled(false);
    environment.set_volumetric_fog_enabled(false);
    camera.set_environment(&environment);
}

fn attach_sun(root: &mut Gd<Node3D>) {
    let mut sun = DirectionalLight3D::new_alloc();
    sun.set_name("SkyboxDebugLight");
    sun.set_shadow(false);
    sun.set_param(light_3d::Param::INTENSITY, 2500.0);
    let rotation = glam::Quat::from_euler(
        glam::EulerRot::XYZ,
        -std::f32::consts::PI / 5.0,
        std::f32::consts::PI / 6.0,
        0.0,
    );
    sun.set_quaternion(Quaternion::new(
        rotation.x, rotation.y, rotation.z, rotation.w,
    ));
    root.add_child(&sun);
}

fn create_material(source: &str) -> Gd<ShaderMaterial> {
    let mut shader = Shader::new_gd();
    shader.set_code(source);
    let mut material = ShaderMaterial::new_gd();
    material.set_shader(&shader);
    material
}

fn create_reference_material(
    data_root: &Path,
    colors: &SkyColors,
    fog: bool,
) -> Result<Gd<ShaderMaterial>, String> {
    let mut missing = PackedInt32Array::new();
    let texture = shared_texture(GRASS_FDID, &data_root.join("textures"), &mut missing)?
        .ok_or_else(|| format!("SkyboxDebug missing cached grass FDID {GRASS_FDID}"))?;
    let mut material = create_material(include_str!(
        "../../../shaders/skybox_debug_reference.gdshader"
    ));
    material.set_shader_parameter("base_texture", &texture.to_variant());
    material.set_shader_parameter("linear_fog_enabled", &fog.to_variant());
    bind_reference_light(&mut material, colors);
    Ok(material)
}

fn bind_reference_light(material: &mut Gd<ShaderMaterial>, colors: &SkyColors) {
    let light = scene_light(&reference_light_colors(colors), NOON);
    for (name, value) in [
        ("ambient", light.ambient),
        ("horizon_ambient", light.horizon_ambient),
        ("ground_ambient", light.ground_ambient),
        ("direct", light.direct),
        ("sun_direction", light.sun_direction),
    ] {
        material.set_shader_parameter(name, &Vector3::from_array(value).to_variant());
    }
}

fn reference_light_colors(colors: &SkyColors) -> RetailLightColors {
    RetailLightColors {
        ambient: linear_to_authored_rgb(colors.ambient_color),
        horizon_ambient: linear_to_authored_rgb(colors.horizon_ambient_color),
        ground_ambient: linear_to_authored_rgb(colors.ground_ambient_color),
        direct: linear_to_authored_rgb(colors.direct_color),
        fog_color: [0.18, 0.2, 0.23],
        fog_start: 15.0,
        fog_end: 45.0,
    }
}

fn attach_reference_plane(root: &mut Gd<Node3D>, material: &Gd<ShaderMaterial>) {
    let mut plane = PlaneMesh::new_gd();
    plane.set_size(Vector2::new(100.0, 100.0));
    let mut node = MeshInstance3D::new_alloc();
    node.set_name("SkyboxDebugGroundPlane");
    node.set_mesh(&plane);
    node.set_material_override(material);
    node.set_cast_shadows_setting(ShadowCastingSetting::OFF);
    root.add_child(&node);
}

fn create_dome_material(colors: &SkyColors) -> Result<Gd<ShaderMaterial>, String> {
    let pixels = generate_cloud_pixels(CLOUD_TEXTURE_WIDTH, CLOUD_TEXTURE_HEIGHT, 0);
    let cloud = texture_from_rgba(&pixels, CLOUD_TEXTURE_WIDTH, CLOUD_TEXTURE_HEIGHT)?;
    let mut material = create_material(include_str!(
        "../../../shaders/skybox_debug_procedural.gdshader"
    ));
    material.set_render_priority(-128);
    material.set_shader_parameter("cloud_texture", &cloud.to_variant());
    bind_dome_colors(&mut material, colors);
    let sun = procedural_sun_direction();
    material.set_shader_parameter("sun_direction", &Vector3::from_array(sun).to_variant());
    let params = Vector3::new(colors.cloud_density, NOON * 0.00012, NOON * 0.00004);
    material.set_shader_parameter("cloud_params", &params.to_variant());
    Ok(material)
}

fn bind_dome_colors(material: &mut Gd<ShaderMaterial>, colors: &SkyColors) {
    for (name, color) in [
        ("sky_top", colors.sky_top),
        ("sky_middle", colors.sky_middle),
        ("sky_band1", colors.sky_band1),
        ("sky_band2", colors.sky_band2),
        ("sky_smog", colors.sky_smog),
        ("sky_fog", colors.fog_color),
        ("sun_color", colors.sun_color),
        ("sun_halo_color", colors.sun_halo_color),
        ("cloud_emissive_color", colors.cloud_emissive_color),
        (
            "cloud_layer1_ambient_color",
            colors.cloud_layer1_ambient_color,
        ),
        (
            "cloud_layer2_ambient_color",
            colors.cloud_layer2_ambient_color,
        ),
    ] {
        material.set_shader_parameter(name, &Vector3::from_array(color).to_variant());
    }
}

fn procedural_sun_direction() -> [f32; 3] {
    let pitch = std::f32::consts::FRAC_PI_2 - NOON / 2880.0 * std::f32::consts::TAU;
    let rotation = glam::Quat::from_rotation_y(0.3) * glam::Quat::from_rotation_x(pitch);
    (rotation * glam::Vec3::NEG_Z).normalize().to_array()
}

fn dome_vertices() -> Vec<DomeVertex> {
    sky_dome_profile()
        .into_iter()
        .enumerate()
        .flat_map(|(band, point)| {
            (0..=LONGITUDE_SEGMENTS).map(move |longitude| {
                let u = longitude as f32 / LONGITUDE_SEGMENTS as f32;
                let phi = std::f32::consts::TAU * u;
                let position = [
                    point.horizontal * phi.cos(),
                    point.height,
                    point.horizontal * phi.sin(),
                ];
                (
                    position.map(|value| value * DOME_RADIUS),
                    [u, band as f32 / SKY_BAND_MAX],
                )
            })
        })
        .collect()
}

fn dome_indices() -> Vec<i32> {
    // Godot front faces are clockwise; reverse the Bevy inward CCW index order.
    (0..6)
        .flat_map(|latitude| {
            (0..LONGITUDE_SEGMENTS as i32).flat_map(move |longitude| {
                let a = latitude * (LONGITUDE_SEGMENTS as i32 + 1) + longitude;
                let b = a + LONGITUDE_SEGMENTS as i32 + 1;
                [a, a + 1, b, b, a + 1, b + 1]
            })
        })
        .collect()
}

fn create_dome_mesh() -> Gd<ArrayMesh> {
    let arrays = dome_mesh_arrays(&dome_vertices());
    let mut dome = ArrayMesh::new_gd();
    dome.add_surface_from_arrays(mesh::PrimitiveType::TRIANGLES, &arrays);
    dome
}

fn dome_mesh_arrays(vertices: &[DomeVertex]) -> VarArray {
    let positions: Vec<Vector3> = vertices
        .iter()
        .map(|vertex| Vector3::from_array(vertex.0))
        .collect();
    let normals: Vec<Vector3> = positions
        .iter()
        .map(|position| -position.normalized())
        .collect();
    let uv: Vec<Vector2> = vertices
        .iter()
        .map(|vertex| Vector2::from_array(vertex.1))
        .collect();
    let positions = PackedVector3Array::from(positions.as_slice()).to_variant();
    let normals = PackedVector3Array::from(normals.as_slice()).to_variant();
    let uv = PackedVector2Array::from(uv.as_slice()).to_variant();
    let indices = PackedInt32Array::from(dome_indices().as_slice()).to_variant();
    mesh_arrays([
        (mesh::ArrayType::VERTEX, positions),
        (mesh::ArrayType::NORMAL, normals),
        (mesh::ArrayType::TEX_UV, uv),
        (mesh::ArrayType::INDEX, indices),
    ])
}

fn mesh_arrays(attributes: [(mesh::ArrayType, Variant); 4]) -> VarArray {
    let mut arrays = VarArray::new();
    arrays.resize(mesh::ArrayType::MAX.ord() as usize, &Variant::nil());
    for (kind, values) in attributes {
        arrays.set(kind.ord() as usize, &values);
    }
    arrays
}

fn attach_dome(camera: &mut Gd<Camera3D>, material: &Gd<ShaderMaterial>) {
    let mut node = MeshInstance3D::new_alloc();
    node.set_name("SkyDome");
    node.set_mesh(&create_dome_mesh());
    node.set_material_override(material);
    node.set_cast_shadows_setting(ShadowCastingSetting::OFF);
    node.set_ignore_occlusion_culling(true);
    // Match the original camera-child identity transform (translation AND rotation).
    camera.add_child(&node);
}

fn sky_stops(colors: &SkyColors) -> [[f32; 3]; 7] {
    [
        colors.sky_top,
        colors.sky_middle,
        colors.sky_band1,
        colors.sky_band2,
        colors.sky_smog,
        colors.fog_color,
        colors.fog_color,
    ]
}

fn create_cubemap(colors: &SkyColors) -> Result<Gd<Cubemap>, String> {
    // Initialize directly to settled noon: original default_sky_colors cube is
    // replaced by update_sky_env_map's first LightParams 12 noon sample.
    let pixels = sky_cubemap_data::build_sky_cubemap(&sky_stops(colors));
    let size = sky_cubemap_data::ENV_MAP_SIZE as i32;
    let face_bytes = size as usize * size as usize * 8;
    let mut faces = Array::<Gd<Image>>::new();
    for pixels in pixels.chunks_exact(face_bytes) {
        let image = Image::create_from_data(
            size,
            size,
            false,
            image::Format::RGBAH,
            &PackedByteArray::from(pixels),
        )
        .ok_or("Godot rejected debug sky cubemap face")?;
        faces.push(&image);
    }
    let mut cube = Cubemap::new_gd();
    let error = cube.create_from_images(&faces);
    if error != godot::global::Error::OK {
        return Err(format!("Godot rejected debug sky cubemap: {error:?}"));
    }
    Ok(cube)
}

// Exact pure cloud generator port; Vec2/IVec2 are glam instead of Bevy re-exports.
fn generate_cloud_pixels(width: u32, height: u32, seed: u32) -> Vec<u8> {
    (0..width * height)
        .flat_map(|pixel| {
            let u = (pixel % width) as f32 / width as f32;
            let v = (pixel / width) as f32 / height as f32;
            let value = (cloud_density(u, v, seed) * 255.0).round() as u8;
            [value, value, value, 255]
        })
        .collect()
}

fn cloud_density(u: f32, v: f32, seed: u32) -> f32 {
    let noise = fbm_periodic(Vec2::new(u * 7.0, v * 9.0), IVec2::new(7, 9), seed);
    let ridges = fbm_periodic(
        Vec2::new(u * 15.0 + 17.3, v * 13.0 - 11.1),
        IVec2::new(15, 13),
        seed ^ CLOUD_RIDGE_SEED_MIX,
    );
    let combined = (noise * 0.72 + (1.0 - (ridges * 2.0 - 1.0).abs()) * 0.28).clamp(0.0, 1.0);
    combined.powf(1.35)
}

fn fbm_periodic(point: Vec2, period: IVec2, seed: u32) -> f32 {
    let mut sum = 0.0;
    let mut amp = 0.55;
    let mut frequency = 1;
    let mut norm = 0.0;
    for octave in 0..CLOUD_OCTAVES {
        let phase = Vec2::new(octave as f32 * 13.1, octave as f32 * -9.7);
        sum += periodic_gradient_noise(
            point * frequency as f32 + phase,
            period * frequency,
            seed.wrapping_add(octave as u32 * 31),
        ) * amp;
        norm += amp;
        amp *= 0.5;
        frequency *= 2;
    }
    ((sum / norm) * 0.5 + 0.5).clamp(0.0, 1.0)
}

fn periodic_gradient_noise(point: Vec2, period: IVec2, seed: u32) -> f32 {
    let cell = point.floor();
    let offset = point - cell;
    let x0 = (cell.x as i32).rem_euclid(period.x);
    let y0 = (cell.y as i32).rem_euclid(period.y);
    let x1 = if x0 + 1 == period.x { 0 } else { x0 + 1 };
    let y1 = if y0 + 1 == period.y { 0 } else { y0 + 1 };
    let n00 = gradient(hash2(x0, y0, seed)).dot(offset);
    let n10 = gradient(hash2(x1, y0, seed)).dot(offset - Vec2::X);
    let n01 = gradient(hash2(x0, y1, seed)).dot(offset - Vec2::Y);
    let n11 = gradient(hash2(x1, y1, seed)).dot(offset - Vec2::ONE);
    let weight = Vec2::new(noise_blend_weight(offset.x), noise_blend_weight(offset.y));
    let lower = n00 + weight.x * (n10 - n00);
    let upper = n01 + weight.x * (n11 - n01);
    lower + weight.y * (upper - lower)
}

fn noise_blend_weight(t: f32) -> f32 {
    t * t * t * (t * (t * 6.0 - 15.0) + 10.0)
}

fn hash2(i: i32, j: i32, seed: u32) -> u32 {
    let mut h = seed
        .wrapping_add((i as u32).wrapping_mul(GRADIENT_HASH_X_MIX))
        .wrapping_add((j as u32).wrapping_mul(GRADIENT_HASH_Y_MIX));
    h ^= h >> 13;
    h = h.wrapping_mul(GRADIENT_HASH_FINAL_MIX);
    h ^ (h >> 16)
}

fn gradient(hash: u32) -> Vec2 {
    match hash & 7 {
        0 => Vec2::new(1.0, 1.0),
        1 => Vec2::new(-1.0, 1.0),
        2 => Vec2::new(1.0, -1.0),
        3 => Vec2::new(-1.0, -1.0),
        4 => Vec2::new(1.0, 0.0),
        5 => Vec2::new(-1.0, 0.0),
        6 => Vec2::new(0.0, 1.0),
        _ => Vec2::new(0.0, -1.0),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn dome_preserves_original_ring_profile_and_band_coordinates() {
        let vertices = dome_vertices();
        assert_eq!(vertices.len(), 231);
        assert!((vertices[0].0[1] - 146.44661).abs() < 0.0001);
        assert_eq!(vertices[0].1, [0.0, 0.0]);
        assert!((vertices[230].0[1] + 853.5534).abs() < 0.0001);
        assert_eq!(vertices[230].1, [1.0, 1.0]);
        for ring in vertices.chunks_exact(33) {
            for axis in 0..3 {
                assert!((ring[0].0[axis] - ring[32].0[axis]).abs() < 0.001);
            }
        }
        assert_eq!(dome_indices().len(), 1152);
    }

    #[test]
    fn cloud_definition_repeats_without_losing_high_seed_detail() {
        for seed in [0, 1, 2, 11, 0x8000_0000, u32::MAX] {
            let center = cloud_density(0.125, 0.25, seed);
            let wrapped = cloud_density(1.125, -0.75, seed);
            let neighbor = cloud_density(0.125 + 1.0 / 1024.0, 0.25 + 1.0 / 1024.0, seed);
            assert!((center - wrapped).abs() < 0.00002);
            assert!((center - neighbor).abs() > 0.000001);
        }
    }

    #[test]
    fn cloud_pixels_are_rgba_deterministic_and_seeded() {
        let pixels = generate_cloud_pixels(32, 64, 0);
        assert_eq!(pixels.len(), 32 * 64 * 4);
        assert_eq!(pixels, generate_cloud_pixels(32, 64, 0));
        assert_ne!(pixels, generate_cloud_pixels(32, 64, 1));
        assert!(
            pixels
                .chunks_exact(4)
                .all(|p| p[0] == p[1] && p[1] == p[2] && p[3] == 255)
        );
        assert!(pixels.chunks_exact(4).any(|p| p[0] != pixels[0]));
    }
}
