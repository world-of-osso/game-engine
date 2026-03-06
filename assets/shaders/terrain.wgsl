// Terrain shader with hex tiling (Heitz & Neyret 2018)
// Breaks visible texture repetition by sampling each ground layer 3x
// at hex-grid-offset UVs with per-cell random rotation + offset.
// Height-based blending uses ground texture alpha as height channel
// to make transitions between layers look more natural.

#import bevy_pbr::forward_io::VertexOutput

// config.x = layer_count (1-4), config.y = height_blend_strength (0=off, typical 2-4)
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
// Uses the ground texture alpha channel as a height map to make layer
// transitions less blobby. Rocks/roots poke through based on height.
// Formula: effective_alpha = saturate(alpha + (height - 0.5) * strength)

fn sample_height(idx: u32, uv: vec2<f32>) -> f32 {
    // Read the alpha channel of the ground texture as height (0..1).
    // Textures without height data have alpha=1.0 everywhere (from fix_1bit_alpha),
    // so (1.0 - 0.5) * strength just shifts alpha up slightly — safe fallback.
    let tiled_uv = uv * TILE_REPEAT;
    return sample_ground(idx, tiled_uv).a;
}

fn height_blend_alpha(base_alpha: f32, height: f32, strength: f32) -> f32 {
    return saturate(base_alpha + (height - 0.5) * strength);
}

// ── Fragment entry ───────────────────────────────────────────────────────────

@fragment
fn fragment(in: VertexOutput) -> @location(0) vec4<f32> {
    let uv = in.uv;
    let layer_count = u32(config.x);
    let blend_strength = config.y;

    // Base layer (full opacity, always present)
    var color = hex_sample(0u, uv);

    // Blend upper layers using packed alpha map (R=layer1, G=layer2, B=layer3)
    // When blend_strength > 0, height from texture alpha modulates transitions.
    if layer_count > 1u {
        let alpha = textureSample(alpha_packed, alpha_sampler, uv);

        let c1 = hex_sample(1u, uv);
        let h1 = sample_height(1u, uv);
        let a1 = height_blend_alpha(alpha.r, h1, blend_strength);
        color = mix(color, c1, a1);

        if layer_count > 2u {
            let c2 = hex_sample(2u, uv);
            let h2 = sample_height(2u, uv);
            let a2 = height_blend_alpha(alpha.g, h2, blend_strength);
            color = mix(color, c2, a2);
        }
        if layer_count > 3u {
            let c3 = hex_sample(3u, uv);
            let h3 = sample_height(3u, uv);
            let a3 = height_blend_alpha(alpha.b, h3, blend_strength);
            color = mix(color, c3, a3);
        }
    }

    // Basic Lambert lighting
    let n = normalize(in.world_normal);
    let sun = normalize(vec3<f32>(0.4, 0.8, 0.3));
    let ndl = max(dot(n, sun), 0.0);
    let ambient = 0.35;
    let lit = color.rgb * (ambient + (1.0 - ambient) * ndl);

    return vec4<f32>(lit, 1.0);
}
