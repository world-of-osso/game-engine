// Unified MapObj (MOHD 0x02) WMO batches. MOCV here is baked light added to the
// scene light, not albedo: final = texture * (2 * MOCV + light), with light the
// daylight (exterior), the MOHD ambient (interior) or nothing (unlit material).
#import bevy_pbr::{
    forward_io::{VertexOutput, FragmentOutput},
    pbr_fragment::pbr_input_from_standard_material,
    pbr_functions::{alpha_discard, apply_pbr_lighting, main_pass_post_lighting_processing},
}

// Mirrors WmoUnifiedLightingMode in terrain_objects_wmo_unified.rs; any other
// mode (Authored) lights with MOCV alone.
const MODE_EXTERIOR: u32 = 1u;
const MODE_ROOT_AMBIENT: u32 = 2u;

struct WmoUnifiedLighting {
    // MOHD ambient, sRGB-encoded like MOCV.
    root_ambient: vec4<f32>,
    mode: u32,
}

@group(#{MATERIAL_BIND_GROUP}) @binding(100) var<uniform> lighting: WmoUnifiedLighting;

// WoW sums MOCV and ambient in gamma space; the Bevy scene is linear.
fn srgb_to_linear(color: vec3<f32>) -> vec3<f32> {
    let low = color / 12.92;
    let high = pow((max(color, vec3(0.0)) + 0.055) / 1.055, vec3(2.4));
    return select(high, low, color <= vec3(0.04045));
}

@fragment
fn fragment(vertex: VertexOutput, @builtin(front_facing) is_front: bool) -> FragmentOutput {
    var in = vertex;
    var authored = vec3(0.0);
#ifdef VERTEX_COLORS
    // Keep MOCV out of StandardMaterial's base color, which would multiply it in.
    authored = 2.0 * vertex.color.rgb;
    in.color = vec4(1.0);
#endif
    var pbr_input = pbr_input_from_standard_material(in, is_front);
    pbr_input.material.base_color = alpha_discard(pbr_input.material, pbr_input.material.base_color);
    let albedo = pbr_input.material.base_color;

    var out: FragmentOutput;
    if lighting.mode == MODE_EXTERIOR {
        let lit = apply_pbr_lighting(pbr_input);
        out.color = vec4(lit.rgb + albedo.rgb * srgb_to_linear(authored), lit.a);
    } else {
        var light = authored;
        if lighting.mode == MODE_ROOT_AMBIENT {
            light += lighting.root_ambient.rgb;
        }
        let emissive = pbr_input.material.emissive.rgb;
        out.color = vec4(albedo.rgb * srgb_to_linear(light) + emissive, albedo.a);
    }
    out.color = main_pass_post_lighting_processing(pbr_input, out.color);
    return out;
}
