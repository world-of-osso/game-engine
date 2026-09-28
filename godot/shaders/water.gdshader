shader_type spatial;
render_mode unshaded, cull_disabled, blend_mix, depth_draw_never, fog_disabled;

// Same executed material inputs and fragment math as assets/shaders/water.wgsl.
uniform sampler2D normal_map : repeat_enable, filter_linear;
uniform float animation_time = 0.0;
uniform vec4 base_color = vec4(0.05, 0.2, 0.4, 1.0);
uniform vec2 scroll_speed_1 = vec2(0.03, 0.02);
uniform vec2 scroll_speed_2 = vec2(-0.02, 0.04);
uniform float normal_scale = 0.15;
uniform float fresnel_power = 3.0;
uniform float specular_strength = 1.5;
uniform vec4 sky_color = vec4(0.6, 0.75, 0.9, 1.0);
varying vec3 world_position;
varying vec3 world_normal;

void vertex() {
    world_position = (MODEL_MATRIX * vec4(VERTEX, 1.0)).xyz;
    world_normal = normalize(MODEL_NORMAL_MATRIX * NORMAL);
}

void fragment() {
    vec2 uv1 = UV * 4.0 + scroll_speed_1 * animation_time;
    vec2 uv2 = UV * 6.0 + scroll_speed_2 * animation_time;
    vec3 n1 = texture(normal_map, uv1).xyz * 2.0 - 1.0;
    vec3 n2 = texture(normal_map, uv2).xyz * 2.0 - 1.0;
    vec3 blended = normalize(vec3((n1.xy + n2.xy) * normal_scale, n1.z * n2.z));
    vec3 view_direction = normalize(CAMERA_POSITION_WORLD - world_position);
    vec3 surface_normal = normalize(world_normal + vec3(blended.x, 0.0, blended.y));
    float fresnel = pow(1.0 - max(dot(surface_normal, view_direction), 0.0), fresnel_power);
    vec3 light_direction = normalize(vec3(0.5, 0.8, 0.3));
    vec3 halfway = normalize(view_direction + light_direction);
    float specular = pow(max(dot(surface_normal, halfway), 0.0), 64.0) * specular_strength;
    ALBEDO = mix(base_color.rgb, sky_color.rgb, fresnel) + vec3(specular);
    float depth_alpha = mix(0.08, 0.7, clamp(COLOR.a, 0.0, 1.0));
    ALPHA = mix(depth_alpha, 0.85, fresnel);
}
