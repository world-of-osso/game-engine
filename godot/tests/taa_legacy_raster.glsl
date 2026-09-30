#version 450
// Independent hand translation of bevy_anti_alias-0.19.0/src/taa/taa.wgsl.
// Test-only fragment MRT: original RESET / TONEMAP compile-time variants.
// Current extent intentionally drives Catmull-Rom coordinates even when history
// extent differs. No corner-weight renormalization, confidence clamp, or HDR fix.
layout(location = 0) in vec2 uv;
layout(location = 0) out vec4 resolved;
layout(location = 1) out vec4 next_history;
layout(set = 0, binding = 0) uniform sampler2D view_target;
layout(set = 0, binding = 1) uniform sampler2D history_linear;
layout(set = 0, binding = 2) uniform sampler2D history_nearest;
layout(set = 0, binding = 3) uniform sampler2D depth;
layout(set = 0, binding = 4) uniform sampler2D motion_vectors;

const float DEFAULT_HISTORY_BLEND_RATE = 0.1;
const float MIN_HISTORY_BLEND_RATE = 0.015;
float max3(vec3 x) { return max(x.r, max(x.g, x.b)); }
vec3 tonemap(vec3 color) { return color * (1.0 / (max3(color) + 1.0)); }
vec3 reverse_tonemap(vec3 color) { return color * (1.0 / (1.0 - max3(color))); }
vec3 RGB_to_YCoCg(vec3 rgb) {
    float y = rgb.r / 4.0 + rgb.g / 2.0 + rgb.b / 4.0;
    float co = rgb.r / 2.0 - rgb.b / 2.0;
    float cg = -rgb.r / 4.0 + rgb.g / 2.0 - rgb.b / 4.0;
    return vec3(y, co, cg);
}
vec3 YCoCg_to_RGB(vec3 ycocg) {
    float r = ycocg.x + ycocg.y - ycocg.z;
    float g = ycocg.x + ycocg.z;
    float b = ycocg.x - ycocg.y - ycocg.z;
    return clamp(vec3(r, g, b), 0.0, 1.0);
}
vec3 clip_towards_aabb_center(vec3 history_color, vec3 aabb_min, vec3 aabb_max) {
    vec3 p_clip = 0.5 * (aabb_max + aabb_min);
    vec3 e_clip = 0.5 * (aabb_max - aabb_min) + 0.00000001;
    vec3 v_clip = history_color - p_clip;
    float ma_unit = max3(abs(v_clip / e_clip));
    if (ma_unit > 1.0) { return p_clip + v_clip / ma_unit; }
    return history_color;
}
vec3 sample_history(float u, float v) {
    return texture(history_linear, vec2(u, v)).rgb;
}
vec3 sample_view_target(vec2 position) {
    vec3 value = texture(view_target, position).rgb;
#ifdef TONEMAP
    value = tonemap(value);
#endif
    return RGB_to_YCoCg(value);
}
void main() {
#ifdef SAMPLE_PROBE
    vec2 position = uv * 2.0 - 0.5;
    resolved = texture(view_target, position);
    next_history = vec4(texture(history_linear, position).rgb, texture(history_nearest, position).a);
    return;
#endif
#ifdef AUX_SAMPLE_PROBE
    vec2 position = uv * 2.0 - 0.5;
    resolved = vec4(texture(depth, position).r, texture(motion_vectors, position).rg, 1.0);
    next_history = resolved;
    return;
#endif
    vec2 texture_size = vec2(textureSize(view_target, 0));
    vec2 texel_size = 1.0 / texture_size;
    vec4 original_color = texture(view_target, uv);
    vec3 current_color = original_color.rgb;
#ifdef TONEMAP
    current_color = tonemap(current_color);
#endif
#ifndef RESET
    vec2 offset = texel_size * 2.0;
    vec2 d_uv_tl = uv + vec2(-offset.x, offset.y);
    vec2 d_uv_tr = uv + vec2(offset.x, offset.y);
    vec2 d_uv_bl = uv + vec2(-offset.x, -offset.y);
    vec2 d_uv_br = uv + vec2(offset.x, -offset.y);
    vec2 closest_uv = uv;
    float d_tl = texture(depth, d_uv_tl).r;
    float d_tr = texture(depth, d_uv_tr).r;
    float closest_depth = texture(depth, uv).r;
    float d_bl = texture(depth, d_uv_bl).r;
    float d_br = texture(depth, d_uv_br).r;
    if (d_tl > closest_depth) { closest_uv = d_uv_tl; closest_depth = d_tl; }
    if (d_tr > closest_depth) { closest_uv = d_uv_tr; closest_depth = d_tr; }
    if (d_bl > closest_depth) { closest_uv = d_uv_bl; closest_depth = d_bl; }
    if (d_br > closest_depth) { closest_uv = d_uv_br; }
    vec2 closest_motion_vector = texture(motion_vectors, closest_uv).rg;
    vec2 history_uv = uv - closest_motion_vector;
    vec2 sample_position = history_uv * texture_size;
    vec2 texel_center = floor(sample_position - 0.5) + 0.5;
    vec2 f = sample_position - texel_center;
    vec2 w0 = f * (-0.5 + f * (1.0 - 0.5 * f));
    vec2 w1 = 1.0 + f * f * (-2.5 + 1.5 * f);
    vec2 w2 = f * (0.5 + f * (2.0 - 1.5 * f));
    vec2 w3 = f * f * (-0.5 + 0.5 * f);
    vec2 w12 = w1 + w2;
    vec2 texel_position_0 = (texel_center - 1.0) * texel_size;
    vec2 texel_position_3 = (texel_center + 2.0) * texel_size;
    vec2 texel_position_12 = (texel_center + w2 / w12) * texel_size;
    vec3 history_color = sample_history(texel_position_12.x, texel_position_0.y) * w12.x * w0.y;
    history_color += sample_history(texel_position_0.x, texel_position_12.y) * w0.x * w12.y;
    history_color += sample_history(texel_position_12.x, texel_position_12.y) * w12.x * w12.y;
    history_color += sample_history(texel_position_3.x, texel_position_12.y) * w3.x * w12.y;
    history_color += sample_history(texel_position_12.x, texel_position_3.y) * w12.x * w3.y;

    vec3 s_tl = sample_view_target(uv + vec2(-texel_size.x, texel_size.y));
    vec3 s_tm = sample_view_target(uv + vec2(0.0, texel_size.y));
    vec3 s_tr = sample_view_target(uv + vec2(texel_size.x, texel_size.y));
    vec3 s_ml = sample_view_target(uv + vec2(-texel_size.x, 0.0));
    vec3 s_mm = RGB_to_YCoCg(current_color);
    vec3 s_mr = sample_view_target(uv + vec2(texel_size.x, 0.0));
    vec3 s_bl = sample_view_target(uv + vec2(-texel_size.x, -texel_size.y));
    vec3 s_bm = sample_view_target(uv + vec2(0.0, -texel_size.y));
    vec3 s_br = sample_view_target(uv + vec2(texel_size.x, -texel_size.y));
    vec3 moment_1 = s_tl + s_tm + s_tr + s_ml + s_mm + s_mr + s_bl + s_bm + s_br;
    vec3 moment_2 = s_tl*s_tl + s_tm*s_tm + s_tr*s_tr + s_ml*s_ml + s_mm*s_mm + s_mr*s_mr + s_bl*s_bl + s_bm*s_bm + s_br*s_br;
    vec3 mean = moment_1 / 9.0;
    vec3 variance = moment_2 / 9.0 - mean * mean;
    vec3 std_deviation = sqrt(max(variance, vec3(0.0)));
    history_color = RGB_to_YCoCg(history_color);
    history_color = clip_towards_aabb_center(history_color, mean - std_deviation, mean + std_deviation);
    history_color = YCoCg_to_RGB(history_color);

    float history_confidence = texture(history_nearest, uv).a;
    vec2 pixel_motion_vector = abs(closest_motion_vector) * texture_size;
    if (pixel_motion_vector.x < 0.01 && pixel_motion_vector.y < 0.01) {
        history_confidence += 10.0;
    } else {
        history_confidence = 1.0;
    }
    float current_color_factor = clamp(1.0 / history_confidence, MIN_HISTORY_BLEND_RATE, DEFAULT_HISTORY_BLEND_RATE);
    if (any(notEqual(clamp(history_uv, 0.0, 1.0), history_uv))) {
        current_color_factor = 1.0;
        history_confidence = 1.0;
    }
    current_color = mix(history_color, current_color, current_color_factor);
#else
    float history_confidence = 1.0 / MIN_HISTORY_BLEND_RATE;
#endif
    next_history = vec4(current_color, history_confidence);
#ifdef TONEMAP
    current_color = reverse_tonemap(current_color);
#endif
    resolved = vec4(current_color, original_color.a);
}
