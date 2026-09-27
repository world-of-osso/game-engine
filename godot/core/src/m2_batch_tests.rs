use super::asset::m2_format::m2_anim::{ColorAnimTracks, TextureAnimTracks};
use super::m2::{self, AnimTrack, Model, TextureUnit};
use std::path::Path;

fn fixture(name: &str) -> Model {
    let directory = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../data/models");
    let model = std::fs::read(directory.join(format!("{name}.m2"))).unwrap();
    let skin = std::fs::read(directory.join(format!("{name}00.skin"))).unwrap();
    if name == "humanmale_hd" {
        let skeleton = std::fs::read(directory.join("humanmale_hd.skel")).unwrap();
        m2::parse_model_with_skeleton(&model, &skin, Some(&skeleton)).unwrap()
    } else {
        m2::parse_model(&model, &skin).unwrap()
    }
}

#[test]
fn real_boar_batches_resolve_authoring_and_draw_order() {
    let model = fixture("boar");
    let batches = m2::resolve_render_batches(&model, false, |_| None).unwrap();
    assert!(!batches.is_empty());
    assert!(
        batches
            .windows(2)
            .all(|pair| (pair[0].priority_plane, pair[0].material_layer)
                <= (pair[1].priority_plane, pair[1].material_layer))
    );
    for batch in &batches {
        let unit = &model.batches[batch.source_unit_index];
        assert_eq!(
            batch.mesh_part_id,
            model.submeshes[batch.submesh_index].mesh_part_id
        );
        assert_eq!(batch.shader_id, unit.shader_id);
        assert_eq!(batch.texture_count, unit.texture_count);
        assert!(batch.transparency > 0.0);
    }
}

#[test]
fn real_hd_model_batch_inventory() {
    let model = fixture("humanmale_hd");
    let batches = m2::resolve_render_batches(&model, false, |_| None).unwrap();
    assert_eq!(model.batches.len(), 113);
    assert_eq!(batches.len(), 113);
    assert_eq!(model.indices.len(), 147966);
    assert!(
        batches
            .windows(2)
            .all(|pair| (pair[0].priority_plane, pair[0].material_layer)
                <= (pair[1].priority_plane, pair[1].material_layer))
    );
}

fn track<T>(value: T) -> AnimTrack<T> {
    AnimTrack {
        interpolation_type: 0,
        global_sequence: -1,
        sequences: vec![(vec![0], vec![value])],
    }
}

fn unit(submesh_index: u16, priority_plane: i8, material_layer: u16) -> TextureUnit {
    TextureUnit {
        flags: 0,
        priority_plane,
        shader_id: 0x4014,
        submesh_index,
        color_index: 0,
        render_flags_index: 0,
        material_layer,
        texture_count: 4,
        texture_id: 0,
        texture_coord_index: 0,
        transparency_index: 0,
        texture_animation_id: 0,
    }
}

#[test]
fn authored_material_texture_animation_and_opacity_resolution() {
    let mut model = fixture("boar");
    model.batches = vec![unit(0, 1, 2), unit(0, -1, 7)];
    model.materials[0].flags = 4;
    model.materials[0].blend_mode = 3;
    model.texture_types = vec![0, 0, 0, 0];
    model.texture_fdids = vec![101, 102, 103, 104];
    model.texture_lookup = vec![0, 1, 2, 3];
    model.texture_unit_lookup = vec![2, 0];
    model.uses_texture_combiner_combos = true;
    model.transparency_lookup = vec![0];
    model.transparency_tracks = vec![track(16384)];
    model.color_tracks = vec![ColorAnimTracks {
        color: track([1.0; 3]),
        opacity: track(16384),
    }];
    let anim = |value| TextureAnimTracks {
        translation: track([value, 0.0, 0.0]),
        rotation: track([0; 4]),
        scale: track([1.0; 3]),
    };
    model.texture_animations = vec![anim(0.25), anim(0.75)];
    model.uv_animation_lookup = vec![0, 1];
    let batches = m2::resolve_render_batches(&model, false, |_| None).unwrap();
    assert_eq!(
        batches
            .iter()
            .map(|b| b.source_unit_index)
            .collect::<Vec<_>>(),
        vec![1, 0]
    );
    let batch = &batches[0];
    assert_eq!(
        (
            batch.render_flags,
            batch.blend_mode,
            batch.shader_id,
            batch.texture_count
        ),
        (4, 3, 0x4014, 4)
    );
    assert_eq!(
        (
            batch.texture_fdid,
            batch.texture_2_fdid,
            &batch.extra_texture_fdids[..]
        ),
        (Some(101), Some(102), &[103, 104][..])
    );
    assert_eq!(
        (
            batch.texture_type,
            batch.overlays.len(),
            batch.uses_texture_combiner_combos
        ),
        (Some(0), 0, true)
    );
    assert_eq!(
        (batch.use_uv_2_1, batch.use_uv_2_2, batch.use_env_map_2),
        (true, false, true)
    );
    assert_eq!(
        (
            batch.transparency_track_index,
            batch.color_opacity_track_index
        ),
        (Some(0), Some(0))
    );
    assert_eq!(
        batch.transparency_anim.as_ref().unwrap().sequences[0].1[0],
        16384
    );
    assert_eq!(
        batch.color_opacity_anim.as_ref().unwrap().sequences[0].1[0],
        16384
    );
    assert!((batch.transparency - 0.25).abs() < 0.001);
    assert_eq!(
        batch.texture_anim.as_ref().unwrap().sequences[0].1[0],
        [0.25, 0.0, 0.0]
    );
    assert_eq!(
        batch.texture_anim_2.as_ref().unwrap().sequences[0].1[0],
        [0.75, 0.0, 0.0]
    );
}

#[test]
fn zero_opacity_policy_filters_before_mesh_material_binding() {
    let mut model = fixture("boar");
    model.batches = vec![unit(0, 0, 0)];
    model.transparency_lookup = vec![0];
    model.transparency_tracks = vec![track(0)];
    model.color_tracks = vec![ColorAnimTracks {
        color: track([1.0; 3]),
        opacity: track(16384),
    }];
    assert!(
        m2::resolve_render_batches(&model, false, |_| None)
            .unwrap()
            .is_empty()
    );
    let kept = m2::resolve_render_batches(&model, true, |_| None).unwrap();
    assert_eq!(kept.len(), 1);
    assert_eq!(kept[0].transparency, 0.0);
}

#[test]
fn invalid_submesh_is_an_error_even_for_zero_opacity() {
    let mut model = fixture("boar");
    model.batches = vec![unit(u16::MAX, 0, 0)];
    assert!(
        m2::resolve_render_batches(&model, false, |_| None)
            .err()
            .unwrap()
            .contains("Batch submesh_index 65535")
    );
}
