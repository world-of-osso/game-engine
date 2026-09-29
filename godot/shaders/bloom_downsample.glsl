#[compute]
#version 450
// Uniform-scale additive bloom port of Bevy 0.19 bloom/bloom.wgsl.
// Bevy contributors, MIT OR Apache-2.0; this port uses MIT.
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

layout(local_size_x = 8, local_size_y = 8, local_size_z = 1) in;
layout(set = 0, binding = 0) uniform sampler2D source_color;
layout(r11f_g11f_b10f, set = 0, binding = 1) uniform writeonly image2D target_color;
layout(push_constant, std430) uniform Parameters { vec4 values; } parameters;

vec3 karis(vec3 color) {
    return color / (1.0 + dot(color, vec3(0.2126, 0.7152, 0.0722)) / 4.0);
}

vec3 soft_threshold(vec3 color) {
    const float threshold = 0.65;
    const float knee = threshold * 0.1;
    float brightness = max(color.r, max(color.g, color.b));
    float soft = clamp(brightness - (threshold - knee), 0.0, 2.0 * knee);
    soft = soft * soft * (0.25 / (knee + 0.00001));
    return color * (max(brightness - threshold, soft) / max(brightness, 0.00001));
}

vec3 filter_13(vec2 uv) {
    vec3 a = textureLodOffset(source_color, uv, 0.0, ivec2(-2, 2)).rgb;
    vec3 b = textureLodOffset(source_color, uv, 0.0, ivec2(0, 2)).rgb;
    vec3 c = textureLodOffset(source_color, uv, 0.0, ivec2(2, 2)).rgb;
    vec3 d = textureLodOffset(source_color, uv, 0.0, ivec2(-2, 0)).rgb;
    vec3 e = textureLod(source_color, uv, 0.0).rgb;
    vec3 f = textureLodOffset(source_color, uv, 0.0, ivec2(2, 0)).rgb;
    vec3 g = textureLodOffset(source_color, uv, 0.0, ivec2(-2, -2)).rgb;
    vec3 h = textureLodOffset(source_color, uv, 0.0, ivec2(0, -2)).rgb;
    vec3 i = textureLodOffset(source_color, uv, 0.0, ivec2(2, -2)).rgb;
    vec3 j = textureLodOffset(source_color, uv, 0.0, ivec2(-1, 1)).rgb;
    vec3 k = textureLodOffset(source_color, uv, 0.0, ivec2(1, 1)).rgb;
    vec3 l = textureLodOffset(source_color, uv, 0.0, ivec2(-1, -1)).rgb;
    vec3 m = textureLodOffset(source_color, uv, 0.0, ivec2(1, -1)).rgb;
    if (parameters.values.x > 0.5) {
        vec3 group0 = (a + b + d + e) * (0.125 / 4.0);
        vec3 group1 = (b + c + e + f) * (0.125 / 4.0);
        vec3 group2 = (d + e + g + h) * (0.125 / 4.0);
        vec3 group3 = (e + f + h + i) * (0.125 / 4.0);
        vec3 group4 = (j + k + l + m) * (0.5 / 4.0);
        vec3 color = karis(group0) + karis(group1) + karis(group2) + karis(group3) + karis(group4);
        return soft_threshold(clamp(color, vec3(0.0001), vec3(3.40282347e37)));
    }
    vec3 color = (a + c + g + i) * 0.03125;
    color += (b + d + f + h) * 0.0625;
    color += (e + j + k + l + m) * 0.125;
    return color;
}

void main() {
    ivec2 pixel = ivec2(gl_GlobalInvocationID.xy);
    ivec2 extent = imageSize(target_color);
    if (any(greaterThanEqual(pixel, extent))) return;
    vec2 uv = (vec2(pixel) + 0.5) / vec2(extent);
    imageStore(target_color, pixel, vec4(filter_13(uv), 1.0));
}
