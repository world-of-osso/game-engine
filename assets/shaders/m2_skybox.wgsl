#import bevy_pbr::forward_io::VertexOutput

struct SkyboxM2Settings {
    color: vec4<f32>,
    transparency: f32,
    alpha_test: f32,
    combine_mode: u32,
    blend_mode: u32,
    uv_mode_1: u32,
    uv_mode_2: u32,
    uv_mode_3: u32,
    uv_mode_4: u32,
    render_flags: u32,
    has_second_texture: u32,
    has_third_texture: u32,
    has_fourth_texture: u32,
    uv_offset_1: vec2<f32>,
    uv_offset_2: vec2<f32>,
}

@group(#{MATERIAL_BIND_GROUP}) @binding(0) var<uniform> material: SkyboxM2Settings;
@group(#{MATERIAL_BIND_GROUP}) @binding(1) var base_texture: texture_2d<f32>;
@group(#{MATERIAL_BIND_GROUP}) @binding(2) var base_sampler: sampler;
@group(#{MATERIAL_BIND_GROUP}) @binding(3) var second_texture: texture_2d<f32>;
@group(#{MATERIAL_BIND_GROUP}) @binding(4) var second_sampler: sampler;
@group(#{MATERIAL_BIND_GROUP}) @binding(5) var third_texture: texture_2d<f32>;
@group(#{MATERIAL_BIND_GROUP}) @binding(6) var third_sampler: sampler;
@group(#{MATERIAL_BIND_GROUP}) @binding(7) var fourth_texture: texture_2d<f32>;
@group(#{MATERIAL_BIND_GROUP}) @binding(8) var fourth_sampler: sampler;

fn combine_textures(
    texture1: vec4<f32>,
    texture2: vec4<f32>,
    texture3: vec4<f32>,
    texture4: vec4<f32>,
    combine_mode: u32,
    has_third_texture: u32,
    has_fourth_texture: u32,
) -> vec4<f32> {
    switch combine_mode {
        case 0x000Eu: {
            return texture1 * texture2;
        }
        case 0x4014u: {
            return clamp(texture1 * texture2 * vec4<f32>(2.0), vec4<f32>(0.0), vec4<f32>(1.0));
        }
        case 0x0010u: {
            return vec4<f32>(texture1.rgb * texture2.rgb, texture1.a);
        }
        case 0x0011u: {
            return texture1 * texture2;
        }
        case 0x4016u: {
            let rgb = clamp(texture1.rgb * texture2.rgb * vec3<f32>(2.0), vec3<f32>(0.0), vec3<f32>(1.0));
            return vec4<f32>(rgb, texture1.a);
        }
        case 0x8015u: {
            let rgb = texture1.rgb + texture2.rgb * texture2.a;
            return vec4<f32>(rgb, 1.0);
        }
        case 0x8001u: {
            let rgb = texture1.rgb * mix(texture2.rgb * vec3<f32>(2.0), vec3<f32>(1.0), vec3<f32>(texture1.a));
            return vec4<f32>(rgb, 1.0);
        }
        case 0x8002u: {
            let rgb = texture1.rgb + texture2.rgb * texture2.a;
            return vec4<f32>(rgb, 1.0);
        }
        case 0x8003u: {
            let rgb = texture1.rgb + texture2.rgb * texture2.a * texture1.a;
            return vec4<f32>(rgb, 1.0);
        }
        case 0x8012u: {
            let stage3 = select(texture2, texture3, has_third_texture != 0u);
            let crossfade = mix(texture2, stage3, vec4<f32>(stage3.a));
            return texture1 * crossfade;
        }
        case 0x8016u: {
            let stage3 = select(texture2, texture3, has_third_texture != 0u);
            let mask = select(stage3.a, texture4.a, has_fourth_texture != 0u);
            let crossfade = mix(texture2, stage3, vec4<f32>(mask));
            return texture1 * crossfade;
        }
        default: {
            return texture1;
        }
    }
}

fn selected_uv(in: VertexOutput, use_secondary_uv: u32, offset: vec2<f32>) -> vec2<f32> {
    var uv = in.uv;
#ifdef VERTEX_UVS_B
    if use_secondary_uv == 1u {
        uv = in.uv_b;
    }
#endif
    return uv + offset;
}

fn sample_optional_stage(
    stage_texture: texture_2d<f32>,
    stage_sampler: sampler,
    uv: vec2<f32>,
    has_texture: u32,
    fallback: vec4<f32>,
) -> vec4<f32> {
    if has_texture != 0u {
        return textureSample(stage_texture, stage_sampler, uv);
    }
    return fallback;
}

struct SkyboxFragmentOutput {
    @location(0) color: vec4<f32>,
    @builtin(frag_depth) depth: f32,
};

@fragment
fn fragment(in: VertexOutput) -> SkyboxFragmentOutput {
    let uv1 = selected_uv(in, material.uv_mode_1, material.uv_offset_1);
    let uv2 = selected_uv(in, material.uv_mode_2, material.uv_offset_2);
    let uv3 = selected_uv(in, material.uv_mode_3, vec2<f32>(0.0));
    let uv4 = selected_uv(in, material.uv_mode_4, vec2<f32>(0.0));
    let texture1 = textureSample(base_texture, base_sampler, uv1);
    var color = texture1;
    if material.has_second_texture != 0u {
        let texture2 = textureSample(second_texture, second_sampler, uv2);
        let texture3 = sample_optional_stage(
            third_texture,
            third_sampler,
            uv3,
            material.has_third_texture,
            texture2,
        );
        let texture4 = sample_optional_stage(
            fourth_texture,
            fourth_sampler,
            uv4,
            material.has_fourth_texture,
            texture3,
        );
        color = combine_textures(
            texture1,
            texture2,
            texture3,
            texture4,
            material.combine_mode,
            material.has_third_texture,
            material.has_fourth_texture,
        );
    }
    color = color * material.color;
    color.a = clamp(color.a * material.transparency, 0.0, 1.0);
    if color.a < material.alpha_test {
        discard;
    }
    // Authored geometry defines the sky image, not its distance from scene objects.
    // Bevy uses reverse-Z: zero places every sky fragment behind scene geometry.
    return SkyboxFragmentOutput(color, 0.0);
}
