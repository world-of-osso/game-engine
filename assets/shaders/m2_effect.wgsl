// Two-texture M2 batches: texture combiners, then Retail lighting and fog in
// authored space (WebWowViewerCpp bindless/m2/m2shader_text.slang).
#import bevy_pbr::{
    forward_io::VertexOutput,
    mesh_bindings::mesh,
    pbr_functions,
    pbr_types,
}
#import "shaders/retail_lighting.wgsl"::{
    RetailSceneLight,
    gamma_to_linear,
    linear_to_gamma,
    retail_apply_fog,
    retail_shade,
    retail_sun_visibility,
}

struct M2EffectSettings {
    transparency: f32,
    alpha_test: f32,
    shader_id: u32,
    blend_mode: u32,
    uv_mode_1: u32,
    uv_mode_2: u32,
    render_flags: u32,
    gx_blend: u32,
    uv_offset_1: vec2<f32>,
    uv_offset_2: vec2<f32>,
}

@group(#{MATERIAL_BIND_GROUP}) @binding(0) var<uniform> settings: M2EffectSettings;
@group(#{MATERIAL_BIND_GROUP}) @binding(1) var base_texture: texture_2d<f32>;
@group(#{MATERIAL_BIND_GROUP}) @binding(2) var base_sampler: sampler;
@group(#{MATERIAL_BIND_GROUP}) @binding(3) var second_texture: texture_2d<f32>;
@group(#{MATERIAL_BIND_GROUP}) @binding(4) var second_sampler: sampler;
@group(#{MATERIAL_BIND_GROUP}) @binding(5) var<storage, read> scene_light: RetailSceneLight;

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
            let rgb = (texture1.rgb) * mix(texture2.rgb * vec3<f32>(2.0), vec3<f32>(1.0), vec3<f32>(texture1.a));
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

fn alpha_mode_flags(blend_mode: u32) -> u32 {
    switch blend_mode {
        case 1u: {
            return pbr_types::STANDARD_MATERIAL_FLAGS_ALPHA_MODE_MASK;
        }
        case 2u, 3u, 7u: {
            return pbr_types::STANDARD_MATERIAL_FLAGS_ALPHA_MODE_BLEND;
        }
        case 4u, 5u, 6u: {
            return pbr_types::STANDARD_MATERIAL_FLAGS_ALPHA_MODE_ADD;
        }
        default: {
            return pbr_types::STANDARD_MATERIAL_FLAGS_ALPHA_MODE_OPAQUE;
        }
    }
}

fn gamma_texel(texel: vec4<f32>) -> vec4<f32> {
    return vec4<f32>(linear_to_gamma(texel.rgb), texel.a);
}

@fragment
fn fragment(in: VertexOutput, @builtin(front_facing) is_front: bool) -> @location(0) vec4<f32> {
    let uv1 = select(in.uv, in.uv_b, settings.uv_mode_1 == 1u) + settings.uv_offset_1;
    let uv2 = select(in.uv, in.uv_b, settings.uv_mode_2 == 1u) + settings.uv_offset_2;
    let texture1 = gamma_texel(textureSample(base_texture, base_sampler, uv1));
    let texture2 = gamma_texel(textureSample(second_texture, second_sampler, uv2));
    var color = combine_textures(texture1, texture2, settings.shader_id);
    color.a = clamp(color.a * settings.transparency, 0.0, 1.0);
    if color.a < settings.alpha_test {
        discard;
    }

    let normal = normalize(pbr_functions::prepare_world_normal(in.world_normal, true, is_front));
    var rgb = color.rgb;
    if (settings.render_flags & 0x1u) == 0u {
        let sun = retail_sun_visibility(
            in.world_position,
            normal,
            in.position.xy,
            mesh[in.instance_index].flags,
        );
        rgb = retail_shade(scene_light, rgb, normal, sun);
    }
    if (settings.render_flags & 0x2u) == 0u {
        rgb = retail_apply_fog(rgb, in.world_position.xyz, settings.gx_blend);
    }

    var pbr_input = pbr_types::pbr_input_new();
    pbr_input.material.flags = alpha_mode_flags(settings.blend_mode);
    pbr_input.frag_coord = in.position;
    pbr_input.world_position = in.world_position;
    return pbr_functions::main_pass_post_lighting_processing(
        pbr_input,
        vec4<f32>(gamma_to_linear(rgb), color.a),
    );
}
