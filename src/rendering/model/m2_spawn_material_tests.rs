use super::{load_batch_material, skybox_batch_needs_effect_combine};
use crate::asset;
use crate::m2_effect_material;
use crate::m2_spawn::{BatchMaterial, ground_offset_y};
use crate::skybox_m2_material::SkyboxM2Material;
use bevy::mesh::{Mesh, PrimitiveTopology};
use bevy::prelude::{AlphaMode, Assets, Image, StandardMaterial};

#[test]
fn ground_offset_uses_lowest_vertex_y() {
    let mut mesh = Mesh::new(
        PrimitiveTopology::TriangleList,
        bevy::asset::RenderAssetUsages::default(),
    );
    mesh.insert_attribute(
        Mesh::ATTRIBUTE_POSITION,
        vec![[0.0, 0.35, 0.0], [0.0, 1.0, 0.0], [0.0, 0.6, 0.0]],
    );
    let batch = asset::m2::M2RenderBatch {
        mesh,
        texture_fdid: None,
        texture_2_fdid: None,
        extra_texture_fdids: Vec::new(),
        texture_type: None,
        overlays: Vec::new(),
        render_flags: 0,
        blend_mode: 0,
        transparency: 1.0,
        texture_anim: None,
        texture_anim_2: None,
        use_uv_2_1: false,
        use_uv_2_2: false,
        use_env_map_2: false,
        shader_id: 0,
        texture_count: 0,
        uses_texture_combiner_combos: false,
        mesh_part_id: 0,
    };
    assert!((ground_offset_y(&[batch]) - 0.35).abs() < 0.001);
}

#[test]
fn invalid_blend_modes_use_additive_alpha_mode() {
    assert!(matches!(
        m2_effect_material::alpha_mode_for_blend(u16::MAX),
        AlphaMode::Add
    ));
}

#[test]
fn forced_skybox_batches_keep_dedicated_material_without_texture() {
    let mut images = Assets::<Image>::default();
    let mut materials = Assets::<StandardMaterial>::default();
    let mut effect_materials = Assets::<crate::m2_effect_material::M2EffectMaterial>::default();
    let mut skybox_materials = Assets::<SkyboxM2Material>::default();
    let batch = asset::m2::M2RenderBatch {
        mesh: Mesh::new(
            PrimitiveTopology::TriangleList,
            bevy::asset::RenderAssetUsages::default(),
        ),
        texture_fdid: None,
        texture_2_fdid: None,
        extra_texture_fdids: Vec::new(),
        texture_type: None,
        overlays: Vec::new(),
        render_flags: 0,
        blend_mode: 1,
        transparency: 1.0,
        texture_anim: None,
        texture_anim_2: None,
        use_uv_2_1: false,
        use_uv_2_2: false,
        use_env_map_2: false,
        shader_id: 0,
        texture_count: 0,
        uses_texture_combiner_combos: false,
        mesh_part_id: 0,
    };

    let material = load_batch_material(
        &batch,
        0,
        &mut images,
        &mut materials,
        &mut effect_materials,
        Some(&mut skybox_materials),
        true,
        None,
    );

    assert!(matches!(material, BatchMaterial::Skybox(_)));
}

#[test]
fn deathskybox_batches_use_features_beyond_base_texture_sampling() {
    let path = std::path::Path::new("data/models/skyboxes/deathskybox.m2");
    let model =
        crate::asset::m2::load_m2_uncached(path, &[0, 0, 0]).expect("load deathskybox model");

    assert!(
        model.batches.iter().any(|batch| {
            batch.texture_count > 1
                || batch.texture_anim.is_some()
                || batch.texture_anim_2.is_some()
                || batch.use_uv_2_1
                || batch.use_uv_2_2
                || batch.use_env_map_2
        }),
        "deathskybox batches unexpectedly only use static base-texture sampling"
    );
}

#[test]
fn deathskybox_single_texture_shader_batches_do_not_force_effect_combine() {
    let path = std::path::Path::new("data/models/skyboxes/deathskybox.m2");
    let model = crate::asset::m2::load_skybox_m2_uncached(path, &[0, 0, 0])
        .expect("load deathskybox model");
    let batch = model
        .batches
        .iter()
        .find(|batch| {
            batch.texture_count == 1 && batch.texture_2_fdid.is_none() && batch.shader_id == 0x0010
        })
        .expect("deathskybox single-texture batch");

    assert!(
        !skybox_batch_needs_effect_combine(batch),
        "single-texture skybox shader batch should stay on the base-texture path"
    );
}

#[test]
fn deathskybox_single_texture_shader_batch_keeps_second_texture_disabled() {
    let path = std::path::Path::new("data/models/skyboxes/deathskybox.m2");
    let model = crate::asset::m2::load_skybox_m2_uncached(path, &[0, 0, 0])
        .expect("load deathskybox model");
    let batch = model
        .batches
        .iter()
        .find(|batch| {
            batch.texture_count == 1 && batch.texture_2_fdid.is_none() && batch.shader_id == 0x0010
        })
        .expect("deathskybox single-texture batch");

    let mut images = Assets::<Image>::default();
    let mut materials = Assets::<StandardMaterial>::default();
    let mut effect_materials = Assets::<crate::m2_effect_material::M2EffectMaterial>::default();
    let mut skybox_materials = Assets::<SkyboxM2Material>::default();

    let material = load_batch_material(
        batch,
        0,
        &mut images,
        &mut materials,
        &mut effect_materials,
        Some(&mut skybox_materials),
        true,
        None,
    );

    let BatchMaterial::Skybox(handle) = material else {
        panic!("expected skybox material for deathskybox single-texture batch");
    };
    let material = skybox_materials
        .get(&handle)
        .expect("skybox material asset");
    assert_eq!(material.settings.combine_mode, 0x2);
    assert_eq!(material.settings.has_second_texture, 0);
}

#[test]
fn deathskybox_single_texture_batches_never_bind_second_texture_state() {
    let path = std::path::Path::new("data/models/skyboxes/deathskybox.m2");
    let model = crate::asset::m2::load_skybox_m2_uncached(path, &[0, 0, 0])
        .expect("load deathskybox model");

    let single_texture_batches: Vec<_> = model
        .batches
        .iter()
        .enumerate()
        .filter(|(_, batch)| batch.texture_count == 1 && batch.texture_2_fdid.is_none())
        .collect();

    assert!(
        !single_texture_batches.is_empty(),
        "deathskybox must keep at least one authored single-texture batch"
    );

    for (batch_index, batch) in single_texture_batches {
        assert!(
            !skybox_batch_needs_effect_combine(batch),
            "single-texture deathskybox batch {batch_index} should not switch to effect combine"
        );

        let mut images = Assets::<Image>::default();
        let mut materials = Assets::<StandardMaterial>::default();
        let mut effect_materials = Assets::<crate::m2_effect_material::M2EffectMaterial>::default();
        let mut skybox_materials = Assets::<SkyboxM2Material>::default();

        let material = load_batch_material(
            batch,
            batch_index,
            &mut images,
            &mut materials,
            &mut effect_materials,
            Some(&mut skybox_materials),
            true,
            None,
        );

        let BatchMaterial::Skybox(handle) = material else {
            panic!("expected skybox material for single-texture deathskybox batch");
        };
        let material = skybox_materials
            .get(&handle)
            .expect("skybox material asset");
        assert_eq!(
            material.settings.has_second_texture, 0,
            "single-texture deathskybox batch {batch_index} should not advertise a second texture"
        );
    }
}

#[test]
fn cloudsky_batches_are_two_texture_and_use_unhandled_shader_ids() {
    let path = std::path::Path::new("data/models/skyboxes/11xp_cloudsky01.m2");
    let model = crate::asset::m2::load_skybox_m2_uncached(path, &[0, 0, 0])
        .expect("load 11xp cloud skybox model");

    let shader_ids: std::collections::BTreeSet<_> =
        model.batches.iter().map(|batch| batch.shader_id).collect();
    let two_texture_batches = model
        .batches
        .iter()
        .filter(|batch| batch.texture_2_fdid.is_some())
        .count();

    assert_eq!(model.batches.len(), 54);
    assert_eq!(two_texture_batches, 54);
    assert_eq!(
        shader_ids,
        std::collections::BTreeSet::from([0x4014, 0x8012, 0x8016])
    );
}

#[test]
fn cloudsky_modern_shader_batches_keep_runtime_second_texture_sampling() {
    let path = std::path::Path::new("data/models/skyboxes/11xp_cloudsky01.m2");
    let model = crate::asset::m2::load_skybox_m2_uncached(path, &[0, 0, 0])
        .expect("load 11xp cloud skybox model");
    let batch = model
        .batches
        .iter()
        .find(|batch| batch.texture_2_fdid.is_some() && matches!(batch.shader_id, 0x8012 | 0x8016))
        .expect("cloud skybox batch with supported modern shader id");

    let mut images = Assets::<Image>::default();
    let mut materials = Assets::<StandardMaterial>::default();
    let mut effect_materials = Assets::<crate::m2_effect_material::M2EffectMaterial>::default();
    let mut skybox_materials = Assets::<SkyboxM2Material>::default();

    let material = load_batch_material(
        batch,
        0,
        &mut images,
        &mut materials,
        &mut effect_materials,
        Some(&mut skybox_materials),
        true,
        None,
    );

    let BatchMaterial::Skybox(handle) = material else {
        panic!("expected skybox material for cloud skybox batch");
    };
    let material = skybox_materials
        .get(&handle)
        .expect("skybox material asset");
    assert_eq!(
        material.settings.has_second_texture, 1,
        "cloud skybox batch with supported modern shader id should keep second-texture sampling enabled"
    );
}

#[test]
fn cloudsky_advanced_effect_batches_require_more_than_two_texture_stages() {
    let path = std::path::Path::new("data/models/skyboxes/11xp_cloudsky01.m2");
    let model = crate::asset::m2::load_skybox_m2_uncached(path, &[0, 0, 0])
        .expect("load 11xp cloud skybox model");
    let advanced_batch = model
        .batches
        .iter()
        .find(|batch| batch.shader_id == 0x8012)
        .expect("cloud skybox advanced-effect batch");

    assert!(
        !advanced_batch.uses_texture_combiner_combos,
        "0x8012 cloud skybox batches are direct shader effects, not combiner-table batches"
    );
    assert!(
        advanced_batch.texture_count >= 3,
        "0x8012 cloud skybox batches need at least three texture stages"
    );
}

#[test]
fn cloudsky_masked_crossfade_batches_preserve_fourth_stage_texture() {
    let path = std::path::Path::new("data/models/skyboxes/11xp_cloudsky01.m2");
    let model = crate::asset::m2::load_skybox_m2_uncached(path, &[0, 0, 0])
        .expect("load 11xp cloud skybox model");
    let masked_batch = model
        .batches
        .iter()
        .find(|batch| batch.shader_id == 0x8016)
        .expect("cloud skybox masked crossfade batch");

    assert_eq!(masked_batch.texture_count, 4);
    assert_eq!(masked_batch.extra_texture_fdids.len(), 2);
}

#[test]
fn authored_skybox_models_reference_locally_available_textures() {
    for skybox_path in [
        std::path::Path::new("data/models/skyboxes/11xp_cloudsky01.m2"),
        std::path::Path::new("data/models/skyboxes/deathskybox.m2"),
    ] {
        let model = crate::asset::m2::load_m2_uncached(skybox_path, &[0, 0, 0])
            .unwrap_or_else(|err| panic!("load skybox {}: {err}", skybox_path.display()));
        let mut missing = std::collections::BTreeSet::new();
        for fdid in model
            .batches
            .iter()
            .flat_map(|batch| [batch.texture_fdid, batch.texture_2_fdid])
            .flatten()
        {
            let local = crate::asset::asset_cache::texture(fdid)
                .unwrap_or_else(|| std::path::PathBuf::from(format!("data/textures/{fdid}.blp")));
            if !local.exists() {
                missing.insert(fdid);
            }
        }
        assert!(
            missing.is_empty(),
            "skybox {} is missing extracted textures for FDIDs: {:?}",
            skybox_path.display(),
            missing
        );
    }
}

#[test]
fn dedicated_skybox_loader_keeps_authored_skybox_render_batches() {
    for skybox_path in [
        std::path::Path::new("data/models/skyboxes/11xp_cloudsky01.m2"),
        std::path::Path::new("data/models/skyboxes/deathskybox.m2"),
    ] {
        let model = crate::asset::m2::load_skybox_m2_uncached(skybox_path, &[0, 0, 0])
            .unwrap_or_else(|err| panic!("load skybox {}: {err}", skybox_path.display()));
        let shader_ids: std::collections::BTreeSet<_> =
            model.batches.iter().map(|batch| batch.shader_id).collect();
        let blend_modes: std::collections::BTreeSet<_> =
            model.batches.iter().map(|batch| batch.blend_mode).collect();
        let two_texture_batches = model
            .batches
            .iter()
            .filter(|batch| batch.texture_2_fdid.is_some())
            .count();
        eprintln!(
            "{} batches={} shader_ids={shader_ids:?} blend_modes={blend_modes:?} two_texture_batches={two_texture_batches}",
            skybox_path.display(),
            model.batches.len()
        );
        for (index, batch) in model.batches.iter().take(5).enumerate() {
            eprintln!(
                "  batch[{index}] tex1={:?} tex2={:?} extras={:?} texture_count={} shader_id=0x{:04x} blend_mode={}",
                batch.texture_fdid,
                batch.texture_2_fdid,
                batch.extra_texture_fdids,
                batch.texture_count,
                batch.shader_id,
                batch.blend_mode
            );
        }

        assert!(
            !model.batches.is_empty(),
            "skybox {} unexpectedly built no render batches in dedicated skybox mode",
            skybox_path.display()
        );
    }
}
