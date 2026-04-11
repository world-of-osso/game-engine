use super::*;

pub(super) fn setup_char_select_scene(mut params: CharSelectSceneSetupParams) {
    let total_start = Instant::now();
    let selection = resolve_scene_setup_selection(&params);
    let (mut bg_node, background_elapsed) = spawn_scene_background(&mut params, &selection);
    let (lighting, camera_elapsed, sky_light_elapsed) =
        spawn_scene_camera_and_lighting(&mut params, &selection, &mut bg_node);
    let char_tf = character_transform(
        selection.scene_entry.as_ref(),
        selection.placement.as_ref(),
        Some(&params.heightmap),
        selection.presentation,
    );
    let (result, model_elapsed) = spawn_scene_model(&mut params, char_tf);
    finalize_scene_setup(&mut params, &selection, bg_node, lighting, result);
    let timings = SceneSetupTimings {
        background_elapsed,
        camera_elapsed,
        sky_light_elapsed,
        model_elapsed,
    };
    log_scene_setup_timings(total_start, timings);
}

fn resolve_scene_setup_selection(
    params: &CharSelectSceneSetupParams<'_, '_>,
) -> SceneSetupSelection {
    let scene_entry =
        background::find_scene_entry(&params.warband, &params.selected_scene).cloned();
    let placement = params
        .warband
        .as_ref()
        .zip(scene_entry.as_ref())
        .and_then(|(warband, scene)| selected_scene_placement(warband, scene));
    let presentation = selected_character_presentation(
        &params.customization_db,
        &params.char_list,
        params.selected.0,
    );
    SceneSetupSelection {
        scene_entry,
        placement,
        presentation,
    }
}

fn spawn_scene_background(
    params: &mut CharSelectSceneSetupParams<'_, '_>,
    selection: &SceneSetupSelection,
) -> (SceneNode, std::time::Duration) {
    let start = Instant::now();
    let mut background_ctx = background::WarbandBackgroundSpawnContext {
        commands: &mut params.commands,
        meshes: &mut params.assets.meshes,
        materials: &mut params.assets.materials,
        effect_materials: &mut params.assets.effect_materials,
        terrain_materials: &mut params.assets.terrain_materials,
        water_materials: &mut params.assets.water_materials,
        images: &mut params.assets.images,
        inv_bp: &mut params.assets.inv_bp,
        heightmap: &mut params.heightmap,
    };
    (
        background::spawn(
            &mut background_ctx,
            selection.scene_entry.as_ref(),
            selection
                .placement
                .as_ref()
                .map(|placement| placement.bevy_position()),
            &mut params.active_scene,
        ),
        start.elapsed(),
    )
}

fn spawn_scene_camera_and_lighting(
    params: &mut CharSelectSceneSetupParams<'_, '_>,
    selection: &SceneSetupSelection,
    bg_node: &mut SceneNode,
) -> (SceneSetupLighting, std::time::Duration, std::time::Duration) {
    let (camera_entity, camera_params, camera_elapsed) = spawn_scene_camera(params, selection);
    attach_char_select_sky_dome(params, camera_entity);
    let sky_light_elapsed =
        attach_scene_skybox_and_spawn_lighting(params, selection, bg_node, camera_params.0);
    (
        SceneSetupLighting {
            camera_entity,
            fov: camera_params.2,
            primary_light: sky_light_elapsed.0.primary_light,
            fill_light: sky_light_elapsed.0.fill_light,
        },
        camera_elapsed,
        sky_light_elapsed.1,
    )
}

fn spawn_scene_camera(
    params: &mut CharSelectSceneSetupParams<'_, '_>,
    selection: &SceneSetupSelection,
) -> (Entity, (Vec3, Vec3, f32), std::time::Duration) {
    let camera_start = Instant::now();
    let camera_entity = camera::spawn_char_select_camera(
        &mut params.commands,
        selection.scene_entry.as_ref(),
        selection.placement.as_ref(),
        Some(&params.heightmap),
        selection.presentation,
    );
    let camera_elapsed = camera_start.elapsed();
    let camera_params = camera::camera_params(
        selection.scene_entry.as_ref(),
        selection.placement.as_ref(),
        selection.presentation,
    );
    (camera_entity, camera_params, camera_elapsed)
}

fn attach_char_select_sky_dome(
    params: &mut CharSelectSceneSetupParams<'_, '_>,
    camera_entity: Entity,
) {
    let dome = spawn_char_select_sky_dome(
        &mut params.commands,
        &mut params.assets.meshes,
        &mut params.assets.sky_materials,
        &mut params.assets.images,
        params.cloud_maps.active_handle(),
        camera_entity,
    );
    params.commands.entity(dome).insert(CharSelectScene);
}

pub(super) fn spawn_char_select_sky_dome(
    commands: &mut Commands,
    meshes: &mut Assets<Mesh>,
    sky_materials: &mut Assets<crate::sky_material::SkyMaterial>,
    images: &mut Assets<Image>,
    cloud_texture: Handle<Image>,
    camera_entity: Entity,
) -> Entity {
    let dome = crate::sky::spawn_sky_dome_entity(
        commands,
        meshes,
        sky_materials,
        camera_entity,
        cloud_texture,
    );
    let colors = crate::sky_lightdata::default_sky_colors();
    let cubemap = crate::sky::build_sky_cubemap(&colors);
    let cubemap_handle = images.add(cubemap);
    commands.insert_resource(crate::sky::SkyEnvMapHandle(cubemap_handle));
    dome
}

fn attach_scene_skybox_and_spawn_lighting(
    params: &mut CharSelectSceneSetupParams<'_, '_>,
    selection: &SceneSetupSelection,
    bg_node: &mut SceneNode,
    camera_translation: Vec3,
) -> (lighting::CharSelectLightingEntities, std::time::Duration) {
    let sky_light_start = Instant::now();
    let skybox_translation = selection
        .placement
        .as_ref()
        .map(|placement| placement.bevy_position())
        .unwrap_or(camera_translation);
    attach_scene_skybox(
        params,
        selection.scene_entry.as_ref(),
        skybox_translation,
        bg_node,
    );
    let dir = lighting::spawn(
        &mut params.commands,
        selection.scene_entry.as_ref(),
        selection.placement.as_ref(),
        selection.presentation,
    );
    (dir, sky_light_start.elapsed())
}

fn attach_scene_skybox(
    params: &mut CharSelectSceneSetupParams<'_, '_>,
    scene_entry: Option<&WarbandSceneEntry>,
    skybox_translation: Vec3,
    bg_node: &mut SceneNode,
) {
    if !should_spawn_authored_char_select_skybox() {
        return;
    }
    let skybox_entity = {
        let mut skybox_ctx = background::WarbandSkyboxSpawnContext {
            commands: &mut params.commands,
            meshes: &mut params.assets.meshes,
            materials: &mut params.assets.materials,
            effect_materials: &mut params.assets.effect_materials,
            skybox_materials: &mut params.assets.skybox_materials,
            images: &mut params.assets.images,
            inv_bp: &mut params.assets.inv_bp,
            creature_display_map: &params.creature_display_map,
        };
        background::spawn_skybox(&mut skybox_ctx, scene_entry, skybox_translation)
    };
    if let Some((entity, path)) = skybox_entity
        .zip(scene_entry.and_then(crate::scenes::char_select::warband::ensure_warband_skybox))
    {
        bg_node.children.push(scene_tree::skybox_scene_node(
            entity,
            path.display().to_string(),
        ));
    }
}

pub(super) fn should_spawn_authored_char_select_skybox() -> bool {
    true
}

fn spawn_scene_model(
    params: &mut CharSelectSceneSetupParams<'_, '_>,
    char_tf: Transform,
) -> (Option<(u64, Entity)>, std::time::Duration) {
    let start = Instant::now();
    let mut spawn_ctx = CharSelectModelSpawnContext {
        commands: &mut params.commands,
        assets: &mut params.assets,
        creature_display_map: &params.creature_display_map,
    };
    (
        spawn_selected_model(
            &mut spawn_ctx,
            &params.char_list,
            params.selected.0,
            char_tf,
        ),
        start.elapsed(),
    )
}

fn finalize_scene_setup(
    params: &mut CharSelectSceneSetupParams<'_, '_>,
    selection: &SceneSetupSelection,
    bg_node: SceneNode,
    lighting: SceneSetupLighting,
    result: Option<(u64, Entity)>,
) {
    params.displayed.0 = result.as_ref().map(|(id, _)| *id);
    let children = build_scene_setup_children(
        params,
        bg_node,
        &lighting,
        result.as_ref().map(|(_, entity)| *entity),
    );
    params
        .commands
        .insert_resource(scene_tree::build_scene_tree(children));
    params.pending_supplemental.scene_id = selection
        .scene_entry
        .as_ref()
        .filter(|scene| {
            !crate::scenes::char_select::warband::supplemental_terrain_tile_coords(scene).is_empty()
        })
        .map(|scene| scene.id);
    params.pending_supplemental.wait_for_next_frame =
        params.pending_supplemental.scene_id.is_some();
}

fn build_scene_setup_children(
    params: &CharSelectSceneSetupParams<'_, '_>,
    bg_node: SceneNode,
    lighting: &SceneSetupLighting,
    model_entity: Option<Entity>,
) -> Vec<SceneNode> {
    let mut children = vec![bg_node];
    if let Some(entity) = model_entity {
        let (race, gender, model) =
            scene_systems::char_info_strings(&params.char_list, params.selected.0);
        let (name, character_id) =
            selected_scene_character_identity(&params.char_list, params.selected.0);
        children.push(scene_tree::character_scene_node(
            entity,
            model,
            race,
            gender,
            name,
            character_id,
        ));
    }
    children.extend(scene_tree::light_scene_nodes(
        lighting.camera_entity,
        lighting.fov,
        None,
        lighting::CHAR_SELECT_AMBIENT_BRIGHTNESS,
        lighting.primary_light,
        Some(lighting.fill_light),
    ));
    children
}

fn log_scene_setup_timings(total_start: Instant, timings: SceneSetupTimings) {
    info!(
        "setup_char_select_scene finished in {:.3}s (background={:.3}s camera={:.3}s sky+light={:.3}s model={:.3}s)",
        total_start.elapsed().as_secs_f32(),
        timings.background_elapsed.as_secs_f32(),
        timings.camera_elapsed.as_secs_f32(),
        timings.sky_light_elapsed.as_secs_f32(),
        timings.model_elapsed.as_secs_f32(),
    );
}
