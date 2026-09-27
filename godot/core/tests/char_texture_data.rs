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
