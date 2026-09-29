// Copyright (c) 2022 Advanced Micro Devices, Inc. All rights reserved.
//
// Permission is hereby granted, free of charge, to any person obtaining a copy
// of this software and associated documentation files (the "Software"), to deal
// in the Software without restriction, including without limitation the rights
// to use, copy, modify, merge, publish, distribute, sublicense, and/or sell
// copies of the Software, and to permit persons to whom the Software is
// furnished to do so, subject to the following conditions:
// The above copyright notice and this permission notice shall be included in
// all copies or substantial portions of the Software.
//
// THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND, EXPRESS OR
// IMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF MERCHANTABILITY,
// FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT SHALL THE
// AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY CLAIM, DAMAGES OR OTHER
// LIABILITY, WHETHER IN AN ACTION OF CONTRACT, TORT OR OTHERWISE, ARISING FROM,
// OUT OF OR IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER DEALINGS IN
// THE SOFTWARE.

shader_type canvas_item;
render_mode unshaded, blend_disabled;

uniform sampler2D screen_texture : hint_screen_texture, repeat_disable, filter_nearest;

vec3 decode_srgb(vec3 value) {
    return mix(value / 12.92, pow((value + 0.055) / 1.055, vec3(2.4)), step(vec3(0.04045), value));
}

vec3 encode_srgb(vec3 value) {
    return mix(value * 12.92, 1.055 * pow(max(value, vec3(0.0)), vec3(1.0 / 2.4)) - 0.055, step(vec3(0.0031308), value));
}

vec3 sample_rgb(vec2 uv) {
    return decode_srgb(texture(screen_texture, uv).rgb);
}

void fragment() {
    vec4 center = texture(screen_texture, SCREEN_UV);
    vec2 texel = SCREEN_PIXEL_SIZE;
    vec3 b = sample_rgb(SCREEN_UV - vec2(0.0, texel.y));
    vec3 d = sample_rgb(SCREEN_UV - vec2(texel.x, 0.0));
    vec3 f = sample_rgb(SCREEN_UV + vec2(texel.x, 0.0));
    vec3 h = sample_rgb(SCREEN_UV + vec2(0.0, texel.y));
    vec3 e = decode_srgb(center.rgb);
    vec3 mn4 = min(min(b, d), min(f, h));
    vec3 mx4 = max(max(b, d), max(f, h));
    // Black neighborhoods have no sharpening lobe; avoid 0/0 in the RCAS bound.
    vec3 hit_min = mn4 / (4.0 * max(mx4, vec3(0.000001)));
    vec3 hit_max = (vec3(10.0) - mx4) / (vec3(-40.0) + 4.0 * mn4);
    vec3 lobe_rgb = max(-hit_min, hit_max);
    float lobe = clamp(max(lobe_rgb.r, max(lobe_rgb.g, lobe_rgb.b)), -0.1875, 0.0) * 0.6;
    vec3 result = ((b + d + f + h) * lobe + e) / (4.0 * lobe + 1.0);
    COLOR = vec4(encode_srgb(result), center.a);
}
