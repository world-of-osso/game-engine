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

fn sample_gradient(elev: f32) -> vec3<f32> {
    var color: vec3<f32>;

    if elev < 0.1 {
        color = sky.sky_smog.rgb;
    } else if elev < 0.3 {
        let t = smoothstep(0.1, 0.3, elev);
        color = mix(sky.sky_smog.rgb, sky.sky_band2.rgb, t);
    } else if elev < 0.5 {
        let t = smoothstep(0.3, 0.5, elev);
        color = mix(sky.sky_band2.rgb, sky.sky_band1.rgb, t);
    } else if elev < 0.7 {
        let t = smoothstep(0.5, 0.7, elev);
        color = mix(sky.sky_band1.rgb, sky.sky_middle.rgb, t);
    } else {
        let t = smoothstep(0.7, 1.0, elev);
        color = mix(sky.sky_middle.rgb, sky.sky_top.rgb, t);
    }

    return color;
}

@fragment
fn fragment(in: VertexOutput) -> @location(0) vec4<f32> {
    let dir = normalize(in.world_position.xyz - view.world_position);

    if dir.y < 0.0 {
        return vec4(sky.sky_smog.rgb, 1.0);
    }

    let elev = clamp(asin(dir.y) / (PI / 2.0), 0.0, 1.0);
    var color = sample_gradient(elev);

    let uv = vec2(
        fract(atan2(dir.z, dir.x) / (2.0 * PI) + 0.5 + sky.cloud_params.y),
        fract(0.5 - asin(dir.y) / PI + sky.cloud_params.z),
    );
    let cloud_a = textureSample(cloud_texture, cloud_sampler, uv).r;
    let cloud_b = textureSample(
        cloud_texture,
        cloud_sampler,
        fract(uv * vec2(1.9, 1.35) + vec2(0.17, 0.29)),
    )
    .r;
    let cloud_shape = mix(cloud_a, cloud_b, 0.35);
    let density = clamp(sky.cloud_params.x, 0.0, 1.0);
    let threshold = mix(0.92, 0.32, density);
    let horizon_mask = smoothstep(0.02, 0.18, elev) * (1.0 - smoothstep(0.82, 0.98, elev));
    let cloud_mask = smoothstep(threshold, 1.0, cloud_shape) * horizon_mask;

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

    return vec4(color, 1.0);
}
