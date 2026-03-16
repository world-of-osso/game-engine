#import bevy_pbr::{
    forward_io::VertexOutput,
    mesh_view_bindings as view_bindings,
    pbr_functions,
    pbr_types,
}

struct M2EffectSettings {
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

fn m2_fog_color(blend_mode: u32) -> vec3<f32> {
    switch blend_mode {
        case 4u: {
            return vec3<f32>(0.0);
        }
        case 5u: {
            return vec3<f32>(1.0);
        }
        case 6u: {
            return vec3<f32>(0.5);
        }
        default: {
            return view_bindings::fog.base_color.rgb;
        }
    }
}

fn apply_m2_distance_fog(color: vec4<f32>, world_position: vec4<f32>) -> vec4<f32> {
#ifdef DISTANCE_FOG
    if (settings.render_flags & 0x2u) != 0u {
        return color;
    }
    var fog = view_bindings::fog;
    fog.base_color = vec4<f32>(m2_fog_color(settings.blend_mode), fog.base_color.a);
    fog.directional_light_color = vec4<f32>(0.0);
    return pbr_functions::apply_fog(
        fog,
        color,
        world_position.xyz,
        view_bindings::view.world_position.xyz,
    );
#else
    return color;
#endif
}

@fragment
fn fragment(in: VertexOutput, @builtin(front_facing) is_front: bool) -> @location(0) vec4<f32> {
    let uv1 = select(in.uv, in.uv_b, settings.uv_mode_1 == 1u) + settings.uv_offset_1;
    let uv2 = select(in.uv, in.uv_b, settings.uv_mode_2 == 1u) + settings.uv_offset_2;
    let texture1 = textureSample(base_texture, base_sampler, uv1);
    let texture2 = textureSample(second_texture, second_sampler, uv2);
    var color = combine_textures(texture1, texture2, settings.shader_id);
    color.a = clamp(color.a * settings.transparency, 0.0, 1.0);
    if color.a < settings.alpha_test {
        discard;
    }

    var pbr_input = pbr_types::pbr_input_new();
    pbr_input.material.base_color = color;
    pbr_input.material.perceptual_roughness = 1.0;
    pbr_input.material.reflectance = vec3<f32>(0.0);
    pbr_input.material.flags = alpha_mode_flags(settings.blend_mode);
    if (settings.render_flags & 0x1u) != 0u {
        pbr_input.material.flags |= pbr_types::STANDARD_MATERIAL_FLAGS_UNLIT_BIT;
    }
    pbr_input.frag_coord = in.position;
    pbr_input.world_position = in.world_position;
    pbr_input.world_normal = pbr_functions::prepare_world_normal(in.world_normal, true, is_front);
    pbr_input.N = normalize(pbr_input.world_normal);
    pbr_input.is_orthographic = view_bindings::view.clip_from_view[3].w == 1.0;
    pbr_input.V = pbr_functions::calculate_view(
        in.world_position,
        pbr_input.is_orthographic,
    );

    var lit = pbr_functions::apply_pbr_lighting(pbr_input);
    lit = apply_m2_distance_fog(lit, in.world_position);
    return pbr_functions::main_pass_post_lighting_processing(pbr_input, lit);
}
