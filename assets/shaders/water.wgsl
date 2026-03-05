// Water shader: dual scrolling normal maps, fresnel, specular highlight.

#import bevy_pbr::{
    mesh_view_bindings::view,
    forward_io::VertexOutput,
}

struct WaterSettings {
    base_color: vec4<f32>,
    scroll_speed_1: vec2<f32>,
    scroll_speed_2: vec2<f32>,
    normal_scale: f32,
    fresnel_power: f32,
    specular_strength: f32,
    time: f32,
}

@group(#{MATERIAL_BIND_GROUP}) @binding(0) var<uniform> settings: WaterSettings;
@group(#{MATERIAL_BIND_GROUP}) @binding(1) var normal_map: texture_2d<f32>;
@group(#{MATERIAL_BIND_GROUP}) @binding(2) var normal_sampler: sampler;

@fragment
fn fragment(in: VertexOutput) -> @location(0) vec4<f32> {
    let time = settings.time;

    // Scroll UVs in two directions at different scales
    let uv1 = in.uv * 4.0 + settings.scroll_speed_1 * time;
    let uv2 = in.uv * 6.0 + settings.scroll_speed_2 * time;

    // Sample and decode normals from [0,1] to [-1,1]
    let n1 = textureSample(normal_map, normal_sampler, uv1).xyz * 2.0 - 1.0;
    let n2 = textureSample(normal_map, normal_sampler, uv2).xyz * 2.0 - 1.0;

    // UDN normal blending
    let blended = normalize(vec3<f32>(
        (n1.xy + n2.xy) * settings.normal_scale,
        n1.z * n2.z
    ));

    // View direction (camera to fragment)
    let V = normalize(view.world_position - in.world_position.xyz);
    // Perturb surface normal with blended ripple normals
    let N = normalize(in.world_normal + vec3<f32>(blended.x, 0.0, blended.y));

    // Fresnel: transparent head-on, opaque at glancing angles
    let NdotV = max(dot(N, V), 0.0);
    let fresnel = pow(1.0 - NdotV, settings.fresnel_power);

    // Sun specular (hardcoded directional light from above-right)
    let L = normalize(vec3<f32>(0.5, 0.8, 0.3));
    let H = normalize(V + L);
    let spec = pow(max(dot(N, H), 0.0), 64.0) * settings.specular_strength;

    // Mix water tint with sky reflection based on fresnel
    let sky_color = vec3<f32>(0.6, 0.75, 0.9);
    let water_color = settings.base_color.rgb;
    let color = mix(water_color, sky_color, fresnel) + vec3<f32>(spec);
    let alpha = mix(0.4, 0.85, fresnel);

    return vec4<f32>(color, alpha);
}
