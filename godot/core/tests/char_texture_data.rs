use std::collections::HashMap;

use game_engine_core::char_texture_data::{
    CharTextureData, TextureLayer, TextureLayout, TextureSection,
};

fn layer(
    target_id: u16,
    order: u32,
    blend_mode: u32,
    mask: i64,
    texture_type: u32,
) -> TextureLayer {
    TextureLayer {
        texture_type,
        layer: order,
        blend_mode,
        section_bitmask: mask,
        target_id,
        layout_id: 1,
    }
}

fn data(layers: Vec<TextureLayer>) -> CharTextureData {
    CharTextureData::from_parts(
        layers,
        HashMap::from([(
            (1, 0),
            TextureSection {
                x: 1,
                y: 0,
                width: 2,
                height: 1,
            },
        )]),
        HashMap::from([(
            1,
            TextureLayout {
                width: 4,
                height: 2,
            },
        )]),
    )
}

fn texture(fdid: u32) -> Option<(Vec<u8>, u32, u32)> {
    if fdid == 2 {
        return Some((vec![10, 20, 30, 255, 40, 50, 60, 255], 2, 1));
    }
    let pixel = match fdid {
        1 => [1, 2, 3, 255],
        3 => [40, 50, 60, 255],
        4 => [90, 100, 110, 128],
        5 => [70, 80, 90, 255],
        _ => return None,
    };
    Some((pixel.to_vec(), 1, 1))
}

#[test]
fn layer_order_target_last_assignment_and_section_coordinates_produce_exact_bytes() {
    let data = data(vec![layer(8, 8, 15, 1, 1), layer(7, 2, 0, 1, 1)]);
    let (pixels, width, height) = data
        .composite_with(&[(8, 3), (7, 2), (8, 4)], 1, texture)
        .unwrap();
    assert_eq!((width, height), (4, 2));
    assert_eq!(
        pixels,
        [
            0, 0, 0, 0, 50, 60, 70, 255, 65, 75, 85, 255, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
            0, 0, 0, 0, 0, 0,
        ]
    );
}

#[test]
fn default_seed_precedes_layers_and_items_follow_in_input_order() {
    let data = data(vec![layer(7, 1, 0, 1, 1)]);
    let result = data
        .composite_model_textures_with(&[(7, 2)], &[(0, 3), (0, 5)], 1, 1, texture)
        .unwrap();
    assert_eq!(
        result.body,
        (
            vec![
                1, 2, 3, 255, 70, 80, 90, 255, 70, 80, 90, 255, 1, 2, 3, 255, 1, 2, 3, 255, 1, 2,
                3, 255, 1, 2, 3, 255, 1, 2, 3, 255,
            ],
            4,
            2
        )
    );
    assert!(result.head.is_none());
    assert!(result.hair.is_none());
}

#[test]
fn missing_layout_and_missing_decoded_image_keep_original_absence_behavior() {
    let data = data(vec![layer(7, 0, 0, 1, 1)]);
    assert!(
        data.composite_model_textures_with(&[], &[], 999, 1, texture)
            .is_none()
    );
    let result = data
        .composite_model_textures_with(&[(7, 999)], &[(0, 999)], 1, 999, texture)
        .unwrap();
    assert_eq!(result.body.0, vec![0; 32]);
}

#[test]
fn hd_body_head_and_hair_keep_nearest_neighbor_crop_bytes() {
    let data = CharTextureData::from_parts(
        vec![TextureLayer {
            texture_type: 6,
            target_id: 10,
            layout_id: 103,
            ..layer(10, 0, 0, -1, 6)
        }],
        HashMap::from([
            (
                (103, 9),
                TextureSection {
                    x: 1024,
                    y: 0,
                    width: 1024,
                    height: 1024,
                },
            ),
            (
                (103, 10),
                TextureSection {
                    x: 1024,
                    y: 0,
                    width: 1024,
                    height: 1024,
                },
            ),
        ]),
        HashMap::from([(
            103,
            TextureLayout {
                width: 2048,
                height: 1024,
            },
        )]),
    );
    let result = data
        .composite_model_textures_with(&[(10, 3)], &[], 103, 1, texture)
        .unwrap();
    assert_eq!((result.body.1, result.body.2), (1024, 512));
    assert_eq!(&result.body.0[..4], &[1, 2, 3, 255]);
    assert_eq!(
        result.head.as_ref().map(|h| (h.1, h.2, &h.0[..4])),
        Some((512, 512, &[1, 2, 3, 255][..]))
    );
    assert_eq!(
        result.hair.as_ref().map(|h| (h.1, h.2, &h.0[..4])),
        Some((512, 512, &[40, 50, 60, 255][..]))
    );
}

/// ChrModelTextureLayer.BlendMode 4/6/7 (multiply/overlay/screen, WMVx
/// `CharacterTextureBuilder::BlendMode`) tint the composited layers below them.
fn tinted_base(blend_mode: u32, tint: [u8; 4]) -> Vec<u8> {
    let data = data(vec![layer(1, 0, 0, -1, 1), layer(30, 4, blend_mode, -1, 1)]);
    let (pixels, _, _) = data
        .composite_with(&[(1, 10), (30, 11)], 1, |fdid| match fdid {
            10 => Some((vec![200, 150, 100, 255], 1, 1)),
            11 => Some((tint.to_vec(), 1, 1)),
            _ => None,
        })
        .unwrap();
    pixels[..4].to_vec()
}

#[test]
fn overlay_multiply_and_screen_layers_tint_the_base_skin() {
    assert_eq!(tinted_base(6, [80, 97, 97, 255]), [180, 125, 76, 255]);
    assert_eq!(tinted_base(4, [80, 97, 97, 255]), [62, 57, 38, 255]);
    assert_eq!(tinted_base(7, [80, 97, 97, 255]), [218, 190, 159, 255]);
}

#[test]
fn tint_layers_weight_the_blend_by_source_alpha() {
    assert_eq!(tinted_base(6, [80, 97, 97, 128]), [189, 137, 87, 255]);
    assert_eq!(tinted_base(6, [80, 97, 97, 0]), [200, 150, 100, 255]);
}

#[test]
fn straight_alpha_layers_blend_instead_of_replacing_the_base() {
    assert_eq!(tinted_base(9, [80, 97, 97, 128]), [139, 123, 98, 255]);
}

/// Only TextureType 1 layers belong to the body atlas; accessory slots such as
/// TextureType 20 (target 38) are separate M2 textures and must not paint the body.
#[test]
fn non_body_texture_type_layers_do_not_paint_the_body_atlas() {
    let data = data(vec![layer(1, 0, 1, -1, 1), layer(38, 1, 1, -1, 20)]);
    let result = data
        .composite_model_textures_with(&[(1, 10), (38, 11)], &[], 1, 10, |fdid| match fdid {
            10 => Some((vec![200, 150, 100, 255], 1, 1)),
            11 => Some((vec![1, 2, 3, 255], 1, 1)),
            _ => None,
        })
        .unwrap();
    assert!(
        result
            .body
            .0
            .chunks_exact(4)
            .all(|pixel| pixel == [200, 150, 100, 255])
    );
}

/// A texture type on its own ChrModelMaterial canvas: its layers only, in layer
/// order, a -1 mask covering the whole canvas (even hair target 10, which the body
/// atlas puts in section 10), later opaque layers replacing earlier ones.
#[test]
fn separate_texture_type_composites_its_layers_on_its_material_canvas() {
    let data = data(vec![
        layer(1, 0, 0, -1, 1),
        layer(10, 1, 1, -1, 9),
        layer(12, 2, 1, -1, 9),
        layer(13, 0, 1, -1, 7),
    ])
    .with_material_sizes(HashMap::from([((1, 9), (2, 2)), ((1, 7), (1, 1))]));
    assert_eq!(data.separate_texture_types(1), [7, 9]);

    let (pixels, width, height) = data
        .composite_texture_type(&[(10, 1), (12, 3)], 1, 9, texture)
        .unwrap();
    assert_eq!((width, height), (2, 2));
    assert_eq!(
        pixels,
        [40, 50, 60, 255].repeat(4),
        "target 12 is drawn last"
    );

    let (pixels, ..) = data
        .composite_texture_type(&[(10, 1)], 1, 9, texture)
        .unwrap();
    assert_eq!(pixels, [1, 2, 3, 255].repeat(4));
    assert!(
        data.composite_texture_type(&[(1, 1)], 1, 9, texture)
            .is_none(),
        "no selected material targets a type 9 layer"
    );
}

#[test]
fn item_textures_alpha_blend_over_the_body() {
    // Wow.exe Paste (solarityclient composer.rs `alpha_blend`): an item texel of alpha
    // 128 mixes with the skin below instead of replacing it.
    let data = data(vec![]);
    let result = data
        .composite_model_textures_with(&[], &[(0, 4)], 1, 1, texture)
        .unwrap();
    let blended =
        |item: u8, skin: u8| ((u16::from(item) * 128 + u16::from(skin) * 127) / 255) as u8;
    assert_eq!(
        result.body.0[4..8],
        [blended(90, 1), blended(100, 2), blended(110, 3), 255]
    );
}

#[test]
fn item_textures_expand_by_stock_paste_scale() {
    // Wow.exe PasteScale (solarityclient composer.rs `blend_scaled_rect`): a texture half
    // its section's size expands 2x with odd texels averaging their neighbours.
    let data = CharTextureData::from_parts(
        vec![],
        HashMap::from([(
            (1, 0),
            TextureSection {
                x: 0,
                y: 0,
                width: 4,
                height: 2,
            },
        )]),
        HashMap::from([(
            1,
            TextureLayout {
                width: 4,
                height: 2,
            },
        )]),
    );
    let result = data
        .composite_model_textures_with(&[], &[(0, 2)], 1, 1, texture)
        .unwrap();
    let row = [
        10, 20, 30, 255, 25, 35, 45, 255, 40, 50, 60, 255, 40, 50, 60, 255,
    ];
    assert_eq!(result.body.0[..16], row);
    assert_eq!(result.body.0[16..], row);
}

#[test]
fn translucent_first_layer_keeps_its_colour_on_an_empty_canvas() {
    // Straight-alpha "over" an empty canvas (WMVx CharacterTextureBuilder::mergeLayer,
    // QPainter SourceOver): the eye texel keeps its colour and alpha instead of
    // darkening towards the canvas' transparent black.
    let data = data(vec![layer(25, 10, 1, -1, 19)])
        .with_material_sizes(HashMap::from([((1, 19), (1, 1))]));
    let (pixels, ..) = data
        .composite_texture_type(&[(25, 4)], 1, 19, texture)
        .unwrap();
    assert_eq!(pixels, [90, 100, 110, 128]);
}
