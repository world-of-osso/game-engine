// Terrain shader with direct repeated sampling.
// Layers blend by MCAL alpha; height-textured maps re-weight by MHID `_h` textures.
// Lighting is Retail's (WebWowViewerCpp 1a8cccbe bindless/adt/adtShader_text.slang):
// matDiffuse = blended layers * MCCV, calcLight, plus the layer-alpha specular
// term, then fog, all in authored (gamma) space.

#import bevy_pbr::{
    forward_io::VertexOutput,
    mesh_bindings::mesh,
    mesh_view_bindings::{view, globals},
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

struct TerrainSettings {
    config: vec4<f32>,
    layer_params_0: vec4<f32>,
    layer_params_1: vec4<f32>,
    layer_params_2: vec4<f32>,
    layer_params_3: vec4<f32>,
    animation_params_0: vec4<f32>,
    animation_params_1: vec4<f32>,
    animation_params_2: vec4<f32>,
    animation_params_3: vec4<f32>,
}

// settings.config.x = layer_count (1-4), settings.config.y = blend mode (see Layer blend)
// settings.config.z = texture repeat, settings.config.w = unused
// settings.layer_params_N.x = height_scale, settings.layer_params_N.y = height_offset
// settings.layer_params_N.z = MCMT terrain material id, settings.layer_params_N.w = overbright multiplier
// settings.animation_params_N.xy = per-layer UV velocity, settings.animation_params_N.z = reflection multiplier
@group(#{MATERIAL_BIND_GROUP}) @binding(0) var<uniform> settings: TerrainSettings;

@group(#{MATERIAL_BIND_GROUP}) @binding(1) var ground_0: texture_2d<f32>;
@group(#{MATERIAL_BIND_GROUP}) @binding(2) var ground_sampler_0: sampler;

@group(#{MATERIAL_BIND_GROUP}) @binding(3) var ground_1: texture_2d<f32>;
@group(#{MATERIAL_BIND_GROUP}) @binding(4) var ground_sampler_1: sampler;

@group(#{MATERIAL_BIND_GROUP}) @binding(5) var ground_2: texture_2d<f32>;
@group(#{MATERIAL_BIND_GROUP}) @binding(6) var ground_sampler_2: sampler;

@group(#{MATERIAL_BIND_GROUP}) @binding(7) var ground_3: texture_2d<f32>;
@group(#{MATERIAL_BIND_GROUP}) @binding(8) var ground_sampler_3: sampler;

@group(#{MATERIAL_BIND_GROUP}) @binding(9) var height_0: texture_2d<f32>;
@group(#{MATERIAL_BIND_GROUP}) @binding(10) var height_sampler_0: sampler;

@group(#{MATERIAL_BIND_GROUP}) @binding(11) var height_1: texture_2d<f32>;
@group(#{MATERIAL_BIND_GROUP}) @binding(12) var height_sampler_1: sampler;

@group(#{MATERIAL_BIND_GROUP}) @binding(13) var height_2: texture_2d<f32>;
@group(#{MATERIAL_BIND_GROUP}) @binding(14) var height_sampler_2: sampler;

@group(#{MATERIAL_BIND_GROUP}) @binding(15) var height_3: texture_2d<f32>;
@group(#{MATERIAL_BIND_GROUP}) @binding(16) var height_sampler_3: sampler;

@group(#{MATERIAL_BIND_GROUP}) @binding(17) var alpha_packed: texture_2d<f32>;
@group(#{MATERIAL_BIND_GROUP}) @binding(18) var alpha_sampler: sampler;

@group(#{MATERIAL_BIND_GROUP}) @binding(19) var shadow_map: texture_2d<f32>;
@group(#{MATERIAL_BIND_GROUP}) @binding(20) var shadow_sampler: sampler;
@group(#{MATERIAL_BIND_GROUP}) @binding(21) var environment_map: texture_cube<f32>;
@group(#{MATERIAL_BIND_GROUP}) @binding(22) var environment_sampler: sampler;
@group(#{MATERIAL_BIND_GROUP}) @binding(23) var<storage, read> scene_light: RetailSceneLight;

const TERRAIN_REFLECTION_FRESNEL_POWER: f32 = 4.0;
// WebWowViewerCpp config adtSpecMult.
const ADT_SPEC_MULT: f32 = 1.0;

// ── Hash: deterministic pseudo-random from grid cell ─────────────────────────

fn hash2(p: vec2<f32>) -> vec2<f32> {
    let k = vec2<f32>(
        dot(p, vec2<f32>(127.1, 311.7)),
        dot(p, vec2<f32>(269.5, 183.3)),
    );
    return fract(sin(k) * 43758.5453);
}

// ── 2D rotation ──────────────────────────────────────────────────────────────

fn rot2(v: vec2<f32>, a: f32) -> vec2<f32> {
    let c = cos(a);
    let s = sin(a);
    return vec2<f32>(v.x * c - v.y * s, v.x * s + v.y * c);
}

// ── Direct ground texture sampling ──────────────────────────────────────────

fn sample_ground(idx: u32, uv: vec2<f32>) -> vec4<f32> {
    switch idx {
        case 0u: { return textureSample(ground_0, ground_sampler_0, uv); }
        case 1u: { return textureSample(ground_1, ground_sampler_1, uv); }
        case 2u: { return textureSample(ground_2, ground_sampler_2, uv); }
        case 3u: { return textureSample(ground_3, ground_sampler_3, uv); }
        default: { return vec4<f32>(0.5, 0.5, 0.5, 1.0); }
    }
}

fn sample_ground_tiled(idx: u32, uv: vec2<f32>) -> vec4<f32> {
    return sample_ground(idx, uv * settings.config.z);
}

fn sample_height(idx: u32, uv: vec2<f32>) -> vec4<f32> {
    switch idx {
        case 0u: { return textureSample(height_0, height_sampler_0, uv); }
        case 1u: { return textureSample(height_1, height_sampler_1, uv); }
        case 2u: { return textureSample(height_2, height_sampler_2, uv); }
        case 3u: { return textureSample(height_3, height_sampler_3, uv); }
        default: { return vec4<f32>(0.5, 0.5, 0.5, 1.0); }
    }
}

fn sample_height_tiled(idx: u32, uv: vec2<f32>) -> vec4<f32> {
    return sample_height(idx, uv * settings.config.z);
}

fn layer_animation_params(idx: u32) -> vec4<f32> {
    switch idx {
        case 0u: { return settings.animation_params_0; }
        case 1u: { return settings.animation_params_1; }
        case 2u: { return settings.animation_params_2; }
        case 3u: { return settings.animation_params_3; }
        default: { return vec4<f32>(0.0); }
    }
}

fn layer_params(idx: u32) -> vec4<f32> {
    switch idx {
        case 0u: { return settings.layer_params_0; }
        case 1u: { return settings.layer_params_1; }
        case 2u: { return settings.layer_params_2; }
        case 3u: { return settings.layer_params_3; }
        default: { return vec4<f32>(1.0, 0.0, 0.0, 0.0); }
    }
}

fn animated_layer_uv(idx: u32, uv: vec2<f32>) -> vec2<f32> {
    return uv + layer_animation_params(idx).xy * globals.time;
}

fn apply_layer_overbright(idx: u32, color: vec4<f32>) -> vec4<f32> {
    let multiplier = layer_params(idx).w;
    return vec4<f32>(color.rgb * multiplier, color.a);
}

fn sample_environment_reflection(normal: vec3<f32>, view_dir: vec3<f32>) -> vec3<f32> {
    let reflection_dir = reflect(-normalize(view_dir), normalize(normal));
    return textureSample(environment_map, environment_sampler, reflection_dir).rgb;
}

fn reflection_mask() -> vec4<f32> {
    return vec4<f32>(
        settings.animation_params_0.z,
        settings.animation_params_1.z,
        settings.animation_params_2.z,
        settings.animation_params_3.z,
    );
}

// ── Hex tiling ───────────────────────────────────────────────────────────────
// Simplex/hex grid: divide tiled UV space into equilateral triangles.
// Each triangle has 3 vertices; for each vertex compute a random rotation
// and UV offset, sample the texture, blend with smoothed barycentric weights.

fn hex_sample(idx: u32, uv: vec2<f32>) -> vec4<f32> {
    // Scale UV to tiled space
    let p = uv * settings.config.z;

    // Transform to simplex (equilateral triangle) grid
    // Skew factor for 2D simplex: (sqrt(3)-1)/2
    let F2 = 0.36602540;  // (sqrt(3)-1)/2
    let G2 = 0.21132487;  // (3-sqrt(3))/6

    let s = (p.x + p.y) * F2;
    let i = floor(p.x + s);
    let j = floor(p.y + s);

    let t = (i + j) * G2;
    // Unskew back to get cell origin in UV space
    let x0 = p.x - (i - t);
    let y0 = p.y - (j - t);

    // Which simplex triangle? (upper-right vs lower-left)
    var i1: f32;
    var j1: f32;
    if x0 > y0 {
        i1 = 1.0; j1 = 0.0;
    } else {
        i1 = 0.0; j1 = 1.0;
    }

    // Offsets for the 3 simplex vertices relative to fragment
    let x1 = x0 - i1 + G2;
    let y1 = y0 - j1 + G2;
    let x2 = x0 - 1.0 + 2.0 * G2;
    let y2 = y0 - 1.0 + 2.0 * G2;

    // Barycentric-like distance weights (radial falloff from each vertex)
    var w0 = max(0.0, 0.5 - x0 * x0 - y0 * y0);
    var w1 = max(0.0, 0.5 - x1 * x1 - y1 * y1);
    var w2 = max(0.0, 0.5 - x2 * x2 - y2 * y2);

    // Smooth falloff (^3 for C2 continuity)
    w0 = w0 * w0 * w0;
    w1 = w1 * w1 * w1;
    w2 = w2 * w2 * w2;

    // Normalize weights
    let wsum = w0 + w1 + w2;
    if wsum < 0.0001 {
        return sample_ground(idx, p);
    }
    w0 = w0 / wsum;
    w1 = w1 / wsum;
    w2 = w2 / wsum;

    // Per-vertex random rotation and offset
    let h0 = hash2(vec2<f32>(i, j));
    let h1 = hash2(vec2<f32>(i + i1, j + j1));
    let h2 = hash2(vec2<f32>(i + 1.0, j + 1.0));

    let a0 = h0.x * 6.2831853;
    let a1 = h1.x * 6.2831853;
    let a2 = h2.x * 6.2831853;

    // Sample at rotated + offset UVs (texture wraps via Repeat sampler)
    let s0 = sample_ground(idx, rot2(p, a0) + h0 * 100.0);
    let s1 = sample_ground(idx, rot2(p, a1) + h1 * 100.0);
    let s2 = sample_ground(idx, rot2(p, a2) + h2 * 100.0);

    // Weighted blend
    var color = s0 * w0 + s1 * w1 + s2 * w2;

    // Variance-preserving contrast correction (in linear space)
    // gain = 1/sqrt(sum of squared weights), compensates averaging
    let g = 1.0 / sqrt(w0 * w0 + w1 * w1 + w2 * w2 + 0.0001);
    // Compute per-pixel mean and re-expand around it
    let mean = (s0.rgb + s1.rgb + s2.rgb) / 3.0;
    color = vec4<f32>((color.rgb - mean) * g + mean, 1.0);

    return color;
}

// ── Layer blend ──────────────────────────────────────────────────────────────
// settings.config.y selects the blend from the map's WDT MPHD flags:
// 0 = layered (4-bit alpha): each layer mixes over the result below it.
// 1 = weighted (big alpha): base = 1 - saturate(a1 + a2 + a3), layer N = aN.
// 2 = height-weighted (MPHD 0x80, wowdev ADT/v18 MTXP shader): weighted, times
//     (_h alpha * heightScale + heightOffset), keep layers within 1 of the highest, normalize.
const BLEND_LAYERED: u32 = 0u;
const BLEND_HEIGHT_WEIGHTED: u32 = 2u;

fn layered_weights(alpha: vec3<f32>, layer_count: u32) -> vec4<f32> {
    var w0 = 1.0;
    var w1 = 0.0;
    var w2 = 0.0;
    var w3 = 0.0;

    if layer_count > 1u {
        w0 = 1.0 - alpha.r;
        w1 = alpha.r;
    }
    if layer_count > 2u {
        let keep = 1.0 - alpha.g;
        w0 = w0 * keep;
        w1 = w1 * keep;
        w2 = alpha.g;
    }
    if layer_count > 3u {
        let keep = 1.0 - alpha.b;
        w0 = w0 * keep;
        w1 = w1 * keep;
        w2 = w2 * keep;
        w3 = alpha.b;
    }

    return vec4<f32>(w0, w1, w2, w3);
}

// Channels of layers a chunk does not have are packed as zero.
fn weighted_weights(alpha: vec3<f32>) -> vec4<f32> {
    return vec4<f32>(1.0 - clamp(alpha.r + alpha.g + alpha.b, 0.0, 1.0), alpha);
}

fn height_weighted(weights: vec4<f32>, heights: vec4<f32>) -> vec4<f32> {
    let pct = weights * heights;
    let pct_max = max(max(pct.x, pct.y), max(pct.z, pct.w));
    let kept = pct * (vec4<f32>(1.0) - clamp(vec4<f32>(pct_max) - pct, vec4<f32>(0.0), vec4<f32>(1.0)));
    let sum = kept.x + kept.y + kept.z + kept.w;
    if sum > 1e-6 {
        return kept / sum;
    }
    return weights;
}

fn layer_height(idx: u32, uv: vec2<f32>) -> f32 {
    let params = layer_params(idx);
    return sample_height_tiled(idx, uv).a * params.x + params.y;
}

// ── Fragment entry ───────────────────────────────────────────────────────────

@fragment
fn fragment(in: VertexOutput, @builtin(front_facing) is_front: bool) -> @location(0) vec4<f32> {
    let uv = in.uv;
    let layer_count = u32(settings.config.x);
    let blend_mode = u32(settings.config.y);

    let alpha = textureSample(alpha_packed, alpha_sampler, uv).rgb;

    let uv0 = animated_layer_uv(0u, uv);
    let uv1 = animated_layer_uv(1u, uv);
    let uv2 = animated_layer_uv(2u, uv);
    let uv3 = animated_layer_uv(3u, uv);
    let c0 = apply_layer_overbright(0u, gamma_layer(sample_ground_tiled(0u, uv0)));
    let c1 = apply_layer_overbright(1u, gamma_layer(sample_ground_tiled(1u, uv1)));
    let c2 = apply_layer_overbright(2u, gamma_layer(sample_ground_tiled(2u, uv2)));
    let c3 = apply_layer_overbright(3u, gamma_layer(sample_ground_tiled(3u, uv3)));

    var weights = weighted_weights(alpha);
    if blend_mode == BLEND_LAYERED {
        weights = layered_weights(alpha, layer_count);
    } else if blend_mode == BLEND_HEIGHT_WEIGHTED {
        let heights = vec4<f32>(
            layer_height(0u, uv0),
            layer_height(1u, uv1),
            layer_height(2u, uv2),
            layer_height(3u, uv3),
        );
        weights = height_weighted(weights, heights);
    }

    let blended = c0 * weights.x + c1 * weights.y + c2 * weights.z + c3 * weights.w;
    // MCCV is decoded as byte / 127, which is the client's `vColor * 2`.
    var diffuse = blended.rgb * in.color.rgb;
    let spec_blend = blended.a;

    let normal = pbr_functions::prepare_world_normal(in.world_normal, true, is_front);
    let n = normalize(normal);
    let view_dir = pbr_functions::calculate_view(in.world_position, view.clip_from_view[3].w == 1.0);
    let reflection = linear_to_gamma(sample_environment_reflection(n, view_dir));
    let reflective_weight = dot(weights, reflection_mask());
    let fresnel = pow(1.0 - max(dot(n, view_dir), 0.0), TERRAIN_REFLECTION_FRESNEL_POWER);
    diffuse = mix(diffuse, reflection, clamp(reflective_weight * fresnel, 0.0, 1.0));

    let sun = retail_sun_visibility(
        in.world_position,
        n,
        in.position.xy,
        mesh[in.instance_index].flags,
    );
    var color = retail_shade(scene_light, diffuse, n, sun);
    // adtShader_text.slang specular: the layer alpha masks a direct-light highlight.
    let half_vector = -normalize(scene_light.sun_direction.xyz - view_dir);
    let highlight = pow(max(0.0, dot(half_vector, n)), 20.0);
    color += spec_blend * scene_light.direct.rgb * highlight * ADT_SPEC_MULT * sun;
    color = retail_apply_fog(color, in.world_position.xyz, 0u);

    var pbr_input = pbr_types::pbr_input_new();
    pbr_input.frag_coord = in.position;
    pbr_input.world_position = in.world_position;
    // Fog is applied above in authored space; only post-processing runs here.
    pbr_input.material.flags = 0u;
    return pbr_functions::main_pass_post_lighting_processing(
        pbr_input,
        vec4<f32>(gamma_to_linear(color), 1.0),
    );
}

fn gamma_layer(color: vec4<f32>) -> vec4<f32> {
    return vec4<f32>(linear_to_gamma(color.rgb), color.a);
}
