// Retail world lighting shared by world materials.
// Source: Deamon87/WebWowViewerCpp 1a8cccbeffc46231c6497e6b3f5bfbf3507d8071,
// wowViewerLib/shaders/slang/common/commonLightFunctions.slang (calcLight,
// applyAndMixAmbients) and commonFogFunctions.slang (fogColors/fogMix).
// Mirrors src/rendering/lighting/retail_light.rs (retail_shade).
//
// The client shades in authored (gamma) space: texels and LightData colours are
// sRGB bytes / 255. Callers convert textures with `linear_to_gamma`, shade, fog,
// and return `gamma_to_linear` of the result to Bevy's linear render target.

#import bevy_pbr::{
    mesh_view_bindings as view_bindings,
    mesh_view_types,
    mesh_types::MESH_FLAGS_SHADOW_RECEIVER_BIT,
    shadows,
}

// GPU copy of RetailSceneLight (authored space); sun_direction is the Bevy world
// direction the sunlight travels.
struct RetailSceneLight {
    ambient: vec4<f32>,
    horizon_ambient: vec4<f32>,
    ground_ambient: vec4<f32>,
    direct: vec4<f32>,
    sun_direction: vec4<f32>,
}

fn linear_to_gamma(linear: vec3<f32>) -> vec3<f32> {
    let low = linear * 12.92;
    let high = 1.055 * pow(max(linear, vec3<f32>(0.0)), vec3<f32>(1.0 / 2.4)) - 0.055;
    return select(high, low, linear <= vec3<f32>(0.0031308));
}

fn gamma_to_linear(gamma: vec3<f32>) -> vec3<f32> {
    let low = gamma / 12.92;
    let high = pow((max(gamma, vec3<f32>(0.0)) + 0.055) / 1.055, vec3<f32>(2.4));
    return select(high, low, gamma <= vec3<f32>(0.04045));
}

fn apply_and_mix_ambients(light: RetailSceneLight, n_dot_l: f32, n_dot_up: f32) -> vec3<f32> {
    var hemisphere: vec3<f32>;
    if n_dot_up >= 0.0 {
        hemisphere = mix(light.horizon_ambient.rgb, light.ambient.rgb, vec3<f32>(n_dot_up));
    } else {
        hemisphere = mix(light.horizon_ambient.rgb, light.ground_ambient.rgb, vec3<f32>(-n_dot_up));
    }
    let sky = hemisphere * 1.1;
    let ground = hemisphere * 0.7;
    return mix(ground, sky, vec3<f32>(0.5 + 0.5 * n_dot_l));
}

// calcLight for an exterior surface without point lights, specular or emission:
// sqrt((diffuse * (ambient + direct * nDotL))^2), which is the gamma term itself.
fn retail_shade(
    light: RetailSceneLight,
    diffuse: vec3<f32>,
    normal: vec3<f32>,
    sun_visibility: f32,
) -> vec3<f32> {
    let n = normalize(normal);
    let n_dot_up = dot(n, vec3<f32>(0.0, 1.0, 0.0));
    let n_dot_l = clamp(dot(n, -normalize(light.sun_direction.xyz)), 0.0, 1.0);
    let ambient = apply_and_mix_ambients(light, n_dot_l, n_dot_up);
    let direct = light.direct.rgb * n_dot_l * sun_visibility;
    return diffuse * (ambient + direct);
}

// Shadow-map visibility of the sun (directional light 0) for a shadow receiver.
fn retail_sun_visibility(
    world_position: vec4<f32>,
    normal: vec3<f32>,
    frag_coord_xy: vec2<f32>,
    mesh_flags: u32,
) -> f32 {
    if view_bindings::lights.n_directional_lights == 0u
        || (mesh_flags & MESH_FLAGS_SHADOW_RECEIVER_BIT) == 0u
        || (view_bindings::lights.directional_lights[0].flags
            & mesh_view_types::DIRECTIONAL_LIGHT_FLAGS_SHADOWS_ENABLED_BIT) == 0u {
        return 1.0;
    }
    let view_z = dot(vec4<f32>(
        view_bindings::view.view_from_world[0].z,
        view_bindings::view.view_from_world[1].z,
        view_bindings::view.view_from_world[2].z,
        view_bindings::view.view_from_world[3].z,
    ), world_position);
    return shadows::fetch_directional_shadow(0u, world_position, normal, view_z, frag_coord_xy);
}

// commonFogFunctions.slang validateFogColor: additive and modulating blends fog
// towards the colour that leaves the framebuffer unchanged.
fn retail_fog_color(fog_color: vec3<f32>, gx_blend: u32) -> vec3<f32> {
    switch gx_blend {
        case 3u, 10u, 13u: {
            return vec3<f32>(0.0);
        }
        case 4u: {
            return vec3<f32>(1.0);
        }
        case 5u: {
            return vec3<f32>(0.5);
        }
        default: {
            return fog_color;
        }
    }
}

// Fog in authored space using the camera's DistanceFog range and colour (the
// world camera's DistanceFog is set from RetailSceneLight). Returns `color`
// unchanged when the camera has no fog.
fn retail_apply_fog(color: vec3<f32>, world_position: vec3<f32>, gx_blend: u32) -> vec3<f32> {
#ifdef DISTANCE_FOG
    let fog = view_bindings::fog;
    let distance = length(world_position - view_bindings::view.world_position.xyz);
    var amount = 0.0;
    if fog.mode == mesh_view_types::FOG_MODE_LINEAR {
        amount = 1.0 - clamp((fog.be.y - distance) / (fog.be.y - fog.be.x), 0.0, 1.0);
    } else if fog.mode == mesh_view_types::FOG_MODE_EXPONENTIAL {
        amount = 1.0 - 1.0 / exp(distance * fog.be.x);
    } else if fog.mode == mesh_view_types::FOG_MODE_EXPONENTIAL_SQUARED {
        let scaled = distance * fog.be.x;
        amount = 1.0 - 1.0 / exp(scaled * scaled);
    }
    let fog_color = retail_fog_color(linear_to_gamma(fog.base_color.rgb), gx_blend);
    return mix(color, fog_color, amount * fog.base_color.a);
#else
    return color;
#endif
}
