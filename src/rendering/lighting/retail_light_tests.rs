use super::*;
use crate::sky_lightdata::{interpolate_colors, load_light_data};

fn assert_vec3(actual: Vec3, expected: [f32; 3], tolerance: f32) {
    assert!(
        actual.abs_diff_eq(Vec3::from_array(expected), tolerance),
        "{actual:?} vs {expected:?}"
    );
}

fn bytes(rgb: [u8; 3]) -> [f32; 3] {
    rgb.map(|channel| channel as f32 / 255.0)
}

fn noon_light() -> RetailSceneLight {
    let rows = load_light_data("data/LightData.ron", 12);
    RetailSceneLight::from_sky_colors(&interpolate_colors(&rows, 1440.0), 1440.0)
}

#[test]
fn noon_light_params_12_scene_light_uses_authored_bytes() {
    let light = noon_light();
    assert_vec3(light.ambient, bytes([127, 149, 170]), 1e-4);
    assert_vec3(light.direct, bytes([106, 86, 66]), 1e-4);
    // LightParams 12 authors no horizon or ground ambient: both take the ambient.
    assert_eq!(light.horizon_ambient, light.ambient);
    assert_eq!(light.ground_ambient, light.ambient);
    assert_vec3(light.fog_color, bytes([77, 120, 143]), 1e-4);
    assert!((light.fog_start - 125.0).abs() < 0.01);
    assert!((light.fog_end - 500.0).abs() < 0.01);
}

#[test]
fn authored_horizon_and_ground_ambients_are_kept() {
    let mut colors = interpolate_colors(&load_light_data("data/LightData.ron", 12), 1440.0);
    colors.horizon_ambient_color = Color::srgb_u8(10, 20, 30);
    colors.ground_ambient_color = Color::srgb_u8(40, 50, 60);
    let light = RetailSceneLight::from_sky_colors(&colors, 1440.0);
    assert_vec3(light.horizon_ambient, bytes([10, 20, 30]), 1e-4);
    assert_vec3(light.ground_ambient, bytes([40, 50, 60]), 1e-4);
}

#[test]
fn sun_direction_follows_client_directional_light_tables() {
    // Noon: phi 2.2165682, theta 3.9269907 -> WoW (-0.5647, -0.5647, -0.6018).
    assert_vec3(
        retail_sun_direction(1440.0),
        [-0.564_72, -0.601_82, 0.564_72],
        1e-4,
    );
    // Dawn (day 0.25): phi 1.9198622 -> the light comes from 20° above the horizon.
    assert_vec3(
        retail_sun_direction(720.0),
        [-0.664_46, -0.342_02, 0.664_46],
        1e-4,
    );
    // Halfway between the two table keys.
    let phi = (1.919_862_2f32 + 2.216_568_2) / 2.0;
    assert!((retail_sun_direction(1080.0).y - phi.cos()).abs() < 1e-4);
}

#[test]
fn retail_shade_matches_calc_light_for_a_concrete_texel() {
    // Up-facing texel (0.5, 0.4, 0.3) at noon: nDotUp 1, nDotL 0.60182.
    let light = noon_light();
    let lit = retail_shade(&light, Vec3::new(0.5, 0.4, 0.3), Vec3::Y, 1.0);
    assert_vec3(lit, [0.379_175, 0.319_672, 0.250_802], 1e-4);
    // Sun shadowed to a quarter: only the direct term scales.
    let shadowed = retail_shade(&light, Vec3::new(0.5, 0.4, 0.3), Vec3::Y, 0.25);
    assert_vec3(shadowed, [0.285_362, 0.258_782, 0.215_755], 1e-4);
}

#[test]
fn retail_shade_blends_horizon_and_ground_ambients_by_normal() {
    let light = RetailSceneLight {
        ambient: Vec3::new(1.0, 0.0, 0.0),
        horizon_ambient: Vec3::new(0.0, 1.0, 0.0),
        ground_ambient: Vec3::new(0.0, 0.0, 1.0),
        direct: Vec3::ZERO,
        sun_direction: Vec3::NEG_Y,
        fog_color: Vec3::ZERO,
        fog_start: 0.0,
        fog_end: 1.0,
    };
    // Horizontal normal: nDotUp 0 -> horizon ambient; nDotL 0 -> ×0.9.
    assert_vec3(
        retail_shade(&light, Vec3::ONE, Vec3::X, 1.0),
        [0.0, 0.9, 0.0],
        1e-5,
    );
    // Down-facing: nDotUp -1 -> ground ambient; nDotL 0 -> ×0.9.
    assert_vec3(
        retail_shade(&light, Vec3::ONE, Vec3::NEG_Y, 1.0),
        [0.0, 0.0, 0.9],
        1e-5,
    );
    // Up-facing towards the sun: nDotL 1 -> ×1.1.
    assert_vec3(
        retail_shade(&light, Vec3::ONE, Vec3::Y, 1.0),
        [1.1, 0.0, 0.0],
        1e-5,
    );
}

#[test]
fn scene_light_uploads_to_the_shared_buffer_in_place() {
    let mut app = App::new();
    app.add_plugins((bevy::app::TaskPoolPlugin::default(), AssetPlugin::default()))
        .init_asset::<ShaderBuffer>()
        .insert_resource(noon_light())
        .add_systems(Update, upload_retail_scene_light);
    app.update();
    let read = |app: &App| -> Vec<u8> {
        app.world()
            .resource::<Assets<ShaderBuffer>>()
            .get(&RETAIL_SCENE_LIGHT_BUFFER)
            .and_then(|buffer| buffer.data.clone())
            .expect("scene light buffer data")
    };
    let expected = |light: &RetailSceneLight| -> Vec<u8> {
        ShaderBuffer::from(RetailSceneLightUniform::from(light))
            .data
            .expect("encoded uniform")
    };
    assert_eq!(read(&app), expected(&noon_light()));

    let dusk = {
        let mut light = noon_light();
        light.direct = Vec3::new(0.2, 0.1, 0.05);
        light
    };
    app.insert_resource(dusk.clone());
    app.update();
    assert_eq!(read(&app), expected(&dusk));
    assert_eq!(app.world().resource::<Assets<ShaderBuffer>>().len(), 1);
}
