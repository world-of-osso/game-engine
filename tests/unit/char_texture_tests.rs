use super::*;
use std::collections::HashMap;

#[test]
fn blend_mode_1_respects_partial_alpha() {
    let mut dst = [100, 120, 140, 255];
    let src = [200, 40, 20, 64];

    super::blend_pixel(&mut dst, 0, &src, 0, true);

    assert_eq!(dst, [125, 99, 109, 255]);
}

#[test]
fn opaque_modes_still_copy_nonzero_alpha_pixels() {
    let mut dst = [100, 120, 140, 255];
    let src = [200, 40, 20, 64];

    super::blend_pixel(&mut dst, 0, &src, 0, false);

    assert_eq!(dst, [200, 40, 20, 255]);
}

#[test]
fn full_head_atlas_layers_do_not_stretch_across_body_canvas() {
    let mut sections = HashMap::new();
    sections.insert(
        (2, 10),
        TextureSection {
            x: 2,
            y: 0,
            width: 2,
            height: 2,
        },
    );
    let data = CharTextureData {
        layers: Vec::new(),
        sections,
        layouts: HashMap::new(),
    };
    let layer = TextureLayer {
        texture_type: 6,
        layer: 0,
        blend_mode: 0,
        section_bitmask: super::FULL_TEXTURE_SECTION_MASK,
        target_id: 10,
        layout_id: 2,
    };
    let tex = vec![
        255, 0, 0, 255, 255, 0, 0, 255, 255, 0, 0, 255, 255, 0, 0, 255,
    ];
    let mut pixels = vec![0u8; 4 * 2 * 4];

    super::blit_layer(
        &data,
        super::BlitLayerInput {
            pixels: &mut pixels,
            canvas_w: 4,
            tex: &tex,
            tex_w: 2,
            tex_h: 2,
            layer: &layer,
            layout_id: 2,
        },
    );

    assert_eq!(&pixels[0..8], &[0, 0, 0, 0, 0, 0, 0, 0]);
    assert_eq!(&pixels[8..16], &[255, 0, 0, 255, 255, 0, 0, 255]);
    assert_eq!(&pixels[24..32], &[255, 0, 0, 255, 255, 0, 0, 255]);
}

#[test]
fn hd_layout_is_converted_to_runtime_body_and_head_textures() {
    let mut sections = HashMap::new();
    sections.insert(
        (103, 9),
        TextureSection {
            x: 1024,
            y: 0,
            width: 1024,
            height: 1024,
        },
    );
    let data = CharTextureData {
        layers: Vec::new(),
        sections,
        layouts: HashMap::new(),
    };
    let mut pixels = vec![0u8; (2048 * 1024 * 4) as usize];
    for y in 0..1024u32 {
        for x in 0..2048u32 {
            let idx = ((y * 2048 + x) * 4) as usize;
            if x < 1024 {
                pixels[idx..idx + 4].copy_from_slice(&[10, 20, 30, 255]);
            } else {
                pixels[idx..idx + 4].copy_from_slice(&[40, 50, 60, 255]);
            }
        }
    }

    let composed = super::runtime_textures_from_layout(&data, pixels, 103, 2048, 1024);

    assert_eq!((composed.body.1, composed.body.2), (1024, 512));
    assert_eq!(&composed.body.0[0..4], &[10, 20, 30, 255]);
    let head = composed.head.expect("expected HD head atlas");
    assert_eq!((head.1, head.2), (512, 512));
    assert_eq!(&head.0[0..4], &[40, 50, 60, 255]);
}

#[test]
fn hd_layout_extracts_runtime_hair_texture_from_section_ten() {
    let mut sections = HashMap::new();
    sections.insert(
        (103, 10),
        TextureSection {
            x: 1024,
            y: 0,
            width: 1024,
            height: 1024,
        },
    );
    let data = CharTextureData {
        layers: Vec::new(),
        sections,
        layouts: HashMap::new(),
    };
    let mut pixels = vec![0u8; (2048 * 1024 * 4) as usize];
    for y in 0..1024u32 {
        for x in 0..2048u32 {
            let idx = ((y * 2048 + x) * 4) as usize;
            if x < 1024 {
                pixels[idx..idx + 4].copy_from_slice(&[10, 20, 30, 255]);
            } else {
                pixels[idx..idx + 4].copy_from_slice(&[40, 50, 60, 255]);
            }
        }
    }

    let hair = super::runtime_texture_for_section(&data, pixels, 103, 2048, 1024, 10)
        .expect("expected HD hair atlas");

    assert_eq!((hair.1, hair.2), (512, 512));
    assert_eq!(&hair.0[0..4], &[40, 50, 60, 255]);
}

#[test]
fn hd_glove_item_sections_change_runtime_body_atlas_pixels() {
    let data = super::load_test_data();

    let base = data
        .composite_model_textures(&[], &[], 103)
        .expect("base HD composite");
    let gloved = data
        .composite_model_textures(&[], &[(1, 149592), (2, 154135)], 103)
        .expect("gloved HD composite");

    let arm_lower = super::scaled_section(*data.sections.get(&(103, 1)).expect("section 1"), 2);
    let hand = super::scaled_section(*data.sections.get(&(103, 2)).expect("section 2"), 2);

    let sample = |pixels: &[u8], width: u32, section: TextureSection| {
        let x = section.x + section.width / 2;
        let y = section.y + section.height / 2;
        let idx = ((y * width + x) * 4) as usize;
        pixels[idx..idx + 4].to_vec()
    };

    assert_ne!(
        sample(&base.body.0, base.body.1, arm_lower),
        sample(&gloved.body.0, gloved.body.1, arm_lower)
    );
    assert_ne!(
        sample(&base.body.0, base.body.1, hand),
        sample(&gloved.body.0, gloved.body.1, hand)
    );
}

#[test]
fn loud_hd_glove_changes_pixels_at_sampled_glove_uv() {
    let data = super::load_test_data();

    let base = data
        .composite_model_textures(&[], &[], 103)
        .expect("base HD composite");
    let gloved = data
        .composite_model_textures(&[], &[(1, 1318191), (2, 1318200)], 103)
        .expect("gloved HD composite");

    let sample_uv = |pixels: &[u8], width: u32, height: u32, u: f32, v: f32| {
        let x = (u * width as f32).floor().clamp(0.0, (width - 1) as f32) as u32;
        let y = (v * height as f32).floor().clamp(0.0, (height - 1) as f32) as u32;
        let idx = ((y * width + x) * 4) as usize;
        pixels[idx..idx + 4].to_vec()
    };

    assert_ne!(
        sample_uv(&base.body.0, base.body.1, base.body.2, 0.125, 0.375),
        sample_uv(&gloved.body.0, gloved.body.1, gloved.body.2, 0.125, 0.375)
    );
}

#[test]
fn hd_boot_item_sections_change_runtime_body_atlas_pixels() {
    let data = super::load_test_data();

    let base = data
        .composite_model_textures(&[], &[], 103)
        .expect("base HD composite");
    let booted = data
        .composite_model_textures(&[], &[(6, 155028), (7, 152769)], 103)
        .expect("booted HD composite");

    let leg_lower = super::scaled_section(*data.sections.get(&(103, 6)).expect("section 6"), 2);
    let foot = super::scaled_section(*data.sections.get(&(103, 7)).expect("section 7"), 2);

    let sample = |pixels: &[u8], width: u32, section: TextureSection| {
        let x = section.x + section.width / 2;
        let y = section.y + section.height / 2;
        let idx = ((y * width + x) * 4) as usize;
        pixels[idx..idx + 4].to_vec()
    };

    assert_ne!(
        sample(&base.body.0, base.body.1, leg_lower),
        sample(&booted.body.0, booted.body.1, leg_lower)
    );
    assert_ne!(
        sample(&base.body.0, base.body.1, foot),
        sample(&booted.body.0, booted.body.1, foot)
    );
}

#[test]
fn loud_hd_boot_changes_pixels_at_sampled_boot_uv() {
    let data = super::load_test_data();

    let base = data
        .composite_model_textures(&[], &[], 103)
        .expect("base HD composite");
    let booted = data
        .composite_model_textures(&[], &[(6, 155028), (7, 152769)], 103)
        .expect("booted HD composite");

    let sample_uv = |pixels: &[u8], width: u32, height: u32, u: f32, v: f32| {
        let x = (u * width as f32).floor().clamp(0.0, (width - 1) as f32) as u32;
        let y = (v * height as f32).floor().clamp(0.0, (height - 1) as f32) as u32;
        let idx = ((y * width + x) * 4) as usize;
        pixels[idx..idx + 4].to_vec()
    };

    assert_ne!(
        sample_uv(&base.body.0, base.body.1, base.body.2, 0.375, 0.78),
        sample_uv(&booted.body.0, booted.body.1, booted.body.2, 0.375, 0.78)
    );
}
