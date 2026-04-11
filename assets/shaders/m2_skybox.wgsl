#import bevy_pbr::forward_io::VertexOutput;

struct SkyboxM2Settings {
    color: vec4<f32>,
    transparency: f32,
    alpha_test: f32,
    shader_id: u32,
    blend_mode: u32,
    uv_mode_1: u32,
    uv_mode_2: u32,
    render_flags: u32,
    uv_offset_1: vec2<f32>,
    uv_offset_2: vec2<f32>,
}

@group(#{MATERIAL_BIND_GROUP}) @binding(0) var<uniform> material: SkyboxM2Settings;
@group(#{MATERIAL_BIND_GROUP}) @binding(1) var base_texture: texture_2d<f32>;
@group(#{MATERIAL_BIND_GROUP}) @binding(2) var base_sampler: sampler;
@group(#{MATERIAL_BIND_GROUP}) @binding(3) var second_texture: texture_2d<f32>;
@group(#{MATERIAL_BIND_GROUP}) @binding(4) var second_sampler: sampler;

fn combine_textures(texture1: vec4<f32>, texture2: vec4<f32>, shader_id: u32) -> vec4<f32> {
    switch shader_id {
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
        default: {
            return texture1;
        }
    }
}

@fragment
fn fragment(in: VertexOutput) -> @location(0) vec4<f32> {
    // Authored M2 skyboxes in the reference clients are rendered without distance fog.
    let uv1 = select(in.uv, in.uv_b, material.uv_mode_1 == 1u) + material.uv_offset_1;
    let uv2 = select(in.uv, in.uv_b, material.uv_mode_2 == 1u) + material.uv_offset_2;
    let texture1 = textureSample(base_texture, base_sampler, uv1);
    let texture2 = textureSample(second_texture, second_sampler, uv2);
    var color = combine_textures(texture1, texture2, material.shader_id) * material.color;
    color.a = clamp(color.a * material.transparency, 0.0, 1.0);
    if color.a < material.alpha_test {
        discard;
    }
    return color;
}
