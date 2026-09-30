#[fragment]
#version 450
// Uniform-scale raster tent port of Bevy 0.19 bloom/bloom.wgsl.
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

layout(location = 0) in vec2 output_uv;
layout(location = 0) out vec4 output_color;
layout(set = 0, binding = 0) uniform sampler2D source_color;

vec3 filter_tent(vec2 uv) {
    vec2 frag_size = vec2(1.0) / vec2(textureSize(source_color, 0));
    float x = frag_size.x;
    float y = frag_size.y;
    vec3 a = texture(source_color, vec2(uv.x - x, uv.y + y)).rgb;
    vec3 b = texture(source_color, vec2(uv.x, uv.y + y)).rgb;
    vec3 c = texture(source_color, vec2(uv.x + x, uv.y + y)).rgb;
    vec3 d = texture(source_color, vec2(uv.x - x, uv.y)).rgb;
    vec3 e = texture(source_color, vec2(uv.x, uv.y)).rgb;
    vec3 f = texture(source_color, vec2(uv.x + x, uv.y)).rgb;
    vec3 g = texture(source_color, vec2(uv.x - x, uv.y - y)).rgb;
    vec3 h = texture(source_color, vec2(uv.x, uv.y - y)).rgb;
    vec3 i = texture(source_color, vec2(uv.x + x, uv.y - y)).rgb;
    vec3 color = e * 0.25;
    color += (b + d + f + h) * 0.125;
    color += (a + c + g + i) * 0.0625;
    return color;
}

void main() {
    // Weighting and destination addition belong to the attachment blend stage:
    // ConstantColor/One RGB and Zero/One alpha, as upsampling_pipeline.rs.
    output_color = vec4(filter_tent(output_uv), 1.0);
}
