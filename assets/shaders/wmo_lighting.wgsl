// WMO batches that light with MOCV (fixed, sRGB-encoded). With light L chosen by mode:
// DaylightTimesMocv: texture * 2 * MOCV * daylight (ordinary MapObj exterior);
// DaylightPlusMocv / RootAmbientPlusMocv: texture * (2 * MOCV + L) (unified MapObj);
// Authored: texture * 2 * MOCV.
#import bevy_pbr::{
    forward_io::{VertexOutput, FragmentOutput},
    pbr_fragment::pbr_input_from_standard_material,
    pbr_functions::{alpha_discard, apply_pbr_lighting, main_pass_post_lighting_processing},
}

// Mirrors WmoLightingMode in terrain_objects_wmo_lighting.rs; Authored is 0.
const MODE_DAYLIGHT_PLUS_MOCV: u32 = 1u;
const MODE_ROOT_AMBIENT_PLUS_MOCV: u32 = 2u;
const MODE_DAYLIGHT_TIMES_MOCV: u32 = 3u;

struct WmoLighting {
    // MOHD ambient, sRGB-encoded like MOCV.
    root_ambient: vec4<f32>,
    mode: u32,
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
    var authored = vec3(0.0);
#ifdef VERTEX_COLORS
    authored = 2.0 * vertex.color.rgb;
    // StandardMaterial multiplies vertex color into base color: only the
    // multiplicative mode keeps MOCV there, as linear light.
    in.color = vec4(1.0);
    if lighting.mode == MODE_DAYLIGHT_TIMES_MOCV {
        in.color = vec4(srgb_to_linear(authored), 1.0);
    }
#endif
    var pbr_input = pbr_input_from_standard_material(in, is_front);
    pbr_input.material.base_color = alpha_discard(pbr_input.material, pbr_input.material.base_color);
    let albedo = pbr_input.material.base_color;

    var out: FragmentOutput;
    if lighting.mode == MODE_DAYLIGHT_TIMES_MOCV {
        out.color = apply_pbr_lighting(pbr_input);
    } else if lighting.mode == MODE_DAYLIGHT_PLUS_MOCV {
        let lit = apply_pbr_lighting(pbr_input);
        out.color = vec4(lit.rgb + albedo.rgb * srgb_to_linear(authored), lit.a);
    } else {
        var light = authored;
        if lighting.mode == MODE_ROOT_AMBIENT_PLUS_MOCV {
            light += lighting.root_ambient.rgb;
        }
        let emissive = pbr_input.material.emissive.rgb;
        out.color = vec4(albedo.rgb * srgb_to_linear(light) + emissive, albedo.a);
    }
    out.color = main_pass_post_lighting_processing(pbr_input, out.color);
    return out;
}
