//! Retail M2 batch material binding against WebWowViewerCpp 1a8cccb's tables and real
//! model data (values read independently from the files).
use game_engine_core::m2;
use game_engine_core::m2_material::{
    self, BatchBinding, IDENTITY_MATRIX, MaterialTracks, gx_blend, pixel_shader_id,
    vertex_shader_id,
};

fn model(fdid: u32) -> m2::Model {
    let read = |name: String| {
        let path = format!("{}/../../data/models/{name}", env!("CARGO_MANIFEST_DIR"));
        std::fs::read(&path).unwrap_or_else(|error| panic!("{path}: {error}"))
    };
    m2::parse_model(&read(format!("{fdid}.m2")), &read(format!("{fdid}00.skin"))).unwrap()
}

#[test]
fn shader_ids_resolve_like_get_pixel_and_vertex_shader_id() {
    // (texture count, shader id) -> (pixel, vertex): the most used ids in local assets.
    let cases = [
        (1, 0x0000, 0, 0),   // Combiners_Opaque, Diffuse_T1
        (1, 0x0010, 1, 0),   // Combiners_Mod
        (1, 0x0090, 1, 1),   // Diffuse_Env
        (1, 0x4010, 1, 10),  // Diffuse_T2
        (2, 0x4011, 6, 2),   // Mod_Mod, Diffuse_T1_T2
        (2, 0x4014, 7, 2),   // Mod_Mod2x
        (2, 0x4016, 9, 2),   // Mod_Mod2xNA
        (2, 0x000e, 4, 3),   // Opaque_Mod2xNA, Diffuse_T1_Env
        (2, 0x0011, 6, 7),   // Mod_Mod, Diffuse_T1_T1
        (2, 0x0007, 13, 7),  // Opaque_AddAlpha
        (2, 0x8000, 12, 3),  // table 0: Opaque_Mod2xNA_Alpha, Diffuse_T1_Env
        (2, 0x8001, 13, 3),  // table 1: Opaque_AddAlpha
        (2, 0x8002, 14, 3),  // table 2: Opaque_AddAlpha_Alpha
        (2, 0x8015, 6, 12),  // table 21: Mod_Mod, Diffuse_EdgeFade_T1_T2
        (6, 0x801c, 32, 14), // table 28: Guild_Opaque, Diffuse_T1_T2_T1
        (1, 0x8022, 1, 9),   // table 34: Combiners_Mod, Diffuse_EdgeFade_T1
        (2, 0x8023, 36, 12), // table 35: Mod_Mod_Depth
    ];
    for (count, id, pixel, vertex) in cases {
        assert_eq!(pixel_shader_id(count, id), Ok(pixel), "pixel of {id:#x}");
        assert_eq!(vertex_shader_id(count, id), Ok(vertex), "vertex of {id:#x}");
    }
    assert!(pixel_shader_id(2, 0x8024).is_err());
    assert!(vertex_shader_id(2, 0x8024).is_err());
    assert_eq!(
        (0..8)
            .map(|mode| gx_blend(mode).unwrap())
            .collect::<Vec<_>>(),
        [0, 1, 2, 10, 3, 4, 5, 13]
    );
}

/// Alliance shield 1036844 batch 0: shader 0x8000 over TXID 1036765 and the env map
/// 249237, no texture flags (clamped), weight 0 from transparency lookup 0, batch flags
/// 0x90 (texture weight applied).
#[test]
fn shield_env_batch_binds_both_textures_clamped() {
    let shield = model(1036844);
    let binding = m2_material::batch_binding(&shield, &shield.batches[0], &[0; 3]).unwrap();
    assert_eq!(
        binding,
        BatchBinding {
            pixel_shader: 12,
            vertex_shader: 3,
            textures: vec![Some(1036765), Some(249237)],
            texture_types: vec![0, 0],
            texture_wrap: 0,
            texture_transforms: [None, None],
            texture_weights: [Some(0), None, None],
            color: None,
            apply_weight: true,
        }
    );
}

/// World tree portal 197068 batch 0: a local rotation track (sequence 0 lasts 16667 ms)
/// keyed 0 -> 4300 ms from identity to (0, 0, -0.72469, 0.68907), about the texture centre.
#[test]
fn portal_texture_rotation_turns_about_the_centre_and_loops_stand() {
    let portal = model(197068);
    let binding = m2_material::batch_binding(&portal, &portal.batches[0], &[0; 3]).unwrap();
    assert_eq!(binding.texture_transforms, [Some(0), None]);
    assert_eq!(binding.textures, vec![Some(1394302)]);
    assert_eq!(binding.texture_wrap, 3);
    let expected = [
        0.948_707_7,
        -0.316_154_7,
        0.316_154_7,
        0.948_707_7,
        -0.132_431_2,
        0.183_723_5,
    ];
    for elapsed in [900, 900 + 16667] {
        let sample = m2_material::sample_material(&MaterialTracks::of(&portal), &binding, elapsed);
        for (actual, wanted) in sample.texture_matrices[0].iter().zip(expected) {
            assert!(
                (actual - wanted).abs() < 1e-5,
                "{elapsed} ms: {:?}",
                sample.texture_matrices[0]
            );
        }
        assert_eq!(sample.texture_matrices[1], IDENTITY_MATRIX);
    }
    assert!(m2_material::material_animates(
        &MaterialTracks::of(&portal),
        &binding
    ));
}

/// Elwynn waterfall 189958 batch 0: a single-texture local V scroll, 0 -> -0.5 at 1334 ms
/// in a 2634 ms Stand, wrapped with the texture.
#[test]
fn waterfall_single_texture_scrolls_and_loops_stand() {
    let waterfall = model(189958);
    let binding = m2_material::batch_binding(&waterfall, &waterfall.batches[0], &[0; 3]).unwrap();
    assert_eq!((binding.pixel_shader, binding.vertex_shader), (1, 0));
    assert_eq!(binding.texture_wrap, 3);
    for elapsed in [700, 700 + 2634 * 3] {
        let matrix =
            m2_material::sample_material(&MaterialTracks::of(&waterfall), &binding, elapsed)
                .texture_matrices[0];
        assert_eq!(&matrix[..4], &[1.0, 0.0, 0.0, 1.0]);
        assert!(matrix[4].abs() < 1e-6);
        assert!(
            (matrix[5] - -0.262_368_8).abs() < 1e-5,
            "{elapsed} ms: {matrix:?}"
        );
    }
}
