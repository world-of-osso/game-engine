#[fragment]
#version 450
// Raster stores retain the destination format's actual conversion and clamping.
layout(location = 0) in vec2 output_uv;
layout(location = 0) out vec4 copied_color;
layout(set = 0, binding = 0) uniform sampler2D source_color;

void main() {
    copied_color = textureLod(source_color, output_uv, 0.0);
}
