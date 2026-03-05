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
}

@group(#{MATERIAL_BIND_GROUP}) @binding(0) var<uniform> sky: SkyUniforms;

@fragment
fn fragment(in: VertexOutput) -> @location(0) vec4<f32> {
    let dir = normalize(in.world_position.xyz - view.world_position);

    if dir.y < 0.0 {
        return vec4(sky.sky_smog.rgb, 1.0);
    }

    let elev = clamp(asin(dir.y) / (PI / 2.0), 0.0, 1.0);

    var color: vec3<f32>;

    if elev < 0.1 {
        // 0.0–0.1: smog
        color = sky.sky_smog.rgb;
    } else if elev < 0.3 {
        // 0.1–0.3: smog → band2
        let t = smoothstep(0.1, 0.3, elev);
        color = mix(sky.sky_smog.rgb, sky.sky_band2.rgb, t);
    } else if elev < 0.5 {
        // 0.3–0.5: band2 → band1
        let t = smoothstep(0.3, 0.5, elev);
        color = mix(sky.sky_band2.rgb, sky.sky_band1.rgb, t);
    } else if elev < 0.7 {
        // 0.5–0.7: band1 → middle
        let t = smoothstep(0.5, 0.7, elev);
        color = mix(sky.sky_band1.rgb, sky.sky_middle.rgb, t);
    } else {
        // 0.7–1.0: middle → top
        let t = smoothstep(0.7, 1.0, elev);
        color = mix(sky.sky_middle.rgb, sky.sky_top.rgb, t);
    }

    return vec4(color, 1.0);
}
