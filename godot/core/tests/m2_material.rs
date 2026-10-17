//! Retail M2 batch material binding against WebWowViewerCpp 1a8cccb's tables and real
//! model data (values read independently from the files).
use game_engine_core::m2;
use game_engine_core::m2_material::{
    self, BatchBinding, IDENTITY_MATRIX, MaterialTime, MaterialTracks, gx_blend, pixel_shader_id,
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

/// Avenging Wrath's real wing/core sheet and three scrolling feather sheets. Values
/// independently read from retail 2368645.m2 and 2373426.skin; no substituted art.
#[test]
fn avenging_wrath_batches_bind_authored_glows_and_premultiplied_blending() {
    let wings = model(2368645);
    assert_eq!(wings.batches.len(), 4);
    for (index, unit) in wings.batches.iter().enumerate() {
        let material = &wings.materials[usize::from(unit.render_flags_index)];
        assert_eq!((material.flags, material.blend_mode), (0x15, 7));
        assert_eq!(gx_blend(material.blend_mode), Ok(13));
        let binding = m2_material::batch_binding(&wings, unit, &[0; 3]).unwrap();
        let (shader, pixel, vertex, textures, wrap) = if index == 0 {
            (0x10, 1, 0, vec![Some(2373422)], 0)
        } else {
            (0x4014, 7, 2, vec![Some(2373423), Some(2373424)], 15)
        };
        assert_eq!(unit.shader_id, shader);
        assert_eq!(
            (binding.pixel_shader, binding.vertex_shader),
            (pixel, vertex)
        );
        assert_eq!(binding.textures, textures);
        assert_eq!(binding.texture_types, vec![0; binding.textures.len()]);
        assert_eq!(binding.texture_wrap, wrap);
        assert!(binding.apply_weight);
        if index > 0 {
            let tracks = MaterialTracks::of(&wings);
            let sample = m2_material::sample_material(&tracks, &binding, tracks.stand_time(900));
            assert_eq!(
                sample.opacity, 0.0,
                "Authored inactive feather sheet {index}"
            );
        }
    }
}

#[test]
fn avenging_wrath_local_opacity_follows_start_hold_and_decay_not_application_age() {
    use m2_material::MaterialTime;

    let wings = model(2368645);
    let tracks = MaterialTracks::of(&wings);
    let binding = m2_material::batch_binding(&wings, &wings.batches[1], &[0; 3]).unwrap();
    let sample = |sequence, time_ms, global_ms| {
        m2_material::sample_material(
            &tracks,
            &binding,
            MaterialTime {
                sequence,
                time_ms,
                global_ms,
            },
        )
    };
    assert_eq!(sample(0, 150, 900).opacity, 0.0);
    assert_eq!(sample(1, 150, 900).opacity, 1.0);
    assert_eq!(sample(1, 150, 10_000).opacity, 1.0);
    let scrolling = sample(1, 150, 450);
    assert_eq!(scrolling.texture_matrices[1][5], 0.5);
    assert_eq!(
        scrolling.texture_matrices,
        sample(2, 120, 450).texture_matrices
    );
    let decay = sample(2, 120, 900).opacity;
    assert!((decay - 47.0 / 167.0).abs() < 1.0 / 32767.0, "{decay}");
}

#[test]
fn single_timeline_material_tints_apply_to_other_authored_sequences() {
    use m2_material::MaterialTime;

    for fdid in [165644, 356426, 125854] {
        let effect = model(fdid);
        let tracks = MaterialTracks::of(&effect);
        let binding = m2_material::batch_binding(&effect, &effect.batches[0], &[0; 3]).unwrap();
        let sample = |sequence| {
            m2_material::sample_material(
                &tracks,
                &binding,
                MaterialTime {
                    sequence,
                    time_ms: 150,
                    global_ms: 450,
                },
            )
        };
        let authored_tint = sample(0).mesh_color;
        assert_ne!(authored_tint, [1.0; 3], "Source tinted model {fdid}");
        assert_eq!(sample(1).mesh_color, authored_tint, "Model {fdid} clip 1");
        assert_eq!(
            sample(effect.sequences.len() - 1).mesh_color,
            authored_tint,
            "Model {fdid} last clip"
        );
    }
}

/// 166338's colour track 0 is global sequence 0 (833ms): gold at 0/333ms,
/// purple at 833ms, including authored negative green. Output clamping belongs
/// to the UNORM/shader boundary, not this sampler (WebWowViewerCpp animate.h).
#[test]
fn avenging_wrath_light_mesh_keeps_authored_global_rgb_across_local_clips() {
    use m2_material::MaterialTime;

    let glow = model(166338);
    let tracks = MaterialTracks::of(&glow);
    let binding = m2_material::batch_binding(&glow, &glow.batches[0], &[0; 3]).unwrap();
    assert_eq!(binding.color, Some(0));
    assert_eq!(tracks.global_sequences[0], 833);
    let sample = |sequence, time_ms, global_ms| {
        m2_material::sample_material(
            &tracks,
            &binding,
            MaterialTime {
                sequence,
                time_ms,
                global_ms,
            },
        )
        .mesh_color
    };
    for (global_ms, expected) in [
        (0, [1.0, 132.0 / 255.0, 0.0]),
        (333, [1.0, 216.0 / 255.0, 0.0]),
        (700, [0.40416464, 0.17638433, 0.6562824]),
    ] {
        let rgb = sample(0, 0, global_ms);
        for channel in 0..3 {
            assert!((rgb[channel] - expected[channel]).abs() < 0.000001);
        }
        assert_eq!(sample(1, 150, global_ms), rgb);
        assert_eq!(sample(2, 300, global_ms + 833), rgb);
    }
    let purple = sample(0, 0, 832);
    assert!(
        purple[1] < 0.0,
        "Preserve authored negative green: {purple:?}"
    );
    assert!(purple[2] > purple[0], "Authored purple: {purple:?}");
}

#[test]
fn constant_local_material_keys_still_update_when_the_playing_clip_changes() {
    use m2_material::MaterialTime;

    let wings = model(2368645);
    let mut tracks = MaterialTracks::of(&wings);
    let binding = m2_material::batch_binding(&wings, &wings.batches[1], &[0; 3]).unwrap();
    let color = binding.color.unwrap();
    tracks.colors[color].opacity.sequences = vec![(vec![0], vec![32767]), (vec![0], vec![0])];
    assert!(m2_material::material_animates(&tracks, &binding));
    for (sequence, opacity) in [(0, 1.0), (1, 0.0)] {
        let sample = m2_material::sample_material(
            &tracks,
            &binding,
            MaterialTime {
                sequence,
                time_ms: 0,
                global_ms: 0,
            },
        );
        assert_eq!(sample.opacity, opacity);
    }
}

#[test]
fn avenging_wrath_resolved_batch_and_particle_textures_decode_as_authored_files() {
    use game_engine_core::blp;
    use std::collections::BTreeSet;

    let mut fdids = BTreeSet::new();
    for fdid in [2368643, 2368644, 2368645, 2368646, 165644] {
        let effect = model(fdid);
        for unit in &effect.batches {
            let binding = m2_material::batch_binding(&effect, unit, &[0; 3]).unwrap();
            for texture in binding.textures {
                fdids.insert(texture.expect("Avenging Wrath has no replaceable mesh texture"));
            }
        }
        for emitter in &effect.particle_emitters {
            if let Some(layers) = &emitter.multi_texture {
                for texture in layers.texture_fdids {
                    fdids.insert(texture.expect("Avenging Wrath particle layer must resolve"));
                }
            } else {
                fdids.insert(
                    emitter
                        .texture_fdid
                        .expect("Avenging Wrath emitter must resolve"),
                );
            }
        }
    }
    assert!(fdids.contains(&2373422));
    assert!(fdids.contains(&2373423));
    assert!(fdids.contains(&2373424));
    for fdid in fdids {
        let path = format!(
            "{}/../../data/textures/{fdid}.blp",
            env!("CARGO_MANIFEST_DIR")
        );
        let bytes = std::fs::read(&path).unwrap_or_else(|error| panic!("{path}: {error}"));
        let image = blp::decode_gpu(&bytes).unwrap_or_else(|error| panic!("{fdid}: {error}"));
        assert!(image.width > 0 && image.height > 0, "{fdid}");
        assert!(!image.data.is_empty(), "{fdid}");
    }
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
    let tracks = MaterialTracks::of(&portal);
    for elapsed in [900, 900 + 16667] {
        let sample = m2_material::sample_material(&tracks, &binding, tracks.stand_time(elapsed));
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
    let tracks = MaterialTracks::of(&waterfall);
    for elapsed in [700, 700 + 2634 * 3] {
        let matrix = m2_material::sample_material(&tracks, &binding, tracks.stand_time(elapsed))
            .texture_matrices[0];
        assert_eq!(&matrix[..4], &[1.0, 0.0, 0.0, 1.0]);
        assert!(matrix[4].abs() < 1e-6);
        assert!(
            (matrix[5] - -0.262_368_8).abs() < 1e-5,
            "{elapsed} ms: {matrix:?}"
        );
    }
}

#[test]
fn material_nonzero_modes_interpolate_ordinary_keys() {
    use game_engine_core::m2::AnimTrack;
    let effect = model(165644);
    let tracks = MaterialTracks::of(&effect);
    for mode in [1, 2, 3] {
        let scalar = AnimTrack {
            interpolation_type: mode,
            global_sequence: -1,
            sequences: vec![(vec![0, 1000], vec![0_i16, 32767])],
        };
        let vector = AnimTrack {
            interpolation_type: mode,
            global_sequence: -1,
            sequences: vec![(vec![0, 1000], vec![[0.0; 3], [1.0; 3]])],
        };
        let time = MaterialTime {
            sequence: 0,
            time_ms: 250,
            global_ms: 900,
        };
        let actual = m2_material::sample_fixed16(&scalar, &tracks, time).unwrap();
        assert!(
            (actual - 0.25).abs() < 1.0 / 32767.0,
            "mode {mode}: {actual}"
        );
        assert_eq!(
            m2_material::sample_vec3(&vector, &tracks, time),
            Some([0.25; 3])
        );
    }
}

#[test]
fn material_short_tables_do_not_replace_an_in_range_empty_timeline() {
    use game_engine_core::m2::AnimTrack;
    let effect = model(165644);
    let tracks = MaterialTracks::of(&effect);
    let track = AnimTrack {
        interpolation_type: 1,
        global_sequence: -1,
        sequences: vec![(vec![0, 1000], vec![[0.0; 3], [1.0; 3]]), (vec![], vec![])],
    };
    for (sequence, expected) in [
        (0, Some([0.25; 3])),
        (1, None),
        (2, Some([0.25; 3])),
        (9, Some([0.25; 3])),
    ] {
        assert_eq!(
            m2_material::sample_vec3(
                &track,
                &tracks,
                MaterialTime {
                    sequence,
                    time_ms: 250,
                    global_ms: 0
                }
            ),
            expected
        );
    }
}
