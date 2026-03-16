#import bevy_pbr::forward_io::VertexOutput

struct M2EffectSettings {
    transparency: f32,
    alpha_test: f32,
    shader_id: u32,
    uv_mode_1: u32,
    uv_mode_2: u32,
    _pad0: u32,
    uv_offset_1: vec2<f32>,
    uv_offset_2: vec2<f32>,
}

@group(#{MATERIAL_BIND_GROUP}) @binding(0) var<uniform> settings: M2EffectSettings;
@group(#{MATERIAL_BIND_GROUP}) @binding(1) var base_texture: texture_2d<f32>;
@group(#{MATERIAL_BIND_GROUP}) @binding(2) var base_sampler: sampler;
@group(#{MATERIAL_BIND_GROUP}) @binding(3) var second_texture: texture_2d<f32>;
@group(#{MATERIAL_BIND_GROUP}) @binding(4) var second_sampler: sampler;

fn combine_textures(texture1: vec4<f32>, texture2: vec4<f32>, shader_id: u32) -> vec4<f32> {
    switch shader_id {
        case 0x4014u: {
            return clamp(texture1 * texture2 * vec4<f32>(2.0), vec4<f32>(0.0), vec4<f32>(1.0));
        }
        case 0x8015u: {
            let rgb = texture1.rgb + texture2.rgb * texture2.a;
            return vec4<f32>(rgb, 1.0);
        }
        default: {
            return texture1;
        }
    }
}

@fragment
fn fragment(in: VertexOutput) -> @location(0) vec4<f32> {
    let uv1 = select(in.uv, in.uv_b, settings.uv_mode_1 == 1u) + settings.uv_offset_1;
    let uv2 = select(in.uv, in.uv_b, settings.uv_mode_2 == 1u) + settings.uv_offset_2;
    let texture1 = textureSample(base_texture, base_sampler, uv1);
    let texture2 = textureSample(second_texture, second_sampler, uv2);
    var color = combine_textures(texture1, texture2, settings.shader_id);
    color.a = clamp(color.a * settings.transparency, 0.0, 1.0);
    if color.a < settings.alpha_test {
        discard;
    }
    return color;
}
