shader_type canvas_item;
// Repeat only the cropped bank member, not neighboring atlas art.
uniform vec4 region;
uniform vec2 repeats;
varying vec4 tint;
void vertex() {
    tint = COLOR;
}
void fragment() {
    vec2 uv = region.xy + fract(UV * repeats) * region.zw;
    vec2 half_pixel = TEXTURE_PIXEL_SIZE * 0.5;
    uv = clamp(uv, region.xy + half_pixel, region.xy + region.zw - half_pixel);
    COLOR = texture(TEXTURE, uv) * tint;
}
