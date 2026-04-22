use super::{M2_SHADER_MODULATE, load_batch_material, skybox_batch_needs_effect_combine};
use crate::asset;
use crate::m2_effect_material;
use crate::m2_spawn::{BatchMaterial, ground_offset_y};
use crate::skybox_m2_material::SkyboxM2Material;
use bevy::mesh::{Mesh, PrimitiveTopology};
use bevy::prelude::{AlphaMode, Assets, Image, StandardMaterial};

const SHADER_SINGLE_TEXTURE: u16 = M2_SHADER_MODULATE;
const SHADER_MOD2X: u16 = 0x4014;
const SHADER_THREE_STAGE: u16 = 0x8012;
const SHADER_FOUR_STAGE: u16 = 0x8016;
const COMBINE_STATIC_MOD2X: u32 = 0x000E;

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
        transparency_track_index: None,
        color_opacity_track_index: None,
        transparency_anim: None,
        color_opacity_anim: None,
        texture_anim: None,
        texture_anim_2: None,
        use_uv_2_1: false,
        use_uv_2_2: false,
        use_env_map_2: false,
        shader_id: 0,
        texture_count: 0,
        uses_texture_combiner_combos: false,
        priority_plane: 0,
        material_layer: 0,
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
        transparency_track_index: None,
        color_opacity_track_index: None,
        transparency_anim: None,
        color_opacity_anim: None,
        texture_anim: None,
        texture_anim_2: None,
        use_uv_2_1: false,
        use_uv_2_2: false,
        use_env_map_2: false,
        shader_id: 0,
        texture_count: 0,
        uses_texture_combiner_combos: false,
        priority_plane: 0,
        material_layer: 0,
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
        None,
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
            batch.texture_count == 1
                && batch.texture_2_fdid.is_none()
                && batch.shader_id == SHADER_SINGLE_TEXTURE
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
            batch.texture_count == 1
                && batch.texture_2_fdid.is_none()
                && batch.shader_id == SHADER_SINGLE_TEXTURE
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
        None,
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
            None,
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
        std::collections::BTreeSet::from([SHADER_MOD2X, SHADER_THREE_STAGE, SHADER_FOUR_STAGE])
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
        .find(|batch| {
            batch.texture_2_fdid.is_some()
                && matches!(batch.shader_id, SHADER_THREE_STAGE | SHADER_FOUR_STAGE)
        })
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
        None,
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

fn load_cloudsky_model() -> crate::asset::m2::M2Model {
    let path = std::path::Path::new("data/models/skyboxes/11xp_cloudsky01.m2");
    crate::asset::m2::load_skybox_m2_uncached(path, &[0, 0, 0])
        .expect("load 11xp cloud skybox model")
}

fn cloudsky_batch_by_shader_id(
    model: &crate::asset::m2::M2Model,
    shader_id: u16,
) -> (usize, &crate::asset::m2::M2RenderBatch) {
    model
        .batches
        .iter()
        .enumerate()
        .find(|(_, batch)| batch.shader_id == shader_id)
        .unwrap_or_else(|| panic!("cloud skybox missing shader id 0x{shader_id:04x}"))
}

fn traced_cloudsky_material(
    batch: &crate::asset::m2::M2RenderBatch,
    batch_index: usize,
) -> SkyboxM2Material {
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
        None,
        None,
    );
    let BatchMaterial::Skybox(handle) = material else {
        panic!("expected cloud skybox batch to route through SkyboxM2Material");
    };
    skybox_materials
        .get(&handle)
        .cloned()
        .expect("cloud skybox material asset")
}

#[test]
fn cloudsky_shader_4014_trace_captures_two_stage_mod2x_path() {
    let model = load_cloudsky_model();
    let (batch_index, batch) = cloudsky_batch_by_shader_id(&model, SHADER_MOD2X);
    let material = traced_cloudsky_material(batch, batch_index);

    assert_eq!(batch.texture_count, 2);
    assert!(!batch.uses_texture_combiner_combos);
    assert_eq!(batch.extra_texture_fdids.len(), 0);
    assert_eq!(
        material.settings.combine_mode, COMBINE_STATIC_MOD2X,
        "SHADER_MOD2X cloudsky batches currently route through static fragment-mode combine mapping"
    );
    assert_eq!(material.settings.has_second_texture, 1);
    assert_eq!(material.settings.has_third_texture, 0);
    assert_eq!(material.settings.has_fourth_texture, 0);
    assert_eq!(material.settings.uv_mode_1, u32::from(batch.use_uv_2_1));
    assert_eq!(material.settings.uv_mode_2, u32::from(batch.use_uv_2_2));
    assert_eq!(material.settings.uv_mode_3, 0);
    assert_eq!(material.settings.uv_mode_4, 0);
}

#[test]
fn cloudsky_shader_8012_trace_captures_three_stage_primary_uv_path() {
    let model = load_cloudsky_model();
    let (batch_index, batch) = cloudsky_batch_by_shader_id(&model, SHADER_THREE_STAGE);
    let material = traced_cloudsky_material(batch, batch_index);

    assert_eq!(batch.texture_count, 3);
    assert_eq!(batch.extra_texture_fdids.len(), 1);
    assert_eq!(
        material.settings.combine_mode,
        u32::from(SHADER_THREE_STAGE)
    );
    assert_eq!(material.settings.has_second_texture, 1);
    assert_eq!(material.settings.has_third_texture, 1);
    assert_eq!(material.settings.has_fourth_texture, 0);
    assert_eq!(material.settings.uv_mode_1, 0);
    assert_eq!(material.settings.uv_mode_2, 0);
    assert_eq!(material.settings.uv_mode_3, 0);
    assert_eq!(material.settings.uv_mode_4, 0);
}

#[test]
fn cloudsky_shader_8016_trace_captures_four_stage_masked_uv4_path() {
    let model = load_cloudsky_model();
    let (batch_index, batch) = cloudsky_batch_by_shader_id(&model, SHADER_FOUR_STAGE);
    let material = traced_cloudsky_material(batch, batch_index);

    assert_eq!(batch.texture_count, 4);
    assert_eq!(batch.extra_texture_fdids.len(), 2);
    assert_eq!(material.settings.combine_mode, u32::from(SHADER_FOUR_STAGE));
    assert_eq!(material.settings.has_second_texture, 1);
    assert_eq!(material.settings.has_third_texture, 1);
    assert_eq!(material.settings.has_fourth_texture, 1);
    assert_eq!(material.settings.uv_mode_1, 0);
    assert_eq!(material.settings.uv_mode_2, 0);
    assert_eq!(material.settings.uv_mode_3, 0);
    assert_eq!(material.settings.uv_mode_4, 1);
}

fn synthetic_missing_texture_fdid() -> u32 {
    for candidate in [u32::MAX, u32::MAX - 1, 4_000_000_000, 3_500_000_000] {
        let path = crate::asset::asset_cache::texture(candidate)
            .unwrap_or_else(|| std::path::PathBuf::from(format!("data/textures/{candidate}.blp")));
        if !path.exists() {
            return candidate;
        }
    }
    panic!("expected at least one high synthetic FDID to be absent from local texture cache");
}

#[test]
fn cloudsky_shader_8012_stage_binding_disables_missing_optional_third_stage() {
    let model = load_cloudsky_model();
    let (batch_index, batch) = cloudsky_batch_by_shader_id(&model, SHADER_THREE_STAGE);
    let mut batch = batch.clone();
    batch.extra_texture_fdids = vec![synthetic_missing_texture_fdid()];
    let material = traced_cloudsky_material(&batch, batch_index);

    assert_eq!(material.settings.has_second_texture, 1);
    assert_eq!(material.settings.has_third_texture, 0);
    assert_eq!(material.settings.has_fourth_texture, 0);
    assert_eq!(
        material.third_texture, material.base_texture,
        "missing optional stage should fall back to base binding"
    );
    assert_eq!(
        material.fourth_texture, material.base_texture,
        "missing optional stage should keep fourth-stage binding on base texture"
    );
}

#[test]
fn cloudsky_shader_8016_stage_binding_keeps_third_stage_but_disables_missing_mask_stage() {
    let model = load_cloudsky_model();
    let (batch_index, batch) = cloudsky_batch_by_shader_id(&model, SHADER_FOUR_STAGE);
    let mut batch = batch.clone();
    let third_fdid = *batch
        .extra_texture_fdids
        .first()
        .expect("cloudsky SHADER_FOUR_STAGE must keep third-stage fdid");
    batch.extra_texture_fdids = vec![third_fdid, synthetic_missing_texture_fdid()];
    let material = traced_cloudsky_material(&batch, batch_index);

    assert_eq!(material.settings.has_second_texture, 1);
    assert_eq!(material.settings.has_third_texture, 1);
    assert_eq!(material.settings.has_fourth_texture, 0);
    assert_ne!(
        material.third_texture, material.base_texture,
        "present optional third stage should not collapse to base binding"
    );
    assert_eq!(
        material.fourth_texture, material.base_texture,
        "missing optional fourth stage should fall back to base binding"
    );
    assert_eq!(
        material.settings.uv_mode_4, 1,
        "SHADER_FOUR_STAGE should keep masked UV4 routing even if mask stage is absent"
    );
}

#[test]
fn cloudsky_modern_shader_uv_selection_is_stable_against_authored_uv_flags() {
    let model = load_cloudsky_model();

    let (mod2x_index, mod2x_batch) = cloudsky_batch_by_shader_id(&model, SHADER_MOD2X);
    let mut mod2x_batch = mod2x_batch.clone();
    mod2x_batch.use_uv_2_1 = true;
    mod2x_batch.use_uv_2_2 = false;
    let mod2x_material = traced_cloudsky_material(&mod2x_batch, mod2x_index);
    assert_eq!(mod2x_material.settings.uv_mode_1, 1);
    assert_eq!(mod2x_material.settings.uv_mode_2, 0);
    assert_eq!(mod2x_material.settings.uv_mode_3, 0);
    assert_eq!(mod2x_material.settings.uv_mode_4, 0);

    let (three_stage_index, three_stage_batch) =
        cloudsky_batch_by_shader_id(&model, SHADER_THREE_STAGE);
    let mut three_stage_batch = three_stage_batch.clone();
    three_stage_batch.use_uv_2_1 = true;
    three_stage_batch.use_uv_2_2 = true;
    let three_stage_material = traced_cloudsky_material(&three_stage_batch, three_stage_index);
    assert_eq!(three_stage_material.settings.uv_mode_1, 0);
    assert_eq!(three_stage_material.settings.uv_mode_2, 0);
    assert_eq!(three_stage_material.settings.uv_mode_3, 0);
    assert_eq!(three_stage_material.settings.uv_mode_4, 0);

    let (four_stage_index, four_stage_batch) =
        cloudsky_batch_by_shader_id(&model, SHADER_FOUR_STAGE);
    let mut four_stage_batch = four_stage_batch.clone();
    four_stage_batch.use_uv_2_1 = false;
    four_stage_batch.use_uv_2_2 = false;
    let four_stage_material = traced_cloudsky_material(&four_stage_batch, four_stage_index);
    assert_eq!(four_stage_material.settings.uv_mode_1, 0);
    assert_eq!(four_stage_material.settings.uv_mode_2, 0);
    assert_eq!(four_stage_material.settings.uv_mode_3, 0);
    assert_eq!(four_stage_material.settings.uv_mode_4, 1);
}

#[test]
fn cloudsky_advanced_effect_batches_require_more_than_two_texture_stages() {
    let path = std::path::Path::new("data/models/skyboxes/11xp_cloudsky01.m2");
    let model = crate::asset::m2::load_skybox_m2_uncached(path, &[0, 0, 0])
        .expect("load 11xp cloud skybox model");
    let advanced_batch = model
        .batches
        .iter()
        .find(|batch| batch.shader_id == SHADER_THREE_STAGE)
        .expect("cloud skybox advanced-effect batch");

    assert!(
        !advanced_batch.uses_texture_combiner_combos,
        "SHADER_THREE_STAGE cloud skybox batches are direct shader effects, not combiner-table batches"
    );
    assert!(
        advanced_batch.texture_count >= 3,
        "SHADER_THREE_STAGE cloud skybox batches need at least three texture stages"
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
        .find(|batch| batch.shader_id == SHADER_FOUR_STAGE)
        .expect("cloud skybox masked crossfade batch");

    assert_eq!(masked_batch.texture_count, 4);
    assert_eq!(masked_batch.extra_texture_fdids.len(), 2);
}

#[test]
fn authored_skybox_models_reference_locally_available_textures() {
    for skybox_path in [
        std::path::Path::new("data/models/skyboxes/11xp_cloudsky01.m2"),
        std::path::Path::new("data/models/skyboxes/costalislandskybox.m2"),
        std::path::Path::new("data/models/skyboxes/deathskybox.m2"),
    ] {
        let model = crate::asset::m2::load_m2_uncached(skybox_path, &[0, 0, 0])
            .unwrap_or_else(|err| panic!("load skybox {}: {err}", skybox_path.display()));
        let mut missing = std::collections::BTreeSet::new();
        for fdid in model.batches.iter().flat_map(|batch| {
            batch
                .texture_fdid
                .into_iter()
                .chain(batch.texture_2_fdid)
                .chain(batch.extra_texture_fdids.iter().copied())
        }) {
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
        std::path::Path::new("data/models/skyboxes/costalislandskybox.m2"),
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
            let vertex_count = batch.mesh.count_vertices();
            let triangle_count = match batch.mesh.indices() {
                Some(bevy::mesh::Indices::U16(indices)) => indices.len() / 3,
                Some(bevy::mesh::Indices::U32(indices)) => indices.len() / 3,
                None => 0,
            };
            eprintln!(
                "  batch[{index}] verts={vertex_count} tris={triangle_count} tex1={:?} tex2={:?} extras={:?} texture_count={} shader_id=0x{:04x} blend_mode={}",
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
