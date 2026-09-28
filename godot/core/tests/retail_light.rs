use game_engine_core::retail_light_data::{RetailLightColors, scene_light, shade, sun_direction};

fn assert_rgb(actual: [f32; 3], expected: [f32; 3]) {
    for (actual, expected) in actual.into_iter().zip(expected) {
        assert!((actual - expected).abs() < 1e-4, "{actual} != {expected}");
    }
}

fn colors() -> RetailLightColors {
    RetailLightColors {
        ambient: [0.5, 0.6, 0.7],
        horizon_ambient: [0.0; 3],
        ground_ambient: [0.0; 3],
        direct: [0.4, 0.3, 0.2],
        fog_color: [0.2, 0.3, 0.4],
        fog_start: 125.0,
        fog_end: 500.0,
    }
}

#[test]
fn scene_light_preserves_authored_colors_and_fog_with_zero_ambient_fallback() {
    let light = scene_light(&colors(), 1440.0);
    assert_eq!(light.ambient, [0.5, 0.6, 0.7]);
    assert_eq!(light.horizon_ambient, light.ambient);
    assert_eq!(light.ground_ambient, light.ambient);
    assert_eq!(light.direct, [0.4, 0.3, 0.2]);
    assert_eq!(light.fog_color, [0.2, 0.3, 0.4]);
    assert_eq!((light.fog_start, light.fog_end), (125.0, 500.0));
    let authored = RetailLightColors {
        horizon_ambient: [0.1, 0.2, 0.3],
        ground_ambient: [0.4, 0.5, 0.6],
        ..colors()
    };
    let light = scene_light(&authored, 1440.0);
    assert_eq!(light.horizon_ambient, authored.horizon_ambient);
    assert_eq!(light.ground_ambient, authored.ground_ambient);
}

#[test]
fn direction_matches_authored_angles_at_noon_and_dawn_with_cyclic_day_wrap() {
    assert_rgb(sun_direction(1440.0), [-0.564_72, -0.601_82, 0.564_72]);
    assert_rgb(sun_direction(720.0), [-0.664_46, -0.342_02, 0.664_46]);
    assert_rgb(sun_direction(1080.0), [-0.621_42, -0.477_16, 0.621_42]);
    assert_rgb(sun_direction(2880.0), sun_direction(0.0));
    assert_rgb(sun_direction(-2160.0), sun_direction(720.0));
}

#[test]
fn hemisphere_shades_up_horizon_and_ground_normals() {
    let mut light = scene_light(
        &RetailLightColors {
            ambient: [1.0, 0.0, 0.0],
            horizon_ambient: [0.0, 1.0, 0.0],
            ground_ambient: [0.0, 0.0, 1.0],
            direct: [0.0; 3],
            ..colors()
        },
        1440.0,
    );
    light.sun_direction = [0.0, -1.0, 0.0];
    assert_rgb(
        shade(&light, [1.0; 3], [1.0, 0.0, 0.0], 1.0),
        [0.0, 0.9, 0.0],
    );
    assert_rgb(
        shade(&light, [1.0; 3], [0.0, -1.0, 0.0], 1.0),
        [0.0, 0.0, 0.9],
    );
    let up = shade(&light, [1.0; 3], [0.0, 1.0, 0.0], 1.0);
    assert_rgb(up, [1.1, 0.0, 0.0]);
}

#[test]
fn direct_sunlight_obeys_shadow_visibility_without_dimming_ambient() {
    let light = scene_light(&colors(), 1440.0);
    let diffuse = [0.5, 0.4, 0.3];
    let normal = [0.0, 1.0, 0.0];
    let shadowed = shade(&light, diffuse, normal, 0.0);
    let lit = shade(&light, diffuse, normal, 1.0);
    assert_rgb(shadowed, [0.255_091, 0.244_887, 0.214_276]);
    for (((lit, shadowed), direct), diffuse) in
        lit.into_iter().zip(shadowed).zip(light.direct).zip(diffuse)
    {
        assert!((lit - shadowed - direct * 0.601_82 * diffuse).abs() < 1e-4);
    }
}
