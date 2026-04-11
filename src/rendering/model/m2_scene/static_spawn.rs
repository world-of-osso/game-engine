use std::path::Path;

use bevy::prelude::*;

use super::{
    M2SceneSpawnContext, attach_m2_model_parts, load_m2_model, load_m2_model_with_skin_fdids,
};

/// Spawn a static M2 model that still carries animation data.
pub struct SpawnedAnimatedStaticM2 {
    pub root: Entity,
    pub model_root: Entity,
}

/// Spawn a static M2 model that still carries animation data.
pub fn spawn_animated_static_m2_parts(
    ctx: &mut M2SceneSpawnContext<'_, '_, '_>,
    m2_path: &Path,
    transform: Transform,
) -> Option<SpawnedAnimatedStaticM2> {
    let Some(model) = load_m2_model(m2_path, ctx.creature_display_map) else {
        return None;
    };
    spawn_animated_static_m2_parts_from_model(ctx, m2_path, transform, model, false, None)
}

pub fn spawn_animated_static_skybox_m2_parts(
    ctx: &mut M2SceneSpawnContext<'_, '_, '_>,
    m2_path: &Path,
    transform: Transform,
    skybox_color: Option<Color>,
) -> Option<SpawnedAnimatedStaticM2> {
    let Some(model) = super::load_skybox_m2_model(m2_path, ctx.creature_display_map) else {
        return None;
    };
    spawn_animated_static_m2_parts_from_model(ctx, m2_path, transform, model, true, skybox_color)
}

pub fn spawn_animated_static_m2_parts_with_skin_fdids(
    ctx: &mut M2SceneSpawnContext<'_, '_, '_>,
    m2_path: &Path,
    transform: Transform,
    skin_fdids: &[u32; 3],
) -> Option<SpawnedAnimatedStaticM2> {
    let Some(model) = load_m2_model_with_skin_fdids(m2_path, skin_fdids) else {
        return None;
    };
    spawn_animated_static_m2_parts_from_model(ctx, m2_path, transform, model, false, None)
}

pub fn spawn_animated_static_m2_parts_from_model(
    ctx: &mut M2SceneSpawnContext<'_, '_, '_>,
    m2_path: &Path,
    transform: Transform,
    model: crate::asset::m2::M2Model,
    force_skybox_material: bool,
    skybox_color: Option<Color>,
) -> Option<SpawnedAnimatedStaticM2> {
    let spawned = spawn_static_model_entities(ctx, m2_path, transform);
    sync_static_skybox_materials(ctx, force_skybox_material);
    attach_m2_model_parts(
        ctx,
        model,
        spawned.model_root,
        static_attach_options(force_skybox_material, skybox_color),
    );
    Some(spawned)
}

fn spawn_static_model_entities(
    ctx: &mut M2SceneSpawnContext<'_, '_, '_>,
    m2_path: &Path,
    transform: Transform,
) -> SpawnedAnimatedStaticM2 {
    let name = m2_path
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or("prop");
    let root = ctx
        .commands
        .spawn((Name::new(name.to_owned()), transform, Visibility::default()))
        .id();
    let model_root = ctx
        .commands
        .spawn((
            Name::new(format!("{name}ModelRoot")),
            Transform::IDENTITY,
            Visibility::default(),
        ))
        .id();
    ctx.commands.entity(model_root).insert(ChildOf(root));
    SpawnedAnimatedStaticM2 { root, model_root }
}

fn sync_static_skybox_materials(
    ctx: &mut M2SceneSpawnContext<'_, '_, '_>,
    force_skybox_material: bool,
) {
    let skybox_materials = if force_skybox_material {
        ctx.assets.skybox_materials.take()
    } else {
        None
    };
    ctx.assets.skybox_materials = skybox_materials;
}

fn static_attach_options(
    force_skybox_material: bool,
    skybox_color: Option<Color>,
) -> super::M2SceneAttachOptions {
    super::M2SceneAttachOptions {
        default_main_hand_torch: false,
        force_skybox_material,
        skybox_color,
    }
}

/// Spawn a static M2 model that still carries animation data.
pub fn spawn_animated_static_m2(
    ctx: &mut M2SceneSpawnContext<'_, '_, '_>,
    m2_path: &Path,
    transform: Transform,
) -> Option<Entity> {
    spawn_animated_static_m2_parts(ctx, m2_path, transform).map(|spawned| spawned.root)
}
