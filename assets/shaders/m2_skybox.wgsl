#import bevy_pbr::forward_io::VertexOutput;

struct SkyboxM2Settings {
    color: vec4<f32>,
}

@group(2) @binding(0) var<uniform> material: SkyboxM2Settings;
@group(2) @binding(1) var base_texture: texture_2d<f32>;
@group(2) @binding(2) var base_sampler: sampler;

@fragment
fn fragment(in: VertexOutput) -> @location(0) vec4<f32> {
    let tex = textureSample(base_texture, base_sampler, in.uv);
    return tex * material.color;
}
