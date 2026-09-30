#[compute]
#version 450
// Literal resolve math from Bevy 0.19.0 src/taa/taa.wgsl.
// Bevy contributors, MIT OR Apache-2.0; this port uses MIT.
// YCoCg and AABB clipping originate from Playdead's MIT temporal shader.
// Permission is hereby granted, free of charge, to any person obtaining a copy
// of this software and associated documentation files (the "Software"), to deal
// in the Software without restriction, including without limitation the rights
// to use, copy, modify, merge, publish, distribute, sublicense, and/or sell
// copies of the Software, and to permit persons to whom the Software is
// furnished to do so, subject to the following conditions:
// The above copyright notice and this permission notice shall be included in all
// copies or substantial portions of the Software.
// THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND, EXPRESS OR
// IMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF MERCHANTABILITY,
// FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT SHALL THE
// AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY CLAIM, DAMAGES OR OTHER
// LIABILITY, WHETHER IN AN ACTION OF CONTRACT, TORT OR OTHERWISE, ARISING FROM,
// OUT OF OR IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER DEALINGS IN THE
// SOFTWARE.

// Caller: clamp-to-edge nearest on current/history_nearest/depth/motion;
// clamp-to-edge linear on history_linear. History samplers share one input.
// Motion is already current-minus-previous UV. No convention conversion here.
// Both outputs have current's extent and must not alias any input or each other.
// RGBA16F outputs are provisional: NOT legacy history-format parity.
layout(local_size_x = 8, local_size_y = 8, local_size_z = 1) in;
layout(set = 0, binding = 0) uniform sampler2D current_input;
layout(set = 0, binding = 1) uniform sampler2D history_linear;
layout(set = 0, binding = 2) uniform sampler2D history_nearest;
layout(set = 0, binding = 3) uniform sampler2D depth_input;
layout(set = 0, binding = 4) uniform sampler2D motion_input;
layout(rgba16f, set = 0, binding = 5) uniform writeonly image2D resolved_output;
layout(rgba16f, set = 0, binding = 6) uniform writeonly image2D history_output;
layout(push_constant, std430) uniform Parameters {
    uint reset;
    uint tonemap;
} parameters;

const float DEFAULT_HISTORY_BLEND_RATE = 0.1;
const float MIN_HISTORY_BLEND_RATE = 0.015;

float max3(vec3 x) { return max(x.r, max(x.g, x.b)); }
vec3 tonemap(vec3 color) { return color * (1.0 / (max3(color) + 1.0)); }
vec3 reverse_tonemap(vec3 color) { return color * (1.0 / (1.0 - max3(color))); }

vec3 rgb_to_ycocg(vec3 rgb) {
    float y = (rgb.r / 4.0) + (rgb.g / 2.0) + (rgb.b / 4.0);
    float co = (rgb.r / 2.0) - (rgb.b / 2.0);
    float cg = (-rgb.r / 4.0) + (rgb.g / 2.0) - (rgb.b / 4.0);
    return vec3(y, co, cg);
}

vec3 ycocg_to_rgb(vec3 color) {
    float r = color.x + color.y - color.z;
    float g = color.x + color.z;
    float b = color.x - color.y - color.z;
    return clamp(vec3(r, g, b), vec3(0.0), vec3(1.0));
}

vec3 clip_towards_aabb_center(vec3 color, vec3 aabb_min, vec3 aabb_max) {
    vec3 center = 0.5 * (aabb_max + aabb_min);
    vec3 extent = 0.5 * (aabb_max - aabb_min) + 0.00000001;
    vec3 delta = color - center;
    float distance = max3(abs(delta / extent));
    if (distance > 1.0) {
        return center + delta / distance;
    }
    return color;
}

vec3 sample_current(vec2 uv) {
    vec3 color = textureLod(current_input, uv, 0.0).rgb;
    if (parameters.tonemap != 0u) { color = tonemap(color); }
    return rgb_to_ycocg(color);
}

vec2 closest_motion(vec2 uv, vec2 texel_size) {
    vec2 offset = texel_size * 2.0;
    vec2 tl = uv + vec2(-offset.x, offset.y);
    vec2 tr = uv + vec2(offset.x, offset.y);
    vec2 bl = uv + vec2(-offset.x, -offset.y);
    vec2 br = uv + vec2(offset.x, -offset.y);
    vec2 closest_uv = uv;
    float d_tl = textureLod(depth_input, tl, 0.0).r;
    float d_tr = textureLod(depth_input, tr, 0.0).r;
    float closest_depth = textureLod(depth_input, uv, 0.0).r;
    float d_bl = textureLod(depth_input, bl, 0.0).r;
    float d_br = textureLod(depth_input, br, 0.0).r;
    if (d_tl > closest_depth) { closest_uv = tl; closest_depth = d_tl; }
    if (d_tr > closest_depth) { closest_uv = tr; closest_depth = d_tr; }
    if (d_bl > closest_depth) { closest_uv = bl; closest_depth = d_bl; }
    if (d_br > closest_depth) { closest_uv = br; }
    return textureLod(motion_input, closest_uv, 0.0).rg;
}

vec3 sample_history(vec2 uv, vec2 texture_size) {
    // Deliberately use CURRENT extent even when history's allocation differs.
    vec2 texel_size = 1.0 / texture_size;
    vec2 position = uv * texture_size;
    vec2 center = floor(position - 0.5) + 0.5;
    vec2 f = position - center;
    vec2 w0 = f * (-0.5 + f * (1.0 - 0.5 * f));
    vec2 w1 = 1.0 + f * f * (-2.5 + 1.5 * f);
    vec2 w2 = f * (0.5 + f * (2.0 - 1.5 * f));
    vec2 w3 = f * f * (-0.5 + 0.5 * f);
    vec2 w12 = w1 + w2;
    vec2 p0 = (center - 1.0) * texel_size;
    vec2 p3 = (center + 2.0) * texel_size;
    vec2 p12 = (center + w2 / w12) * texel_size;
    vec3 color = textureLod(history_linear, vec2(p12.x, p0.y), 0.0).rgb * w12.x * w0.y;
    color += textureLod(history_linear, vec2(p0.x, p12.y), 0.0).rgb * w0.x * w12.y;
    color += textureLod(history_linear, p12, 0.0).rgb * w12.x * w12.y;
    color += textureLod(history_linear, vec2(p3.x, p12.y), 0.0).rgb * w3.x * w12.y;
    color += textureLod(history_linear, vec2(p12.x, p3.y), 0.0).rgb * w12.x * w3.y;
    return color; // Five taps, no renormalization.
}

vec3 clip_history(vec3 history_color, vec3 current_color, vec2 uv, vec2 texel_size) {
    vec3 tl = sample_current(uv + vec2(-texel_size.x, texel_size.y));
    vec3 tm = sample_current(uv + vec2(0.0, texel_size.y));
    vec3 tr = sample_current(uv + vec2(texel_size.x, texel_size.y));
    vec3 ml = sample_current(uv + vec2(-texel_size.x, 0.0));
    vec3 mm = rgb_to_ycocg(current_color);
    vec3 mr = sample_current(uv + vec2(texel_size.x, 0.0));
    vec3 bl = sample_current(uv + vec2(-texel_size.x, -texel_size.y));
    vec3 bm = sample_current(uv + vec2(0.0, -texel_size.y));
    vec3 br = sample_current(uv + vec2(texel_size.x, -texel_size.y));
    vec3 moment1 = tl + tm + tr + ml + mm + mr + bl + bm + br;
    vec3 moment2 = tl * tl + tm * tm + tr * tr + ml * ml + mm * mm + mr * mr + bl * bl + bm * bm + br * br;
    vec3 mean = moment1 / 9.0;
    vec3 variance = moment2 / 9.0 - mean * mean;
    vec3 deviation = sqrt(max(variance, vec3(0.0)));
    return ycocg_to_rgb(clip_towards_aabb_center(rgb_to_ycocg(history_color), mean - deviation, mean + deviation));
}

void main() {
    ivec2 pixel = ivec2(gl_GlobalInvocationID.xy);
    ivec2 extent = textureSize(current_input, 0);
    if (any(greaterThanEqual(pixel, extent))) { return; }
    vec2 texture_size = vec2(extent);
    vec2 uv = (vec2(pixel) + 0.5) / texture_size;
    vec4 original = textureLod(current_input, uv, 0.0);
    vec3 current_color = original.rgb;
    if (parameters.tonemap != 0u) { current_color = tonemap(current_color); }
    float confidence = 1.0 / MIN_HISTORY_BLEND_RATE;
    if (parameters.reset == 0u) {
        vec2 motion = closest_motion(uv, 1.0 / texture_size);
        vec2 history_uv = uv - motion;
        vec3 history_color = sample_history(history_uv, texture_size);
        history_color = clip_history(history_color, current_color, uv, 1.0 / texture_size);
        confidence = textureLod(history_nearest, uv, 0.0).a;
        vec2 pixel_motion = abs(motion) * texture_size;
        if (pixel_motion.x < 0.01 && pixel_motion.y < 0.01) {
            confidence += 10.0;
        } else {
            confidence = 1.0;
        }
        float current_factor = clamp(1.0 / confidence, MIN_HISTORY_BLEND_RATE, DEFAULT_HISTORY_BLEND_RATE);
        if (any(notEqual(clamp(history_uv, vec2(0.0), vec2(1.0)), history_uv))) {
            current_factor = 1.0;
            confidence = 1.0;
        }
        current_color = mix(history_color, current_color, current_factor);
    }
    imageStore(history_output, pixel, vec4(current_color, confidence));
    if (parameters.tonemap != 0u) { current_color = reverse_tonemap(current_color); }
    imageStore(resolved_output, pixel, vec4(current_color, original.a));
}
