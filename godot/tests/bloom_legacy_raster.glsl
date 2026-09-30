#version 450
// Independent test-only GLSL translation of bevy_post_process 0.19.0
// src/bloom/bloom.wgsl. Bevy contributors; MIT (bloom_legacy_LICENSE-MIT).
// Specializes UNIFORM_SCALE, full viewport and project threshold .65/.1.
// FIRST_DOWNSAMPLE selects Karis + USE_THRESHOLD; UPSAMPLE selects tent.
// Blend weighting is deliberately absent: upsampling_pipeline.rs applies it
// through Constant/One RGB and Zero/One alpha attachment blending.
layout(location = 0) in vec2 output_uv;
layout(location = 0) out vec4 output_color;
layout(set = 0, binding = 0) uniform sampler2D input_texture;
layout(push_constant, std430) uniform BloomUniforms {
    vec4 threshold_precomputations;
    vec4 viewport;
    vec2 scale;
    float aspect;
    float padding;
} uniforms;

vec3 soft_threshold(vec3 color) {
    float brightness = max(color.r, max(color.g, color.b));
    float softness = brightness - uniforms.threshold_precomputations.y;
    softness = clamp(softness, 0.0, uniforms.threshold_precomputations.z);
    softness = softness * softness * uniforms.threshold_precomputations.w;
    float contribution = max(brightness - uniforms.threshold_precomputations.x, softness);
    contribution /= max(brightness, 0.00001);
    return color * contribution;
}

float tonemapping_luminance(vec3 v) {
    return dot(v, vec3(0.2126, 0.7152, 0.0722));
}

float karis_average(vec3 color) {
    float luma = tonemapping_luminance(color) / 4.0;
    return 1.0 / (1.0 + luma);
}

vec3 sample_input_13_tap(vec2 uv) {
    vec3 a = textureOffset(input_texture, uv, ivec2(-2, 2)).rgb;
    vec3 b = textureOffset(input_texture, uv, ivec2(0, 2)).rgb;
    vec3 c = textureOffset(input_texture, uv, ivec2(2, 2)).rgb;
    vec3 d = textureOffset(input_texture, uv, ivec2(-2, 0)).rgb;
    vec3 e = texture(input_texture, uv).rgb;
    vec3 f = textureOffset(input_texture, uv, ivec2(2, 0)).rgb;
    vec3 g = textureOffset(input_texture, uv, ivec2(-2, -2)).rgb;
    vec3 h = textureOffset(input_texture, uv, ivec2(0, -2)).rgb;
    vec3 i = textureOffset(input_texture, uv, ivec2(2, -2)).rgb;
    vec3 j = textureOffset(input_texture, uv, ivec2(-1, 1)).rgb;
    vec3 k = textureOffset(input_texture, uv, ivec2(1, 1)).rgb;
    vec3 l = textureOffset(input_texture, uv, ivec2(-1, -1)).rgb;
    vec3 m = textureOffset(input_texture, uv, ivec2(1, -1)).rgb;
#ifdef FIRST_DOWNSAMPLE
    vec3 group0 = (a + b + d + e) * (0.125 / 4.0);
    vec3 group1 = (b + c + e + f) * (0.125 / 4.0);
    vec3 group2 = (d + e + g + h) * (0.125 / 4.0);
    vec3 group3 = (e + f + h + i) * (0.125 / 4.0);
    vec3 group4 = (j + k + l + m) * (0.5 / 4.0);
    group0 *= karis_average(group0);
    group1 *= karis_average(group1);
    group2 *= karis_average(group2);
    group3 *= karis_average(group3);
    group4 *= karis_average(group4);
    return group0 + group1 + group2 + group3 + group4;
#else
    vec3 sample_value = (a + c + g + i) * 0.03125;
    sample_value += (b + d + f + h) * 0.0625;
    sample_value += (e + j + k + l + m) * 0.125;
    return sample_value;
#endif
}

vec3 sample_input_3x3_tent(vec2 uv) {
    vec2 frag_size = uniforms.scale / vec2(textureSize(input_texture, 0));
    float x = frag_size.x;
    float y = frag_size.y;
    vec3 a = texture(input_texture, vec2(uv.x - x, uv.y + y)).rgb;
    vec3 b = texture(input_texture, vec2(uv.x, uv.y + y)).rgb;
    vec3 c = texture(input_texture, vec2(uv.x + x, uv.y + y)).rgb;
    vec3 d = texture(input_texture, vec2(uv.x - x, uv.y)).rgb;
    vec3 e = texture(input_texture, vec2(uv.x, uv.y)).rgb;
    vec3 f = texture(input_texture, vec2(uv.x + x, uv.y)).rgb;
    vec3 g = texture(input_texture, vec2(uv.x - x, uv.y - y)).rgb;
    vec3 h = texture(input_texture, vec2(uv.x, uv.y - y)).rgb;
    vec3 i = texture(input_texture, vec2(uv.x + x, uv.y - y)).rgb;
    vec3 sample_value = e * 0.25;
    sample_value += (b + d + f + h) * 0.125;
    sample_value += (a + c + g + i) * 0.0625;
    return sample_value;
}

void main() {
#ifdef UPSAMPLE
    output_color = vec4(sample_input_3x3_tent(output_uv), 1.0);
#elif defined(FIRST_DOWNSAMPLE)
    vec2 sample_uv = uniforms.viewport.xy + output_uv * uniforms.viewport.zw;
    vec3 sample_value = sample_input_13_tap(sample_uv);
    sample_value = clamp(sample_value, vec3(0.0001), vec3(3.40282347E+37));
    sample_value = soft_threshold(sample_value);
    output_color = vec4(sample_value, 1.0);
#else
    output_color = vec4(sample_input_13_tap(output_uv), 1.0);
#endif
}
