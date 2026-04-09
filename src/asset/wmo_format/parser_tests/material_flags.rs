use super::*;

fn mat_with_shader(shader: u32, flags: u32) -> WmoMaterialDef {
    WmoMaterialDef {
        texture_fdid: 0,
        texture_2_fdid: 0,
        texture_3_fdid: 0,
        flags,
        material_flags: WmoMaterialFlags::default(),
        sidn_color: [0.0; 4],
        diff_color: [0.0; 4],
        ground_type: 0,
        blend_mode: 0,
        shader,
        uv_translation_speed: None,
    }
}

#[test]
fn uses_second_uv_requires_flag_and_shader() {
    assert!(mat_with_shader(6, SECOND_UV_FLAG).uses_second_uv_set());
    assert!(mat_with_shader(9, SECOND_UV_FLAG).uses_second_uv_set());
    assert!(mat_with_shader(15, SECOND_UV_FLAG).uses_second_uv_set());
    assert!(!mat_with_shader(10, SECOND_UV_FLAG).uses_second_uv_set());
    assert!(!mat_with_shader(6, 0).uses_second_uv_set());
}

#[test]
fn uses_second_uv_all_valid_shaders() {
    for shader in [6, 7, 8, 9, 11, 12, 13, 14, 15] {
        assert!(
            mat_with_shader(shader, SECOND_UV_FLAG).uses_second_uv_set(),
            "shader {shader} should use second UV"
        );
    }
}

#[test]
fn uses_second_uv_invalid_shaders() {
    for shader in [0, 1, 2, 3, 4, 5, 10, 16, 17, 18, 100] {
        assert!(
            !mat_with_shader(shader, SECOND_UV_FLAG).uses_second_uv_set(),
            "shader {shader} should NOT use second UV"
        );
    }
}

#[test]
fn uses_generated_tangents_shaders_10_and_14() {
    assert!(mat_with_shader(10, 0).uses_generated_tangents());
    assert!(mat_with_shader(14, 0).uses_generated_tangents());
    assert!(!mat_with_shader(0, 0).uses_generated_tangents());
    assert!(!mat_with_shader(6, 0).uses_generated_tangents());
    assert!(!mat_with_shader(15, 0).uses_generated_tangents());
}

#[test]
fn uses_third_uv_requires_flag_and_shader_18() {
    assert!(mat_with_shader(18, THIRD_UV_FLAG).uses_third_uv_set());
    assert!(!mat_with_shader(6, THIRD_UV_FLAG).uses_third_uv_set());
    assert!(!mat_with_shader(18, 0).uses_third_uv_set());
}

#[test]
fn uses_second_color_blend_alpha() {
    assert!(mat_with_shader(0, SECOND_COLOR_BLEND_ALPHA_FLAG).uses_second_color_blend_alpha());
    assert!(!mat_with_shader(0, 0).uses_second_color_blend_alpha());
    assert!(mat_with_shader(18, SECOND_COLOR_BLEND_ALPHA_FLAG).uses_second_color_blend_alpha());
}

#[test]
fn material_flags_from_bits_selective() {
    let flags = WmoMaterialFlags::from_bits(0x01 | 0x04 | 0x10 | 0x40);
    assert!(flags.unlit);
    assert!(!flags.unfogged);
    assert!(flags.unculled);
    assert!(!flags.exterior_light);
    assert!(flags.sidn);
    assert!(flags.window);
    assert!(!flags.clamp_s);
    assert!(flags.clamp_t);
}

#[test]
fn material_flags_all_zero() {
    let flags = WmoMaterialFlags::from_bits(0);
    assert_eq!(flags, WmoMaterialFlags::default());
}

#[test]
fn material_flags_all_set() {
    let flags = WmoMaterialFlags::from_bits(0x7F);
    assert!(flags.unlit);
    assert!(flags.unfogged);
    assert!(flags.unculled);
    assert!(flags.exterior_light);
    assert!(flags.sidn);
    assert!(flags.window);
    assert!(flags.clamp_s);
    assert!(flags.clamp_t);
}
