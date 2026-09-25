// World M2 batches: StandardMaterial texture, vertex colour and alpha handling,
// shaded with Retail's calcLight and fogged in authored space.
// Source: WebWowViewerCpp bindless/m2/m2shader_text.slang (calcLight with
// IsAffectedByLight, makeFog unless UnFogged, fog colour by blend mode).

#import bevy_pbr::{
    forward_io::{VertexOutput, FragmentOutput},
    pbr_fragment::pbr_input_from_standard_material,
    pbr_functions::{alpha_discard, main_pass_post_lighting_processing},
}
#import "shaders/retail_lighting.wgsl"::{
    RetailSceneLight,
    gamma_to_linear,
    linear_to_gamma,
    retail_apply_fog,
    retail_shade,
    retail_sun_visibility,
}

struct RetailLitParams {
    flags: u32,
    gx_blend: u32,
}

const RETAIL_UNLIT: u32 = 1u;
const RETAIL_UNFOGGED: u32 = 2u;

@group(#{MATERIAL_BIND_GROUP}) @binding(100) var<storage, read> scene_light: RetailSceneLight;
@group(#{MATERIAL_BIND_GROUP}) @binding(101) var<uniform> retail: RetailLitParams;

@fragment
fn fragment(in: VertexOutput, @builtin(front_facing) is_front: bool) -> FragmentOutput {
    var pbr_input = pbr_input_from_standard_material(in, is_front);
    pbr_input.material.base_color = alpha_discard(pbr_input.material, pbr_input.material.base_color);

    var color = linear_to_gamma(pbr_input.material.base_color.rgb);
    if (retail.flags & RETAIL_UNLIT) == 0u {
        let sun = retail_sun_visibility(
            pbr_input.world_position,
            pbr_input.world_normal,
            pbr_input.frag_coord.xy,
            pbr_input.flags,
        );
        color = retail_shade(scene_light, color, pbr_input.N, sun);
    }
    if (retail.flags & RETAIL_UNFOGGED) == 0u {
        color = retail_apply_fog(color, pbr_input.world_position.xyz, retail.gx_blend);
    }

    var out: FragmentOutput;
    // The base material's fog is disabled; this applies alpha premultiplication
    // and in-shader tonemapping only.
    out.color = main_pass_post_lighting_processing(
        pbr_input,
        vec4<f32>(gamma_to_linear(color), pbr_input.material.base_color.a),
    );
    return out;
}
