use super::{
    Placeholders, TerrainMaterial, build_chunk_material, pack_shadow_map, shadow_bit_is_set,
    terrain_layer_animation_params, terrain_texture_repeat, texture_layer_params,
};
use crate::asset::adt;
use bevy::asset::Assets;
use bevy::image::Image;
use bevy::math::{Vec2, Vec4};

const TEST_TEXTURE_PARAM_FLAG_0: u32 = 0x10;
const TEST_TEXTURE_PARAM_FLAG_1: u32 = 0x20;
const TEST_ROTATING_TEXTURE_FLAGS: u32 = 0x40 | 0x19;
const TEST_OVERBRIGHT_LAYER_FLAG: u32 = 0x80;
const TEST_REFLECTION_LAYER_FLAG: u32 = 0x400;

#[test]
fn pack_shadow_map_expands_mcsh_bits_to_64x64_pixels() {
    let mut images = Assets::<Image>::default();
    let mut shadow_map = [0u8; 512];
    shadow_map[0] = 0b0000_0001;
    shadow_map[1] = 0b0000_0001;

    assert!(shadow_bit_is_set(&shadow_map, 0, 0));
    assert!(shadow_bit_is_set(&shadow_map, 0, 8));

    let handle = pack_shadow_map(&mut images, Some(&shadow_map));
    let image = images.get(&handle).expect("expected shadow image");
    let data = image.data.as_ref().expect("expected shadow pixels");

    assert_eq!(image.texture_descriptor.size.width, 64);
    assert_eq!(image.texture_descriptor.size.height, 64);
    assert_eq!(&data[0..4], &[0, 0, 0, 255]);
    assert_eq!(&data[4..8], &[255, 255, 255, 255]);
    assert_eq!(&data[32..36], &[0, 0, 0, 255]);
}

#[test]
fn texture_layer_params_use_mtxp_per_texture_index() {
    let tex_data = adt::AdtTexData {
        texture_amplifier: None,
        texture_fdids: vec![11, 22],
        height_texture_fdids: vec![],
        texture_flags: vec![0, 0],
        texture_params: vec![
            adt::TextureParams {
                flags: TEST_TEXTURE_PARAM_FLAG_0,
                height_scale: 1.25,
                height_offset: -0.5,
            },
            adt::TextureParams {
                flags: TEST_TEXTURE_PARAM_FLAG_1,
                height_scale: 0.75,
                height_offset: 0.125,
            },
        ],
        chunk_layers: vec![],
    };
    let layers = vec![
        adt::TextureLayer {
            texture_index: 1,
            flags: adt::MclyFlags::default(),
            effect_id: 0,
            material_id: 9,
            alpha_map: None,
        },
        adt::TextureLayer {
            texture_index: 0,
            flags: adt::MclyFlags::default(),
            effect_id: 0,
            material_id: 4,
            alpha_map: None,
        },
    ];

    let params = texture_layer_params(&tex_data, &layers);

    assert_eq!(params[0], Vec4::new(0.75, 0.125, 9.0, 0.0));
    assert_eq!(params[1], Vec4::new(1.25, -0.5, 4.0, 0.0));
}

#[test]
fn terrain_texture_repeat_scales_with_mamp() {
    assert_eq!(terrain_texture_repeat(None), 8.0);
    assert_eq!(terrain_texture_repeat(Some(0)), 8.0);
    assert_eq!(terrain_texture_repeat(Some(2)), 32.0);
}

#[test]
fn terrain_layer_animation_params_follow_mcly_rotation_and_speed() {
    let layers = vec![
        adt::TextureLayer {
            texture_index: 0,
            flags: adt::MclyFlags {
                raw: TEST_ROTATING_TEXTURE_FLAGS,
            },
            effect_id: 0,
            material_id: 0,
            alpha_map: None,
        },
        adt::TextureLayer {
            texture_index: 1,
            flags: adt::MclyFlags::default(),
            effect_id: 0,
            material_id: 0,
            alpha_map: None,
        },
    ];

    let params = terrain_layer_animation_params(&layers);

    assert!(
        Vec2::new(params[0].x, params[0].y).distance(Vec2::new(
            -std::f32::consts::SQRT_2,
            std::f32::consts::SQRT_2
        )) < 0.0001
    );
    assert_eq!(params[1], Vec4::ZERO);
}

#[test]
fn chunk_material_uses_mhid_height_textures_per_layer() {
    let mut terrain_materials = Assets::<TerrainMaterial>::default();
    let mut images = Assets::<Image>::default();
    let placeholder = Placeholders {
        image: images.add(Image::default()),
        alpha: images.add(Image::default()),
        cubemap: images.add(Image::default()),
    };
    let diffuse_0 = images.add(Image::default());
    let diffuse_1 = images.add(Image::default());
    let height_0 = images.add(Image::default());
    let height_1 = images.add(Image::default());
    let tex_data = adt::AdtTexData {
        texture_amplifier: None,
        texture_fdids: vec![11, 22],
        height_texture_fdids: vec![111, 222],
        texture_flags: vec![0, 0],
        texture_params: vec![],
        chunk_layers: vec![],
    };
    let chunk_tex = adt::ChunkTexLayers {
        layers: vec![
            adt::TextureLayer {
                texture_index: 1,
                flags: adt::MclyFlags::default(),
                effect_id: 0,
                material_id: 0,
                alpha_map: None,
            },
            adt::TextureLayer {
                texture_index: 0,
                flags: adt::MclyFlags::default(),
                effect_id: 0,
                material_id: 0,
                alpha_map: None,
            },
        ],
    };
    let ground_images = vec![Some(diffuse_0.clone()), Some(diffuse_1.clone())];
    let height_images = vec![Some(height_0.clone()), Some(height_1.clone())];

    let handle = build_chunk_material(
        &mut terrain_materials,
        &mut images,
        &tex_data,
        &chunk_tex,
        &ground_images,
        Some(&height_images),
        None,
        &placeholder,
    );
    let material = terrain_materials
        .get(&handle)
        .expect("expected terrain material");

    assert_eq!(material.height_0, height_1);
    assert_eq!(material.height_1, height_0);
}

#[test]
fn texture_layer_params_encode_overbright_multiplier() {
    let tex_data = adt::AdtTexData {
        texture_amplifier: None,
        texture_fdids: vec![11, 22],
        height_texture_fdids: vec![],
        texture_flags: vec![0, 0],
        texture_params: vec![
            adt::TextureParams {
                flags: 0,
                height_scale: 1.0,
                height_offset: 0.0,
            },
            adt::TextureParams {
                flags: 0,
                height_scale: 1.0,
                height_offset: 0.0,
            },
        ],
        chunk_layers: vec![],
    };
    let layers = vec![
        adt::TextureLayer {
            texture_index: 0,
            flags: adt::MclyFlags {
                raw: TEST_OVERBRIGHT_LAYER_FLAG,
            },
            effect_id: 0,
            material_id: 0,
            alpha_map: None,
        },
        adt::TextureLayer {
            texture_index: 1,
            flags: adt::MclyFlags::default(),
            effect_id: 0,
            material_id: 0,
            alpha_map: None,
        },
    ];

    let params = texture_layer_params(&tex_data, &layers);

    assert_eq!(params[0].w, 2.0);
    assert_eq!(params[1].w, 1.0);
}

#[test]
fn terrain_layer_animation_params_encode_reflection_flag() {
    let layers = vec![
        adt::TextureLayer {
            texture_index: 0,
            flags: adt::MclyFlags {
                raw: TEST_REFLECTION_LAYER_FLAG,
            },
            effect_id: 0,
            material_id: 0,
            alpha_map: None,
        },
        adt::TextureLayer {
            texture_index: 1,
            flags: adt::MclyFlags::default(),
            effect_id: 0,
            material_id: 0,
            alpha_map: None,
        },
    ];

    let params = terrain_layer_animation_params(&layers);

    assert_eq!(params[0].z, 1.0);
    assert_eq!(params[1].z, 0.0);
}
