#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum WmoLayerCombine {
    None,
    AlphaBlend,
    Add,
    Multiply,
    Mod2x,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct WmoShaderDescriptor {
    pub env_reflection: bool,
    pub metallic: bool,
    pub emissive: bool,
    pub second_layer: WmoLayerCombine,
    pub third_layer: WmoLayerCombine,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct WmoSurfaceParams {
    pub roughness: f32,
    pub reflectance: f32,
    pub metallic: f32,
}

pub(crate) fn describe_wmo_shader(shader: u32) -> WmoShaderDescriptor {
    use WmoLayerCombine::{Add, AlphaBlend, Mod2x, Multiply, None};

    match shader {
        2 => shader_descriptor(false, true, false, None, None),
        3 => shader_descriptor(true, false, false, None, None),
        5 => shader_descriptor(true, true, false, None, None),
        6 => shader_descriptor(false, false, false, AlphaBlend, None),
        7 => shader_descriptor(true, true, false, AlphaBlend, Add),
        8 => shader_descriptor(false, false, false, AlphaBlend, None),
        9 => shader_descriptor(false, false, true, Add, None),
        10 => shader_descriptor(true, false, false, None, None),
        11 => shader_descriptor(true, true, false, AlphaBlend, AlphaBlend),
        12 => shader_descriptor(true, true, true, Add, Add),
        13 => shader_descriptor(false, false, false, AlphaBlend, None),
        14 => shader_descriptor(true, false, false, None, None),
        15 => shader_descriptor(false, false, true, Add, None),
        17 => shader_descriptor(true, true, true, Add, Add),
        18 | 19 => shader_descriptor(false, false, false, Mod2x, None),
        20 => shader_descriptor(false, false, false, AlphaBlend, None),
        22 => shader_descriptor(true, true, false, Multiply, None),
        _ => shader_descriptor(false, false, false, None, None),
    }
}

fn shader_descriptor(
    env_reflection: bool,
    metallic: bool,
    emissive: bool,
    second_layer: WmoLayerCombine,
    third_layer: WmoLayerCombine,
) -> WmoShaderDescriptor {
    WmoShaderDescriptor {
        env_reflection,
        metallic,
        emissive,
        second_layer,
        third_layer,
    }
}

const DEFAULT_ROUGHNESS: f32 = 0.88;
const DEFAULT_REFLECTANCE: f32 = 0.18;
const PROP_ROUGHNESS: f32 = 0.97;
const PROP_REFLECTANCE: f32 = 0.02;
const ENV_ROUGHNESS: f32 = 0.35;
const ENV_REFLECTANCE: f32 = 0.45;
const METAL_ROUGHNESS: f32 = 0.25;
const METAL_REFLECTANCE: f32 = 0.5;
const METAL_METALLIC: f32 = 0.85;

pub(crate) fn wmo_surface_params(
    has_texture: bool,
    unculled: bool,
    shader: u32,
) -> WmoSurfaceParams {
    let descriptor = describe_wmo_shader(shader);
    if descriptor.metallic {
        return metal_surface_params();
    }

    if descriptor.env_reflection {
        return env_surface_params();
    }

    default_surface_params(unculled || !has_texture)
}

fn metal_surface_params() -> WmoSurfaceParams {
    WmoSurfaceParams {
        roughness: METAL_ROUGHNESS,
        reflectance: METAL_REFLECTANCE,
        metallic: METAL_METALLIC,
    }
}

fn env_surface_params() -> WmoSurfaceParams {
    WmoSurfaceParams {
        roughness: ENV_ROUGHNESS,
        reflectance: ENV_REFLECTANCE,
        metallic: 0.0,
    }
}

fn default_surface_params(prop_like_surface: bool) -> WmoSurfaceParams {
    let roughness = if prop_like_surface {
        PROP_ROUGHNESS
    } else {
        DEFAULT_ROUGHNESS
    };
    let reflectance = if prop_like_surface {
        PROP_REFLECTANCE
    } else {
        DEFAULT_REFLECTANCE
    };
    WmoSurfaceParams {
        roughness,
        reflectance,
        metallic: 0.0,
    }
}

pub(crate) fn composite_wmo_shader_layer(
    base_pixels: &mut [u8],
    overlay_pixels: &[u8],
    combine: WmoLayerCombine,
) {
    if matches!(combine, WmoLayerCombine::None) {
        return;
    }

    for (base_pixel, overlay_pixel) in base_pixels
        .chunks_exact_mut(4)
        .zip(overlay_pixels.chunks_exact(4))
    {
        match combine {
            WmoLayerCombine::None => {}
            WmoLayerCombine::AlphaBlend => blend_alpha(base_pixel, overlay_pixel),
            WmoLayerCombine::Add => blend_add(base_pixel, overlay_pixel),
            WmoLayerCombine::Multiply => blend_multiply(base_pixel, overlay_pixel),
            WmoLayerCombine::Mod2x => blend_mod2x(base_pixel, overlay_pixel),
        }
    }
}

fn blend_alpha(base_pixel: &mut [u8], overlay_pixel: &[u8]) {
    let overlay_alpha = overlay_pixel[3] as u16;
    let base_alpha = 255_u16.saturating_sub(overlay_alpha);

    for channel in 0..3 {
        base_pixel[channel] = ((base_pixel[channel] as u16 * base_alpha
            + overlay_pixel[channel] as u16 * overlay_alpha
            + 127)
            / 255) as u8;
    }

    base_pixel[3] = base_pixel[3].max(overlay_pixel[3]);
}

fn blend_add(base_pixel: &mut [u8], overlay_pixel: &[u8]) {
    let overlay_alpha = overlay_pixel[3] as u16;
    for channel in 0..3 {
        let overlay_value = (overlay_pixel[channel] as u16 * overlay_alpha + 127) / 255;
        let added = base_pixel[channel] as u16 + overlay_value;
        base_pixel[channel] = added.min(255) as u8;
    }
    base_pixel[3] = base_pixel[3].max(overlay_pixel[3]);
}

fn blend_multiply(base_pixel: &mut [u8], overlay_pixel: &[u8]) {
    for channel in 0..3 {
        base_pixel[channel] =
            ((base_pixel[channel] as u16 * overlay_pixel[channel] as u16 + 127) / 255) as u8;
    }
    base_pixel[3] = base_pixel[3].max(overlay_pixel[3]);
}

fn blend_mod2x(base_pixel: &mut [u8], overlay_pixel: &[u8]) {
    for channel in 0..3 {
        let modulated = base_pixel[channel] as u16 * overlay_pixel[channel] as u16 * 2;
        let scaled = ((modulated + 127) / 255).min(255);
        base_pixel[channel] = scaled as u8;
    }
    base_pixel[3] = base_pixel[3].max(overlay_pixel[3]);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn shader_descriptor_marks_env_metal_and_emissive_families() {
        assert_eq!(
            describe_wmo_shader(12),
            WmoShaderDescriptor {
                env_reflection: true,
                metallic: true,
                emissive: true,
                second_layer: WmoLayerCombine::Add,
                third_layer: WmoLayerCombine::Add,
            }
        );
        assert_eq!(
            describe_wmo_shader(18),
            WmoShaderDescriptor {
                env_reflection: false,
                metallic: false,
                emissive: false,
                second_layer: WmoLayerCombine::Mod2x,
                third_layer: WmoLayerCombine::None,
            }
        );
        assert_eq!(
            describe_wmo_shader(3),
            WmoShaderDescriptor {
                env_reflection: true,
                metallic: false,
                emissive: false,
                second_layer: WmoLayerCombine::None,
                third_layer: WmoLayerCombine::None,
            }
        );
    }

    #[test]
    fn composite_wmo_shader_layer_alpha_blends_two_layer_diffuse() {
        let mut base = vec![100, 100, 100, 255];
        let overlay = vec![200, 50, 0, 128];

        composite_wmo_shader_layer(&mut base, &overlay, WmoLayerCombine::AlphaBlend);

        assert_eq!(base, vec![150, 75, 50, 255]);
    }

    #[test]
    fn composite_wmo_shader_layer_modulates_for_mod2x_variants() {
        let mut base = vec![64, 128, 255, 255];
        let overlay = vec![128, 64, 128, 255];

        composite_wmo_shader_layer(&mut base, &overlay, WmoLayerCombine::Mod2x);

        assert_eq!(base, vec![64, 64, 255, 255]);
    }

    #[test]
    fn wmo_surface_params_raise_reflection_for_env_metal_shaders() {
        assert_eq!(
            wmo_surface_params(true, false, 5),
            WmoSurfaceParams {
                roughness: 0.25,
                reflectance: 0.5,
                metallic: 0.85,
            }
        );
        assert_eq!(
            wmo_surface_params(true, false, 0),
            WmoSurfaceParams {
                roughness: 0.88,
                reflectance: 0.18,
                metallic: 0.0,
            }
        );
    }
}
