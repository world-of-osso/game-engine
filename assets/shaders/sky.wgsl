#import bevy_pbr::{
    mesh_view_bindings::view,
    forward_io::VertexOutput,
}

const PI: f32 = 3.14159265;

struct SkyUniforms {
    sky_top:    vec4<f32>,
    sky_middle: vec4<f32>,
    sky_band1:  vec4<f32>,
    sky_band2:  vec4<f32>,
    sky_smog:   vec4<f32>,
    sky_fog:    vec4<f32>,
    sun_color: vec4<f32>,
    sun_halo_color: vec4<f32>,
    cloud_emissive_color: vec4<f32>,
    cloud_layer1_ambient_color: vec4<f32>,
    cloud_layer2_ambient_color: vec4<f32>,
    sun_direction: vec4<f32>,
    cloud_params: vec4<f32>,
}

@group(#{MATERIAL_BIND_GROUP}) @binding(0) var<uniform> sky: SkyUniforms;
@group(#{MATERIAL_BIND_GROUP}) @binding(1) var cloud_texture: texture_2d<f32>;
@group(#{MATERIAL_BIND_GROUP}) @binding(2) var cloud_sampler: sampler;

// Dome colour stops, top pole first (see src/rendering/skybox/sky_gradient.rs).
// `band` is the mesh's per-ring band coordinate; the GPU interpolates it along
// the dome edges exactly as the client interpolated its per-vertex colours.
fn sample_gradient(band: f32) -> vec3<f32> {
    var stops = array<vec3<f32>, 7>(
        sky.sky_top.rgb,
        sky.sky_middle.rgb,
        sky.sky_band1.rgb,
        sky.sky_band2.rgb,
        sky.sky_smog.rgb,
        sky.sky_fog.rgb,
        sky.sky_fog.rgb,
    );
    let b = clamp(band, 0.0, 6.0);
    let i = min(u32(floor(b)), 5u);
    return mix(stops[i], stops[i + 1u], b - f32(i));
}

struct SkyFragmentOutput {
    @location(0) color: vec4<f32>,
    @builtin(frag_depth) depth: f32,
};

@fragment
fn fragment(in: VertexOutput) -> SkyFragmentOutput {
    let dir = normalize(in.world_position.xyz - view.world_position);
    var color = sample_gradient(in.uv.y * 6.0);

    let elev = clamp(asin(dir.y) / (PI / 2.0), 0.0, 1.0);

    let uv = vec2(
        atan2(dir.z, dir.x) / (2.0 * PI) + 0.5 + sky.cloud_params.y,
        0.5 - asin(dir.y) / PI + sky.cloud_params.z,
    );
    let cloud_a = textureSample(cloud_texture, cloud_sampler, uv).r;
    let cloud_b = textureSample(
        cloud_texture,
        cloud_sampler,
        uv * vec2(2.0, 1.35) + vec2(0.17, 0.29),
    )
    .r;
    let cloud_shape = mix(cloud_a, cloud_b, 0.35);
    let density = clamp(sky.cloud_params.x, 0.0, 1.0);
    let threshold = mix(0.92, 0.32, density);
    let horizon_mask = smoothstep(0.02, 0.18, elev) * (1.0 - smoothstep(0.82, 0.98, elev));
    let cloud_mask = smoothstep(threshold - 0.1, threshold + 0.1, cloud_shape) * horizon_mask;

    let sun_dir = normalize(sky.sun_direction.xyz);
    let sun_alignment = max(dot(dir, sun_dir), 0.0);
    let halo = pow(sun_alignment, 18.0);
    let highlight = pow(sun_alignment, 36.0);
    let cloud_ambient = mix(
        sky.cloud_layer2_ambient_color.rgb,
        sky.cloud_layer1_ambient_color.rgb,
        clamp(cloud_shape * 1.2, 0.0, 1.0),
    );
    let cloud_lighting = sky.sun_halo_color.rgb * halo + sky.sun_color.rgb * highlight;
    let cloud_color = cloud_ambient + sky.cloud_emissive_color.rgb + cloud_lighting;
    color = mix(color, cloud_color, cloud_mask);

    // Bevy uses reverse-Z: zero places the dome behind all scene geometry.
    return SkyFragmentOutput(vec4(color, 1.0), 0.0);
}
