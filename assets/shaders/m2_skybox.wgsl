#import bevy_pbr::forward_io::VertexOutput;

struct SkyboxM2Settings {
    color: vec4<f32>,
}

@group(#{MATERIAL_BIND_GROUP}) @binding(0) var<uniform> material: SkyboxM2Settings;
@group(#{MATERIAL_BIND_GROUP}) @binding(1) var base_texture: texture_2d<f32>;
@group(#{MATERIAL_BIND_GROUP}) @binding(2) var base_sampler: sampler;

@fragment
fn fragment(in: VertexOutput) -> @location(0) vec4<f32> {
    // Authored M2 skyboxes in the reference clients are rendered without distance fog.
    let tex = textureSample(base_texture, base_sampler, in.uv);
    return tex * material.color;
}
