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
    let second_texture = batch
        .texture_2_fdid
        .and_then(|second_fdid| load_repeat_texture(second_fdid, texture_dir, images));
    Some(materials.add(skybox_m2_material(
        Some(base_texture),
        second_texture,
        color,
        batch,
    )))
}

fn skybox_batch_needs_effect_combine(batch: &asset::m2::M2RenderBatch) -> bool {
    batch.texture_2_fdid.is_some()
        || batch.texture_count > 1
        || batch.texture_anim.is_some()
        || batch.texture_anim_2.is_some()
        || batch.use_uv_2_1
        || batch.use_uv_2_2
        || batch.use_env_map_2
        || batch.shader_id != 0
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
    color: Option<Color>,
    batch: &asset::m2::M2RenderBatch,
) -> SkyboxM2Material {
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
            shader_id: batch.shader_id as u32,
            blend_mode: batch.blend_mode as u32,
            uv_mode_1: u32::from(batch.use_uv_2_1),
            uv_mode_2: u32::from(batch.use_uv_2_2),
            render_flags: batch.render_flags as u32,
            uv_offset_1: Vec2::ZERO,
            uv_offset_2: Vec2::ZERO,
        },
        base_texture: texture.unwrap_or_default(),
        second_texture: second_texture.unwrap_or_default(),
        blend_mode: batch.blend_mode,
        texture_anim_1: batch.texture_anim.clone(),
        texture_anim_2: batch.texture_anim_2.clone(),
    }
}

#[cfg(test)]
mod tests {
    use super::load_batch_material;
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
                    "  batch[{index}] tex1={:?} tex2={:?} shader_id=0x{:04x} blend_mode={}",
                    batch.texture_fdid, batch.texture_2_fdid, batch.shader_id, batch.blend_mode
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
