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
#ifdef FINAL_COMPOSITE
layout(rgba16f, set = 0, binding = 1) uniform image2D target_color;
#else
layout(r11f_g11f_b10f, set = 0, binding = 1) uniform image2D target_color;
#endif
layout(push_constant, std430) uniform Parameters { vec4 values; } parameters;

vec3 filter_tent(vec2 uv) {
    vec2 texel = 1.0 / vec2(textureSize(source_color, 0));
    vec3 a = textureLod(source_color, uv + vec2(-texel.x, texel.y), 0.0).rgb;
    vec3 b = textureLod(source_color, uv + vec2(0.0, texel.y), 0.0).rgb;
    vec3 c = textureLod(source_color, uv + texel, 0.0).rgb;
    vec3 d = textureLod(source_color, uv + vec2(-texel.x, 0.0), 0.0).rgb;
    vec3 e = textureLod(source_color, uv, 0.0).rgb;
    vec3 f = textureLod(source_color, uv + vec2(texel.x, 0.0), 0.0).rgb;
    vec3 g = textureLod(source_color, uv - texel, 0.0).rgb;
    vec3 h = textureLod(source_color, uv + vec2(0.0, -texel.y), 0.0).rgb;
    vec3 i = textureLod(source_color, uv + vec2(texel.x, -texel.y), 0.0).rgb;
    vec3 color = e * 0.25;
    color += (b + d + f + h) * 0.125;
    color += (a + c + g + i) * 0.0625;
    return color;
}

void main() {
    ivec2 pixel = ivec2(gl_GlobalInvocationID.xy);
    ivec2 extent = imageSize(target_color);
    if (any(greaterThanEqual(pixel, extent))) return;
    vec2 uv = (vec2(pixel) + 0.5) / vec2(extent);
    // Source is always a DISTINCT lower-resolution texture. Only our own
    // target pixel is read, so this additive write has no neighbourhood race.
    vec4 original = imageLoad(target_color, pixel);
    imageStore(target_color, pixel, vec4(original.rgb + filter_tent(uv) * parameters.values.x, original.a));
}
