// Retail WMO lighting (see terrain_objects_wmo_lighting.rs): with fixed, sRGB-encoded
// MOCV `m` and alpha `a`,
//   exterior = texture * (scene daylight + 2 * m)
//   interior = texture * (interior ambient + 2 * m)
//   out      = mix(interior, exterior, exterior_lit ? 1 : a)
// and an unlit material shows the texture alone.
#import bevy_pbr::{
    forward_io::{VertexOutput, FragmentOutput},
    pbr_fragment::pbr_input_from_standard_material,
    pbr_functions::{alpha_discard, apply_pbr_lighting, main_pass_post_lighting_processing},
}

struct WmoLighting {
    interior_ambient: vec4<f32>,
    exterior_lit: u32,
    unlit: u32,
}

@group(#{MATERIAL_BIND_GROUP}) @binding(100) var<uniform> lighting: WmoLighting;

// WoW sums MOCV and ambient in gamma space; the Bevy scene is linear.
fn srgb_to_linear(color: vec3<f32>) -> vec3<f32> {
    let low = color / 12.92;
    let high = pow((max(color, vec3(0.0)) + 0.055) / 1.055, vec3(2.4));
    return select(high, low, color <= vec3(0.04045));
}

@fragment
fn fragment(vertex: VertexOutput, @builtin(front_facing) is_front: bool) -> FragmentOutput {
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
