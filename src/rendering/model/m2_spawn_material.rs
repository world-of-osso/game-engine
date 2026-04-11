use std::path::{Path, PathBuf};
use std::sync::{Mutex, OnceLock};

use bevy::asset::AssetId;
use bevy::prelude::*;

use crate::asset;
use crate::m2_effect_material::{self, M2EffectMaterial, M2EffectSettings};
use crate::skybox_m2_material::{SkyboxM2Material, SkyboxM2Settings};

use super::{BatchMaterial, PLACEHOLDER_COLORS};

static REPEAT_TEXTURE_CACHE: OnceLock<Mutex<std::collections::HashMap<u32, AssetId<Image>>>> =
    OnceLock::new();

pub(super) fn load_batch_material(
    batch: &asset::m2::M2RenderBatch,
    index: usize,
    images: &mut Assets<Image>,
    materials: &mut Assets<StandardMaterial>,
    effect_materials: &mut Assets<M2EffectMaterial>,
    skybox_materials: Option<&mut Assets<SkyboxM2Material>>,
    force_skybox_material: bool,
    skybox_color: Option<Color>,
) -> BatchMaterial {
    let texture_dir = PathBuf::from("data/textures");
    if force_skybox_material {
        if let Some(materials) = skybox_materials {
            if let Some(mat) =
                try_load_skybox_material(batch, &texture_dir, images, materials, skybox_color)
            {
                return BatchMaterial::Skybox(mat);
            }
            return BatchMaterial::Skybox(materials.add(skybox_m2_material(
                None,
                None,
                None,
                None,
                Some(PLACEHOLDER_COLORS[index % PLACEHOLDER_COLORS.len()]),
                batch,
            )));
        }
    }
    if should_use_effect_material(batch)
        && let Some(mat) = try_load_effect_material(batch, &texture_dir, images, effect_materials)
    {
        return BatchMaterial::Effect(mat);
    }
    if let Some(fdid) = batch.texture_fdid {
        let blp_path = asset::asset_cache::texture(fdid)
            .unwrap_or_else(|| texture_dir.join(format!("{fdid}.blp")));
        if let Some(mat) =
            try_load_textured_material(&blp_path, batch, &texture_dir, images, materials)
        {
            return BatchMaterial::Standard(mat);
        }
    }
    let color = PLACEHOLDER_COLORS[index % PLACEHOLDER_COLORS.len()];
    BatchMaterial::Standard(materials.add(m2_material(None, Some(color), batch)))
}

fn should_use_effect_material(batch: &asset::m2::M2RenderBatch) -> bool {
    batch.texture_2_fdid.is_some() && batch.blend_mode >= 2 && batch.overlays.is_empty()
}

fn try_load_textured_material(
    blp_path: &Path,
    batch: &asset::m2::M2RenderBatch,
    texture_dir: &Path,
    images: &mut Assets<Image>,
    materials: &mut Assets<StandardMaterial>,
) -> Option<Handle<StandardMaterial>> {
    if !blp_path.exists() {
        return None;
    }
    let image =
        crate::m2_texture_composite::load_composited_texture(blp_path, batch, texture_dir, images)
            .map_err(|e| eprintln!("{e}"))
            .ok()?;
    Some(materials.add(m2_material(Some(image), None, batch)))
}

fn try_load_effect_material(
    batch: &asset::m2::M2RenderBatch,
    texture_dir: &Path,
    images: &mut Assets<Image>,
    materials: &mut Assets<M2EffectMaterial>,
) -> Option<Handle<M2EffectMaterial>> {
    let base_fdid = batch.texture_fdid?;
    let second_fdid = batch.texture_2_fdid?;
    let base_texture = load_repeat_texture(base_fdid, texture_dir, images)?;
    let second_texture = load_repeat_texture(second_fdid, texture_dir, images)?;
    let alpha_test = match batch.blend_mode {
        1 => 224.0 / 255.0 * batch.transparency,
        2..=7 => (1.0 / 255.0) * batch.transparency,
        _ => 0.0,
    };
    Some(materials.add(M2EffectMaterial {
        settings: M2EffectSettings {
            transparency: batch.transparency,
            alpha_test,
            shader_id: batch.shader_id as u32,
            blend_mode: batch.blend_mode as u32,
            uv_mode_1: u32::from(batch.use_uv_2_1),
            uv_mode_2: u32::from(batch.use_uv_2_2),
            render_flags: batch.render_flags as u32,
            uv_offset_1: Vec2::ZERO,
            uv_offset_2: Vec2::ZERO,
        },
        base_texture,
        second_texture,
        blend_mode: batch.blend_mode,
        two_sided: batch.render_flags & 0x04 != 0,
        texture_anim_1: batch.texture_anim.clone(),
        texture_anim_2: batch.texture_anim_2.clone(),
    }))
}

fn try_load_skybox_material(
    batch: &asset::m2::M2RenderBatch,
    texture_dir: &Path,
    images: &mut Assets<Image>,
    materials: &mut Assets<SkyboxM2Material>,
    color: Option<Color>,
) -> Option<Handle<SkyboxM2Material>> {
    let fdid = batch.texture_fdid?;
    let advanced_batch = skybox_batch_needs_effect_combine(batch);
    let base_texture = if advanced_batch {
        load_repeat_texture(fdid, texture_dir, images)?
    } else {
        let blp_path = asset::asset_cache::texture(fdid)
            .unwrap_or_else(|| texture_dir.join(format!("{fdid}.blp")));
        if !blp_path.exists() {
            return None;
        }
        crate::m2_texture_composite::load_composited_texture(&blp_path, batch, texture_dir, images)
            .map_err(|e| eprintln!("{e}"))
            .ok()?
    };
    let second_texture = if advanced_batch {
        batch
            .texture_2_fdid
            .and_then(|second_fdid| load_repeat_texture(second_fdid, texture_dir, images))
    } else {
        None
    };
    let third_texture = if advanced_batch {
        batch
            .extra_texture_fdids
            .first()
            .and_then(|fdid| load_repeat_texture(*fdid, texture_dir, images))
    } else {
        None
    };
    let fourth_texture = if advanced_batch {
        batch
            .extra_texture_fdids
            .get(1)
            .and_then(|fdid| load_repeat_texture(*fdid, texture_dir, images))
    } else {
        None
    };
    Some(materials.add(skybox_m2_material(
        Some(base_texture),
        second_texture,
        third_texture,
        fourth_texture,
        color,
        batch,
    )))
}

fn skybox_shader_supports_runtime_combine(shader_id: u16) -> bool {
    matches!(
        shader_id,
        0x0010 | 0x0011 | 0x4014 | 0x4016 | 0x8001 | 0x8002 | 0x8003 | 0x8012 | 0x8015 | 0x8016
    )
}

const SKYBOX_SINGLE_TEXTURE_FRAGMENT_MODES: [u16; 6] = [0x1, 0x2, 0x3, 0x4, 0x5, 0x6];
const SKYBOX_TWO_TEXTURE_FRAGMENT_MODES: [[u16; 6]; 6] = [
    [0x7, 0x8, 0x9, 0xA, 0xB, 0xC],
    [0xD, 0xE, 0xF, 0x10, 0x11, 0x12],
    [0x13, 0x14, 0x15, 0x16, 0x17, 0x18],
    [0x19, 0x1A, 0x1B, 0x1C, 0x1D, 0x1E],
    [0x1F, 0x20, 0x21, 0x22, 0x23, 0x24],
    [0x1F, 0x20, 0x21, 0x22, 0x23, 0x24],
];
const SKYBOX_STATIC_FRAGMENT_MODES: [usize; 7] = [0, 1, 1, 1, 1, 5, 5];

fn static_fragment_mode_for_blend(blend_mode: u16) -> usize {
    SKYBOX_STATIC_FRAGMENT_MODES
        .get(blend_mode as usize)
        .copied()
        .unwrap_or(1)
}

fn resolve_skybox_combine_mode(batch: &asset::m2::M2RenderBatch) -> u32 {
    if batch.uses_texture_combiner_combos || (batch.shader_id & 0x8000) != 0 {
        return batch.shader_id as u32;
    }
    let static_mode = static_fragment_mode_for_blend(batch.blend_mode);
    if batch.texture_count > 1 {
        SKYBOX_TWO_TEXTURE_FRAGMENT_MODES[static_mode][static_mode] as u32
    } else {
        SKYBOX_SINGLE_TEXTURE_FRAGMENT_MODES[static_mode] as u32
    }
}

fn resolve_skybox_uv_modes(batch: &asset::m2::M2RenderBatch) -> [u32; 4] {
    match batch.shader_id {
        0x8012 => [0, 0, 0, 0],
        0x8016 => [0, 0, 0, 1],
        _ => [
            u32::from(batch.use_uv_2_1),
            u32::from(batch.use_uv_2_2),
            0,
            0,
        ],
    }
}

fn skybox_batch_needs_effect_combine(batch: &asset::m2::M2RenderBatch) -> bool {
    if batch.texture_count > 1 {
        return true;
    }
    if !skybox_shader_supports_runtime_combine(batch.shader_id) {
        return false;
    }
    let uses_multitexture_shader = batch.texture_count > 1 && batch.shader_id != 0;
    batch.texture_2_fdid.is_some()
        || batch.texture_count > 1
        || batch.texture_anim.is_some()
        || batch.texture_anim_2.is_some()
        || batch.use_uv_2_1
        || batch.use_uv_2_2
        || batch.use_env_map_2
        || uses_multitexture_shader
}

fn load_repeat_texture(
    fdid: u32,
    texture_dir: &Path,
    images: &mut Assets<Image>,
) -> Option<Handle<Image>> {
    let cache = REPEAT_TEXTURE_CACHE.get_or_init(|| Mutex::new(std::collections::HashMap::new()));
    if let Some(handle) = crate::asset_lifetime::lookup_cached_asset_handle(cache, &fdid, images) {
        return Some(handle);
    }
    let blp_path = asset::asset_cache::texture(fdid)
        .unwrap_or_else(|| texture_dir.join(format!("{fdid}.blp")));
    if !blp_path.exists() {
        return None;
    }
    let (pixels, width, height) = asset::blp::load_blp_rgba(&blp_path).ok()?;
    let mut image = crate::rgba_image(pixels, width, height);
    image.sampler = m2_effect_material::repeat_sampler();
    let handle = images.add(image);
    crate::asset_lifetime::prune_unused_asset_handles(cache, images);
    cache.lock().unwrap().insert(fdid, handle.id());
    Some(handle)
}

/// Build a StandardMaterial from M2 render flags (two-sided, unlit, blend mode).
pub(super) fn m2_material(
    texture: Option<Handle<Image>>,
    color: Option<Color>,
    batch: &asset::m2::M2RenderBatch,
) -> StandardMaterial {
    let two_sided = batch.render_flags & 0x04 != 0;
    let unlit = batch.render_flags & 0x01 != 0;
    let cull_mode = if two_sided {
        None
    } else {
        Some(bevy::render::render_resource::Face::Back)
    };
    let alpha_mode = m2_effect_material::alpha_mode_for_blend(batch.blend_mode);
    StandardMaterial {
        base_color_texture: texture,
        base_color: color.unwrap_or(Color::srgba(1.0, 1.0, 1.0, batch.transparency)),
        unlit,
        cull_mode,
        double_sided: two_sided,
        alpha_mode,
        ..default()
    }
}

pub(crate) fn skybox_m2_material(
    texture: Option<Handle<Image>>,
    second_texture: Option<Handle<Image>>,
    third_texture: Option<Handle<Image>>,
    fourth_texture: Option<Handle<Image>>,
    color: Option<Color>,
    batch: &asset::m2::M2RenderBatch,
) -> SkyboxM2Material {
    let has_second_texture = second_texture.is_some();
    let has_third_texture = third_texture.is_some();
    let has_fourth_texture = fourth_texture.is_some();
    let uv_modes = resolve_skybox_uv_modes(batch);
    let base_texture = texture.unwrap_or_default();
    let second_texture = second_texture.unwrap_or_else(|| base_texture.clone());
    let third_texture = third_texture.unwrap_or_else(|| base_texture.clone());
    let fourth_texture = fourth_texture.unwrap_or_else(|| base_texture.clone());
    SkyboxM2Material {
        settings: SkyboxM2Settings {
            color: color
                .unwrap_or(Color::WHITE)
                .to_linear()
                .to_f32_array()
                .into(),
            transparency: 1.0,
            // Authored skybox textures use low alpha values for soft cloud edges.
            // Applying the normal M2 alpha-test thresholds discards the entire dome.
            alpha_test: 0.0,
            combine_mode: resolve_skybox_combine_mode(batch),
            blend_mode: batch.blend_mode as u32,
            uv_mode_1: uv_modes[0],
            uv_mode_2: uv_modes[1],
            uv_mode_3: uv_modes[2],
            uv_mode_4: uv_modes[3],
            render_flags: batch.render_flags as u32,
            has_second_texture: u32::from(has_second_texture),
            has_third_texture: u32::from(has_third_texture),
            has_fourth_texture: u32::from(has_fourth_texture),
            uv_offset_1: Vec2::ZERO,
            uv_offset_2: Vec2::ZERO,
        },
        base_texture,
        second_texture,
        third_texture,
        fourth_texture,
        blend_mode: batch.blend_mode,
        texture_anim_1: batch.texture_anim.clone(),
        texture_anim_2: batch.texture_anim_2.clone(),
    }
}

#[cfg(test)]
mod tests {
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
                batch.texture_count == 1
                    && batch.texture_2_fdid.is_none()
                    && batch.shader_id == 0x0010
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
                    && batch.shader_id == 0x0010
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
            let mut effect_materials =
                Assets::<crate::m2_effect_material::M2EffectMaterial>::default();
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
            .find(|batch| {
                batch.texture_2_fdid.is_some() && matches!(batch.shader_id, 0x8012 | 0x8016)
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
                let local = crate::asset::asset_cache::texture(fdid).unwrap_or_else(|| {
                    std::path::PathBuf::from(format!("data/textures/{fdid}.blp"))
                });
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
}
