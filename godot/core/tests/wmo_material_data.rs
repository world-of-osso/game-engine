use game_engine_core::wmo_material_data::{
    WmoLayerCombine, WmoShaderDescriptor, WmoSurfaceParams, composite_wmo_shader_layer,
    describe_wmo_shader, wmo_surface_params,
};

#[test]
fn authored_shader_ids_select_cpu_layers() {
    use WmoLayerCombine::{Add, AlphaBlend, Mod2x, Multiply, None};

    for (shader, expected) in [
        (6, (false, false, false, None, None)),
        (13, (false, false, false, None, None)),
        (8, (false, false, false, AlphaBlend, None)),
        (7, (true, true, false, AlphaBlend, Add)),
        (11, (true, true, false, AlphaBlend, AlphaBlend)),
        (12, (true, true, true, Add, Add)),
        (18, (false, false, false, Mod2x, None)),
        (22, (true, true, false, Multiply, None)),
    ] {
        let descriptor = describe_wmo_shader(shader);
        assert_eq!(
            descriptor,
            WmoShaderDescriptor {
                env_reflection: expected.0,
                metallic: expected.1,
                emissive: expected.2,
                second_layer: expected.3,
                third_layer: expected.4,
            },
            "shader {shader}"
        );
    }
}

#[test]
fn authored_shader_layers_composite_exact_rgba_and_preserve_max_alpha() {
    let second = [200, 40, 20, 128];
    let third = [10, 100, 200, 64];
    for (shader, expected) in [
        (6, [100, 80, 60, 90]),
        (13, [100, 80, 60, 90]),
        (8, [150, 60, 40, 128]),
        (7, [153, 85, 90, 128]),
        (11, [115, 70, 80, 128]),
        (12, [203, 125, 120, 128]),
        (18, [157, 25, 9, 128]),
        (22, [78, 13, 5, 128]),
    ] {
        let descriptor = describe_wmo_shader(shader);
        let mut pixels = [100, 80, 60, 90];
        composite_wmo_shader_layer(&mut pixels, &second, descriptor.second_layer);
        composite_wmo_shader_layer(&mut pixels, &third, descriptor.third_layer);
        assert_eq!(pixels, expected, "shader {shader}");
    }
}

#[test]
fn authored_surface_constants_depend_on_material_and_prop_flags() {
    for (has_texture, unculled, shader, expected) in [
        (true, false, 0, (0.88, 0.18)),
        (false, false, 0, (0.97, 0.02)),
        (true, true, 0, (0.97, 0.02)),
        (true, false, 3, (0.35, 0.45)),
        (true, false, 5, (0.25, 0.5)),
    ] {
        assert_eq!(
            wmo_surface_params(has_texture, unculled, shader),
            WmoSurfaceParams {
                roughness: expected.0,
                reflectance: expected.1,
            }
        );
    }
}
