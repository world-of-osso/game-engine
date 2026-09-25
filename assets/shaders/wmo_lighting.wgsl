// Retail WMO lighting (see terrain_objects_wmo_lighting.rs): with fixed, sRGB-encoded
// MOCV `m` and alpha `a`,
//   exterior = texture * (scene daylight + 2 * m)
//   interior = texture * (interior ambient + 2 * m)
//   out      = mix(interior, exterior, exterior_lit ? 1 : a)
// and an unlit material shows the texture alone. Two-layer shaders (MOMT 6, 13)
// blend a second texture by the second MOCV alpha, which needs its own vertex input.
#import bevy_pbr::{
    forward_io::{VertexOutput, FragmentOutput},
    mesh_functions,
    pbr_fragment::pbr_input_from_standard_material,
    pbr_functions::{alpha_discard, apply_pbr_lighting, main_pass_post_lighting_processing},
    view_transformations::position_world_to_clip,
}

// MOMT shader, mirrors terrain_objects_wmo_lighting.rs; the other one is 6
// (TwoLayerDiffuse).
const TWO_LAYER_DIFFUSE_OPAQUE: u32 = 13u;

struct WmoLighting {
    interior_ambient: vec4<f32>,
    exterior_lit: u32,
    unlit: u32,
    two_layer_shader: u32,
}

@group(#{MATERIAL_BIND_GROUP}) @binding(100) var<uniform> lighting: WmoLighting;
@group(#{MATERIAL_BIND_GROUP}) @binding(101) var second_texture: texture_2d<f32>;
@group(#{MATERIAL_BIND_GROUP}) @binding(102) var second_sampler: sampler;

// forward_io's Vertex/VertexOutput plus the second MOCV alpha.
struct WmoVertex {
    @builtin(instance_index) instance_index: u32,
#ifdef VERTEX_POSITIONS
    @location(0) position: vec3<f32>,
#endif
#ifdef VERTEX_NORMALS
    @location(1) normal: vec3<f32>,
#endif
#ifdef VERTEX_UVS_A
    @location(2) uv: vec2<f32>,
#endif
#ifdef VERTEX_UVS_B
    @location(3) uv_b: vec2<f32>,
#endif
#ifdef VERTEX_TANGENTS
    @location(4) tangent: vec4<f32>,
#endif
#ifdef VERTEX_COLORS
    @location(5) color: vec4<f32>,
#endif
#ifdef WMO_SECOND_MOCV
    @location(8) second_mocv_alpha: f32,
#endif
}

struct WmoVertexOutput {
    @builtin(position) position: vec4<f32>,
    @location(0) world_position: vec4<f32>,
    @location(1) world_normal: vec3<f32>,
#ifdef VERTEX_UVS_A
    @location(2) uv: vec2<f32>,
#endif
#ifdef VERTEX_UVS_B
    @location(3) uv_b: vec2<f32>,
#endif
#ifdef VERTEX_TANGENTS
    @location(4) world_tangent: vec4<f32>,
#endif
#ifdef VERTEX_COLORS
    @location(5) color: vec4<f32>,
#endif
#ifdef VERTEX_OUTPUT_INSTANCE_INDEX
    @location(6) @interpolate(flat) instance_index: u32,
#endif
#ifdef VISIBILITY_RANGE_DITHER
    @location(7) @interpolate(flat) visibility_range_dither: i32,
#endif
    @location(8) second_mocv_alpha: f32,
}

// bevy_pbr mesh.wgsl `vertex` without skinning and morph targets.
@vertex
fn vertex(vertex: WmoVertex) -> WmoVertexOutput {
    var out: WmoVertexOutput;
    let world_from_local = mesh_functions::get_world_from_local(vertex.instance_index);
#ifdef VERTEX_NORMALS
    out.world_normal = mesh_functions::mesh_normal_local_to_world(vertex.normal, vertex.instance_index);
#endif
#ifdef VERTEX_POSITIONS
    out.world_position = mesh_functions::mesh_position_local_to_world(world_from_local, vec4(vertex.position, 1.0));
    out.position = position_world_to_clip(out.world_position.xyz);
#endif
#ifdef VERTEX_UVS_A
    out.uv = vertex.uv;
#endif
#ifdef VERTEX_UVS_B
    out.uv_b = vertex.uv_b;
#endif
#ifdef VERTEX_TANGENTS
    out.world_tangent = mesh_functions::mesh_tangent_local_to_world(
        world_from_local,
        vertex.tangent,
        vertex.instance_index
    );
#endif
#ifdef VERTEX_COLORS
    out.color = vertex.color;
#endif
#ifdef VERTEX_OUTPUT_INSTANCE_INDEX
    out.instance_index = vertex.instance_index;
#endif
#ifdef VISIBILITY_RANGE_DITHER
    out.visibility_range_dither = mesh_functions::get_visibility_range_dither_level(
        vertex.instance_index, world_from_local[3]);
#endif
    // Without a second MOCV Retail uses alpha 255: the first layer alone.
    out.second_mocv_alpha = 1.0;
#ifdef WMO_SECOND_MOCV
    out.second_mocv_alpha = vertex.second_mocv_alpha;
#endif
    return out;
}

fn forward_vertex_output(wmo: WmoVertexOutput) -> VertexOutput {
    var out: VertexOutput;
    out.position = wmo.position;
    out.world_position = wmo.world_position;
    out.world_normal = wmo.world_normal;
#ifdef VERTEX_UVS_A
    out.uv = wmo.uv;
#endif
#ifdef VERTEX_UVS_B
    out.uv_b = wmo.uv_b;
#endif
#ifdef VERTEX_TANGENTS
    out.world_tangent = wmo.world_tangent;
#endif
#ifdef VERTEX_COLORS
    out.color = wmo.color;
#endif
#ifdef VERTEX_OUTPUT_INSTANCE_INDEX
    out.instance_index = wmo.instance_index;
#endif
#ifdef VISIBILITY_RANGE_DITHER
    out.visibility_range_dither = wmo.visibility_range_dither;
#endif
    return out;
}

// WoW sums MOCV and ambient in gamma space; the Bevy scene is linear.
fn srgb_to_linear(color: vec3<f32>) -> vec3<f32> {
    let low = color / 12.92;
    let high = pow((max(color, vec3(0.0)) + 0.055) / 1.055, vec3(2.4));
    return select(high, low, color <= vec3(0.04045));
}

fn linear_to_srgb(color: vec3<f32>) -> vec3<f32> {
    let low = color * 12.92;
    let high = 1.055 * pow(max(color, vec3(0.0)), vec3(1.0 / 2.4)) - 0.055;
    return select(high, low, color <= vec3(0.0031308));
}

// Retail two-layer diffuse (WebWowViewerCpp caclWMOFragMat), mixed in gamma space.
fn two_layer_diffuse(first: vec4<f32>, uv: vec2<f32>, second_mocv_alpha: f32) -> vec4<f32> {
    let second = textureSample(second_texture, second_sampler, uv);
    let layer1 = linear_to_srgb(first.rgb);
    let layer2 = linear_to_srgb(second.rgb);
    if lighting.two_layer_shader == TWO_LAYER_DIFFUSE_OPAQUE {
        return vec4(srgb_to_linear(mix(layer2, layer1, second_mocv_alpha)), 1.0);
    }
    let under = mix(layer1, layer2, second.a);
    return vec4(srgb_to_linear(mix(under, layer1, second_mocv_alpha)), first.a);
}

@fragment
fn fragment(wmo: WmoVertexOutput, @builtin(front_facing) is_front: bool) -> FragmentOutput {
    let vertex = forward_vertex_output(wmo);
    var in = vertex;
    // Without MOCV Retail uses color 0 and alpha 0.
    var authored = vec3(0.0);
    var exterior_blend = 0.0;
#ifdef VERTEX_COLORS
    authored = 2.0 * vertex.color.rgb;
    exterior_blend = vertex.color.a;
    // Keep MOCV out of StandardMaterial's base color, which would multiply it in.
    in.color = vec4(1.0);
#endif
    if lighting.exterior_lit != 0u {
        exterior_blend = 1.0;
    }
    var pbr_input = pbr_input_from_standard_material(in, is_front);
    if lighting.two_layer_shader != 0u {
        // Retail substitutes (1, 1) for a missing second MOTV.
        var second_uv = vec2(1.0);
#ifdef VERTEX_UVS_B
        second_uv = vertex.uv_b;
#endif
        pbr_input.material.base_color =
            two_layer_diffuse(pbr_input.material.base_color, second_uv, wmo.second_mocv_alpha);
    }
    pbr_input.material.base_color = alpha_discard(pbr_input.material, pbr_input.material.base_color);
    let albedo = pbr_input.material.base_color;
    let emissive = pbr_input.material.emissive.rgb;

    var out: FragmentOutput;
    if lighting.unlit != 0u {
        out.color = vec4(albedo.rgb + emissive, albedo.a);
    } else {
        let daylight = apply_pbr_lighting(pbr_input);
        let exterior = daylight.rgb + albedo.rgb * srgb_to_linear(authored);
        let interior =
            albedo.rgb * srgb_to_linear(lighting.interior_ambient.rgb + authored) + emissive;
        out.color = vec4(mix(interior, exterior, exterior_blend), daylight.a);
    }
    out.color = main_pass_post_lighting_processing(pbr_input, out.color);
    return out;
}
