shader_type spatial;
render_mode fog_disabled, specular_disabled, cull_back, blend_mix;

uniform sampler2D ribbon_texture : source_color, filter_linear, repeat_enable;
uniform int gx_blend = 2;
uniform vec4 uv_matrix = vec4(1.0, 0.0, 0.0, 1.0);
uniform vec2 uv_offset = vec2(0.0);

void fragment() {
    mat2 linear = mat2(uv_matrix.xy, uv_matrix.zw);
    vec2 transformed_uv = linear * UV + uv_offset;
    vec4 texel = texture(ribbon_texture, transformed_uv) * COLOR;
    if (gx_blend == 1 && texel.a < 0.5) { discard; }
    if (gx_blend == 0 || gx_blend == 1 || gx_blend == 3 || gx_blend == 4 || gx_blend == 5) {
        texel.a = 1.0;
    }
    if (gx_blend == 5) { texel.rgb *= 2.0; }
    if (gx_blend == 13) { texel.rgb *= texel.a; }
    ALBEDO = texel.rgb;
    ALPHA = texel.a;
}
