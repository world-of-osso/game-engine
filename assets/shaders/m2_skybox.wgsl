#import bevy_pbr::{
    pbr_fragment::pbr_input_from_standard_material,
    pbr_types::StandardMaterial,
}

struct SkyboxM2Settings {
    color: vec4<f32>,
}

@group(2) @binding(0) var<uniform> material: SkyboxM2Settings;
@group(2) @binding(1) var base_texture: texture_2d<f32>;
@group(2) @binding(2) var base_sampler: sampler;

@fragment
fn fragment(
    in: bevy_pbr::forward_io::VertexOutput,
    is_front: bool,
) -> @location(0) vec4<f32> {
    var pbr_input = pbr_input_from_standard_material(in, is_front);
    let uv = pbr_input.material.uv;
    let tex = textureSample(base_texture, base_sampler, uv);
    return tex * material.color;
}
