// Terrain shader with hex tiling (Heitz & Neyret 2018)
// Breaks visible texture repetition by sampling each ground layer 3x
// at hex-grid-offset UVs with per-cell random rotation + offset.
// Height-based blending uses ground texture alpha as height channel
// to make transitions between layers look more natural.

#import bevy_pbr::{
    forward_io::VertexOutput,
    mesh_view_bindings::view,
    pbr_functions,
    pbr_types,
}

// config.x = layer_count (1-4), config.y = height_blend_strength (0=off, typical 2-4)
// config.z = perceptual_roughness, config.w = reflectance
@group(#{MATERIAL_BIND_GROUP}) @binding(0) var<uniform> config: vec4<f32>;

@group(#{MATERIAL_BIND_GROUP}) @binding(1) var ground_0: texture_2d<f32>;
@group(#{MATERIAL_BIND_GROUP}) @binding(2) var ground_sampler_0: sampler;

@group(#{MATERIAL_BIND_GROUP}) @binding(3) var ground_1: texture_2d<f32>;
@group(#{MATERIAL_BIND_GROUP}) @binding(4) var ground_sampler_1: sampler;

@group(#{MATERIAL_BIND_GROUP}) @binding(5) var ground_2: texture_2d<f32>;
@group(#{MATERIAL_BIND_GROUP}) @binding(6) var ground_sampler_2: sampler;

@group(#{MATERIAL_BIND_GROUP}) @binding(7) var ground_3: texture_2d<f32>;
@group(#{MATERIAL_BIND_GROUP}) @binding(8) var ground_sampler_3: sampler;

@group(#{MATERIAL_BIND_GROUP}) @binding(9) var alpha_packed: texture_2d<f32>;
@group(#{MATERIAL_BIND_GROUP}) @binding(10) var alpha_sampler: sampler;

const TILE_REPEAT: f32 = 8.0;

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

// ── Hex tiling ───────────────────────────────────────────────────────────────
// Simplex/hex grid: divide tiled UV space into equilateral triangles.
// Each triangle has 3 vertices; for each vertex compute a random rotation
// and UV offset, sample the texture, blend with smoothed barycentric weights.

fn hex_sample(idx: u32, uv: vec2<f32>) -> vec4<f32> {
    // Scale UV to tiled space
    let p = uv * TILE_REPEAT;

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

// ── Height-based blend ──────────────────────────────────────────────────────
// WoW-style layer stacking still starts from alpha-painted order (base -> 1 -> 2 -> 3),
// but we re-weight each layer by its texture height channel and normalize.
// This keeps paint masks authoritative while letting rocky/high texels win locally.

fn paint_weights(alpha: vec3<f32>, layer_count: u32) -> vec4<f32> {
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

fn height_weight(height: f32, strength: f32) -> f32 {
    // strength=0 -> no height influence. Positive strength amplifies highs,
    // de-emphasizes lows, while remaining stable for textures with flat alpha.
    return exp2((height - 0.5) * max(strength, 0.0));
}

// ── Fragment entry ───────────────────────────────────────────────────────────

@fragment
fn fragment(in: VertexOutput, @builtin(front_facing) is_front: bool) -> @location(0) vec4<f32> {
    let uv = in.uv;
    let layer_count = u32(config.x);
    let blend_strength = config.y;
    let perceptual_roughness = config.z;
    let reflectance = config.w;

    let alpha = textureSample(alpha_packed, alpha_sampler, uv).rgb;
    let paint = paint_weights(alpha, layer_count);

    // Sample each potential layer once; alpha channel is used as height.
    let c0 = hex_sample(0u, uv);
    let c1 = hex_sample(1u, uv);
    let c2 = hex_sample(2u, uv);
    let c3 = hex_sample(3u, uv);

    var weights = vec4<f32>(
        paint.x * height_weight(c0.a, blend_strength),
        paint.y * height_weight(c1.a, blend_strength),
        paint.z * height_weight(c2.a, blend_strength),
        paint.w * height_weight(c3.a, blend_strength),
    );
    let wsum = weights.x + weights.y + weights.z + weights.w;
    if wsum > 1e-6 {
        weights = weights / wsum;
    } else {
        weights = paint;
    }

    let color = vec4<f32>(
        c0.rgb * weights.x + c1.rgb * weights.y + c2.rgb * weights.z + c3.rgb * weights.w,
        1.0,
    );

    var pbr_input = pbr_types::pbr_input_new();
    pbr_input.material.base_color = color;
    pbr_input.material.perceptual_roughness = perceptual_roughness;
    pbr_input.material.reflectance = vec3<f32>(reflectance);
    pbr_input.material.flags = pbr_types::STANDARD_MATERIAL_FLAGS_FOG_ENABLED_BIT;
    pbr_input.frag_coord = in.position;
    pbr_input.world_position = in.world_position;
    pbr_input.world_normal = pbr_functions::prepare_world_normal(in.world_normal, true, is_front);
    pbr_input.N = normalize(pbr_input.world_normal);
    pbr_input.is_orthographic = view.clip_from_view[3].w == 1.0;
    pbr_input.V = pbr_functions::calculate_view(
        in.world_position,
        pbr_input.is_orthographic,
    );

    let lit = pbr_functions::apply_pbr_lighting(pbr_input);
    return pbr_functions::main_pass_post_lighting_processing(pbr_input, lit);
}
