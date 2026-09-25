use super::*;
use crate::retail_light::RetailSceneLight;
use game_engine::asset::m2_format::m2_camera::parse_camera_snapshot;

#[derive(Clone, Copy)]
pub(super) struct Framing {
    pub eye: Vec3,
    pub focus: Vec3,
    pub fov: f32,
    pub near: f32,
    pub far: f32,
}

pub(super) struct Backdrop {
    pub fdid: u32,
    pub root: Entity,
    pub framing: Framing,
}

pub(super) fn spawn(
    ctx: &mut CharCreateSpawnContext<'_, '_, '_>,
    fdid: u32,
) -> Result<Backdrop, String> {
    let path = asset::asset_cache::model(fdid)
        .ok_or_else(|| format!("creation scene {fdid}: local CASC model unavailable"))?;
    let bytes = std::fs::read(&path).map_err(|error| format!("{}: {error}", path.display()))?;
    let camera = parse_camera_snapshot(&bytes)?;
    let model = asset::m2::load_m2(&path, &[0; 3])?;
    let anchor = model
        .attachments
        .iter()
        .find(|point| point.id == 0)
        .ok_or_else(|| format!("creation scene {fdid}: attachment 0 missing"))?;
    let (transform, framing) = normalize_scene(&camera, anchor.position);
    let ambient = authored_ambient(&model);
    let root = spawn_model(ctx, &path, model, transform)?;
    ctx.commands.entity(root).insert((
        CharCreateScene,
        Name::new(format!("CharCreateBackdrop_{fdid}")),
    ));
    // Retail lights a standalone model scene with the model's own ambient.
    ctx.commands
        .insert_resource(RetailSceneLight::m2_scene(Vec3::from_array(ambient)));
    Ok(Backdrop {
        fdid,
        root,
        framing,
    })
}

/// `M2Object::getM2SceneAmbientLight`: the sum of every light's ambient term,
/// white when the model authors none.
fn authored_ambient(model: &asset::m2::M2Model) -> [f32; 3] {
    let ambient = model
        .lights
        .iter()
        .map(|light| {
            Vec3::from_array(asset::m2_light::evaluate_light_ambient(
                light,
                0,
                0,
                0,
                &model.global_sequences,
            ))
        })
        .sum::<Vec3>();
    if ambient.length() < 0.0001 {
        return [1.0; 3];
    }
    ambient.to_array()
}

fn spawn_model(
    ctx: &mut CharCreateSpawnContext<'_, '_, '_>,
    path: &std::path::Path,
    model: asset::m2::M2Model,
    transform: Transform,
) -> Result<Entity, String> {
    let mut model_ctx = m2_scene::M2SceneSpawnContext {
        commands: ctx.commands,
        assets: crate::m2_spawn::SpawnAssets {
            meshes: ctx.meshes,
            materials: ctx.materials,
            effect_materials: ctx.effect_materials,
            skybox_materials: None,
            images: ctx.images,
            inverse_bindposes: ctx.inv_bp,
        },
        creature_display_map: ctx.creature_display_map,
    };
    m2_scene::spawn_animated_static_m2_parts_from_model(
        &mut model_ctx,
        path,
        transform,
        model,
        false,
        None,
    )
    .map(|spawned| spawned.root)
    .ok_or_else(|| format!("failed to spawn creation scene {}", path.display()))
}

fn normalize_scene(
    camera: &game_engine::asset::m2_format::m2_camera::M2CameraSnapshot,
    attachment: [f32; 3],
) -> (Transform, Framing) {
    let convert = |p: [f32; 3]| Vec3::new(p[0], p[2], -p[1]);
    let anchor = convert(attachment);
    let eye = convert(camera.position);
    let focus = convert(camera.target);
    let offset = eye - focus;
    // Rigidly move the authored actor anchor to the preview origin and align
    // the camera axis with +Z; the camera-to-background shot is unchanged.
    let rotation = Quat::from_rotation_y((-offset.x).atan2(offset.z));
    let transform = Transform::from_rotation(rotation).with_translation(-(rotation * anchor));
    let framing = Framing {
        eye: transform.transform_point(eye),
        focus: transform.transform_point(focus),
        fov: camera.fov,
        near: camera.near_clip,
        far: camera.far_clip,
    };
    (transform, framing)
}
