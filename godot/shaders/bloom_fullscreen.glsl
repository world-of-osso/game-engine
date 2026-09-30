#[vertex]
#version 450
// Bevy 0.19 fullscreen.wgsl triangle, adapted to Vulkan's clip-Y convention.
// Bevy contributors, MIT OR Apache-2.0; MIT terms in bloom_upsample.glsl.
layout(location = 0) out vec2 output_uv;

void main() {
    vec2 uv = vec2(float(gl_VertexIndex >> 1), float(gl_VertexIndex & 1)) * 2.0;
    gl_Position = vec4(uv * 2.0 - 1.0, 0.0, 1.0);
    output_uv = uv;
}
