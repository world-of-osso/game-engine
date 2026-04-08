use super::terrain_objects_wmo_surface::wmo_material_props;
use super::*;

pub(super) fn spawn_wmo_group_batches(
    commands: &mut Commands,
    assets: &mut WmoAssets<'_>,
    root: &wmo::WmoRootData,
    interior_ambient: Option<[f32; 4]>,
    group_entity: Entity,
    batches: Vec<wmo::WmoGroupBatch>,
) {
    for batch in batches {
        let material_props = wmo_material_props(root, batch.material_index);
        let mat = wmo_batch_material(
            assets.materials,
            assets.images,
            batch.material_index,
            &material_props,
            interior_ambient,
            batch.has_vertex_color,
        );
        let mut child = commands.spawn((
            Mesh3d(assets.meshes.add(batch.mesh)),
            MeshMaterial3d(mat),
            Transform::default(),
            Visibility::default(),
            WmoCollisionMesh,
        ));
        if let Some(glow) = material_props.sidn_glow {
            child.insert(glow);
        }
        let child = child.id();
        commands.entity(group_entity).add_child(child);
    }
}

pub(super) fn spawn_wmo_group_lights(
    commands: &mut Commands,
    root: &wmo::WmoRootData,
    group: &wmo::WmoGroupData,
    group_entity: Entity,
) {
    for (light_index, light) in collect_group_lights(root, group) {
        let Some(light_entity) = spawn_wmo_group_light(commands, light_index, light) else {
            continue;
        };
        commands.entity(group_entity).add_child(light_entity);
    }
}

pub(super) fn spawn_wmo_group_fogs(
    commands: &mut Commands,
    root: &wmo::WmoRootData,
    group: &wmo::WmoGroupData,
    group_entity: Entity,
) {
    for (fog_index, fog) in collect_group_fogs(root, group) {
        let fog_entity = spawn_wmo_group_fog(commands, fog_index, fog);
        commands.entity(group_entity).add_child(fog_entity);
    }
}

pub(super) fn spawn_wmo_group_liquid(
    commands: &mut Commands,
    assets: &mut WmoAssets<'_>,
    group: &wmo::WmoGroupData,
    group_entity: Entity,
) {
    let Some(liquid) = group.liquid.as_ref() else {
        return;
    };
    let mesh = build_wmo_liquid_mesh(liquid);
    let Some(material) = build_wmo_liquid_material(assets.water_materials, assets.images) else {
        return;
    };
    let liquid_entity = commands
        .spawn((
            Name::new("wmo_liquid"),
            Mesh3d(assets.meshes.add(mesh)),
            MeshMaterial3d(material),
            Transform::default(),
            Visibility::default(),
        ))
        .id();
    commands.entity(group_entity).add_child(liquid_entity);
}

pub(super) fn build_wmo_liquid_material(
    water_materials: &mut Assets<WaterMaterial>,
    images: &mut Assets<Image>,
) -> Option<Handle<WaterMaterial>> {
    let normal_map = images.add(water_material::generate_water_normal_map());
    Some(water_materials.add(WaterMaterial {
        settings: WaterSettings::default(),
        normal_map,
    }))
}

pub(super) const WMO_LIQUID_TILE_SIZE: f32 = 4.166_662_5;
pub(super) const WMO_LIQUID_Z_OFFSET: f32 = -1.0;

pub(super) fn build_wmo_liquid_mesh(liquid: &wmo::WmoLiquid) -> Mesh {
    let (positions, normals, uvs, colors, indices) = build_wmo_liquid_geometry(liquid);
    let mut mesh = Mesh::new(
        PrimitiveTopology::TriangleList,
        RenderAssetUsages::default(),
    );
    mesh.insert_attribute(Mesh::ATTRIBUTE_POSITION, positions);
    mesh.insert_attribute(Mesh::ATTRIBUTE_NORMAL, normals);
    mesh.insert_attribute(Mesh::ATTRIBUTE_UV_0, uvs);
    mesh.insert_attribute(Mesh::ATTRIBUTE_COLOR, colors);
    mesh.insert_indices(Indices::U32(indices));
    mesh
}

pub(super) type WmoLiquidGeometry = (
    Vec<[f32; 3]>,
    Vec<[f32; 3]>,
    Vec<[f32; 2]>,
    Vec<[f32; 4]>,
    Vec<u32>,
);

pub(super) fn build_wmo_liquid_geometry(liquid: &wmo::WmoLiquid) -> WmoLiquidGeometry {
    let width = liquid.header.x_tiles.max(0) as usize;
    let height = liquid.header.y_tiles.max(0) as usize;
    let mut positions = Vec::with_capacity(width * height * 4);
    let mut normals = Vec::with_capacity(width * height * 4);
    let mut uvs = Vec::with_capacity(width * height * 4);
    let mut colors = Vec::with_capacity(width * height * 4);
    let mut indices = Vec::with_capacity(width * height * 6);

    for row in 0..height {
        for col in 0..width {
            if !wmo_liquid_tile_exists(liquid, row, col) {
                continue;
            }
            let base = positions.len() as u32;
            for (dr, dc) in [(0usize, 0usize), (0, 1), (1, 0), (1, 1)] {
                let local = wmo_liquid_local_pos(liquid, row + dr, col + dc);
                positions.push(crate::asset::wmo::wmo_local_to_bevy(
                    local[0], local[1], local[2],
                ));
                normals.push([0.0, 1.0, 0.0]);
                uvs.push([
                    (col + dc) as f32 / width.max(1) as f32,
                    (row + dr) as f32 / height.max(1) as f32,
                ]);
                colors.push([1.0, 1.0, 1.0, 1.0]);
            }
            indices.extend_from_slice(&[base, base + 1, base + 2, base + 2, base + 1, base + 3]);
        }
    }

    (positions, normals, uvs, colors, indices)
}

pub(super) fn wmo_liquid_tile_exists(liquid: &wmo::WmoLiquid, row: usize, col: usize) -> bool {
    let width = liquid.header.x_tiles.max(0) as usize;
    let tile_index = row.saturating_mul(width).saturating_add(col);
    liquid
        .tiles
        .get(tile_index)
        .is_none_or(|tile| tile.liquid_type != 0x0F)
}

pub(super) fn wmo_liquid_local_pos(liquid: &wmo::WmoLiquid, row: usize, col: usize) -> [f32; 3] {
    let base = liquid.header.position;
    let x = base[0] + col as f32 * WMO_LIQUID_TILE_SIZE;
    let y = base[1] + row as f32 * WMO_LIQUID_TILE_SIZE;
    let z = wmo_liquid_height(liquid, row, col) + WMO_LIQUID_Z_OFFSET;
    [x, y, z]
}

pub(super) fn wmo_liquid_height(liquid: &wmo::WmoLiquid, row: usize, col: usize) -> f32 {
    let width = liquid.header.x_verts.max(0) as usize;
    let vertex_index = row.saturating_mul(width).saturating_add(col);
    liquid
        .vertices
        .get(vertex_index)
        .map(|vertex| vertex.height)
        .unwrap_or(liquid.header.position[2])
}

pub(super) fn collect_group_lights<'a>(
    root: &'a wmo::WmoRootData,
    group: &wmo::WmoGroupData,
) -> Vec<(u16, &'a wmo::WmoLight)> {
    group
        .light_refs
        .iter()
        .filter_map(|&light_index| {
            root.lights
                .get(light_index as usize)
                .map(|light| (light_index, light))
        })
        .collect()
}

pub(super) fn collect_group_fogs<'a>(
    root: &'a wmo::WmoRootData,
    group: &wmo::WmoGroupData,
) -> Vec<(u8, &'a wmo::WmoFog)> {
    let mut fogs = Vec::new();
    let mut seen = std::collections::HashSet::new();
    for fog_index in group.header.fog_ids {
        if !seen.insert(fog_index) {
            continue;
        }
        let Some(fog) = root.fogs.get(fog_index as usize) else {
            continue;
        };
        fogs.push((fog_index, fog));
    }
    fogs
}

pub(super) fn spawn_wmo_group_light(
    commands: &mut Commands,
    light_index: u16,
    light: &wmo::WmoLight,
) -> Option<Entity> {
    match light.light_type {
        wmo::WmoLightType::Omni => Some(spawn_wmo_point_light(commands, light_index, light)),
        wmo::WmoLightType::Spot => Some(spawn_wmo_spot_light(commands, light_index, light)),
        wmo::WmoLightType::Directional | wmo::WmoLightType::Ambient => None,
    }
}

pub(super) fn spawn_wmo_group_fog(
    commands: &mut Commands,
    fog_index: u8,
    fog: &wmo::WmoFog,
) -> Entity {
    commands
        .spawn((
            Name::new(format!("WmoFog{fog_index}")),
            wmo_fog_transform(fog),
            WmoGroupFogVolume {
                fog_index,
                smaller_radius: fog.smaller_radius,
                larger_radius: fog.larger_radius,
                fog_end: fog.fog_end,
                fog_start_multiplier: fog.fog_start_multiplier,
                color_1: fog.color_1,
                underwater_fog_end: fog.underwater_fog_end,
                underwater_fog_start_multiplier: fog.underwater_fog_start_multiplier,
                color_2: fog.color_2,
            },
            Visibility::default(),
        ))
        .id()
}

pub(super) fn spawn_wmo_point_light(
    commands: &mut Commands,
    light_index: u16,
    light: &wmo::WmoLight,
) -> Entity {
    commands
        .spawn((
            Name::new(format!("WmoLight{light_index}")),
            wmo_light_transform(light),
            authored_wmo_point_light(light),
            Visibility::default(),
        ))
        .id()
}

pub(super) fn spawn_wmo_spot_light(
    commands: &mut Commands,
    light_index: u16,
    light: &wmo::WmoLight,
) -> Entity {
    commands
        .spawn((
            Name::new(format!("WmoSpotLight{light_index}")),
            wmo_light_transform(light),
            authored_wmo_spot_light(light),
            Visibility::default(),
        ))
        .id()
}

pub(super) fn wmo_light_transform(light: &wmo::WmoLight) -> Transform {
    let [x, y, z] = crate::asset::wmo::wmo_local_to_bevy(
        light.position[0],
        light.position[1],
        light.position[2],
    );
    Transform::from_translation(Vec3::new(x, y, z)).with_rotation(wow_quat_to_bevy(light.rotation))
}

pub(super) fn wmo_fog_transform(fog: &wmo::WmoFog) -> Transform {
    let [x, y, z] =
        crate::asset::wmo::wmo_local_to_bevy(fog.position[0], fog.position[1], fog.position[2]);
    Transform::from_translation(Vec3::new(x, y, z)).with_scale(Vec3::splat(
        fog.larger_radius.max(fog.smaller_radius).max(1.0),
    ))
}

pub(super) fn authored_wmo_point_light(light: &wmo::WmoLight) -> PointLight {
    PointLight {
        color: Color::linear_rgb(light.color[0], light.color[1], light.color[2]),
        intensity: wmo_light_intensity(light),
        range: wmo_light_range(light),
        radius: light.attenuation_start.min(light.attenuation_end),
        shadows_enabled: false,
        ..default()
    }
}

pub(super) fn authored_wmo_spot_light(light: &wmo::WmoLight) -> SpotLight {
    SpotLight {
        color: Color::linear_rgb(light.color[0], light.color[1], light.color[2]),
        intensity: wmo_light_intensity(light),
        range: wmo_light_range(light),
        radius: light.attenuation_start.min(light.attenuation_end),
        inner_angle: std::f32::consts::FRAC_PI_6,
        outer_angle: std::f32::consts::FRAC_PI_3,
        shadows_enabled: false,
        ..default()
    }
}

pub(super) fn wmo_light_intensity(light: &wmo::WmoLight) -> f32 {
    light.intensity.max(0.0)
}

pub(super) fn wmo_light_range(light: &wmo::WmoLight) -> f32 {
    if light.use_attenuation {
        light.attenuation_end.max(light.attenuation_start)
    } else {
        light.attenuation_end.max(1.0)
    }
}

pub(super) fn spawn_wmo_group_doodads(
    commands: &mut Commands,
    assets: &mut WmoAssets<'_>,
    root: &wmo::WmoRootData,
    group: &wmo::WmoGroupData,
    group_entity: Entity,
    active_doodad_set: u16,
) {
    for doodad in collect_group_doodads(root, group, active_doodad_set) {
        let Some(entity) = spawn_wmo_group_doodad(commands, assets, &doodad) else {
            continue;
        };
        commands.entity(group_entity).add_child(entity);
    }
}

#[derive(Clone, Debug, PartialEq)]
pub(super) struct WmoGroupDoodad {
    pub(super) model_fdid: u32,
    pub(super) transform: Transform,
}

pub(super) fn collect_group_doodads(
    root: &wmo::WmoRootData,
    group: &wmo::WmoGroupData,
    active_doodad_set: u16,
) -> Vec<WmoGroupDoodad> {
    let active_indices = active_wmo_doodad_indices(root, active_doodad_set);
    group
        .doodad_refs
        .iter()
        .filter_map(|&doodad_index| {
            if !active_indices.contains(&doodad_index) {
                return None;
            }
            let doodad_def = root.doodad_defs.get(doodad_index as usize)?;
            let model_fdid = resolve_wmo_doodad_fdid(root, doodad_def.name_offset)?;
            Some(WmoGroupDoodad {
                model_fdid,
                transform: wmo_doodad_transform(doodad_def),
            })
        })
        .collect()
}

pub(super) fn active_wmo_doodad_indices(
    root: &wmo::WmoRootData,
    active_doodad_set: u16,
) -> std::collections::HashSet<u16> {
    let mut indices = std::collections::HashSet::new();
    if root.doodad_sets.is_empty() {
        indices.extend((0..root.doodad_defs.len()).filter_map(|idx| u16::try_from(idx).ok()));
        return indices;
    }

    add_wmo_doodad_set_indices(&mut indices, root.doodad_sets.first());
    if active_doodad_set != 0 {
        add_wmo_doodad_set_indices(
            &mut indices,
            root.doodad_sets.get(active_doodad_set as usize),
        );
    }
    indices
}

pub(super) fn add_wmo_doodad_set_indices(
    indices: &mut std::collections::HashSet<u16>,
    doodad_set: Option<&wmo::WmoDoodadSet>,
) {
    let Some(doodad_set) = doodad_set else { return };
    let start = doodad_set.start_doodad;
    let end = start.saturating_add(doodad_set.n_doodads);
    indices.extend((start..end).filter_map(|idx| u16::try_from(idx).ok()));
}

/// Resolve a doodad FDID from MODI (preferred) or MODN name → listfile lookup (fallback).
///
/// MODD entries reference doodads by `name_offset` — a byte offset into the MODN string table.
/// MODI entries are indexed by *name index* (sequential position), not byte offset.
pub(super) fn resolve_wmo_doodad_fdid(root: &wmo::WmoRootData, name_offset: u32) -> Option<u32> {
    let name_index = root
        .doodad_names
        .iter()
        .position(|n| n.offset == name_offset);

    // MODI path: use FDID directly, no listfile needed
    let modi_fdid = name_index.and_then(|idx| root.doodad_file_ids.get(idx).copied());
    if let Some(fdid) = modi_fdid.filter(|&id| id != 0) {
        return Some(fdid);
    }

    // Fallback: MODN name → listfile path → FDID
    let name = name_index
        .and_then(|idx| root.doodad_names.get(idx))
        .map(|n| &n.name)?;
    game_engine::listfile::lookup_path(name)
}

pub(super) fn wmo_doodad_transform(doodad_def: &wmo::WmoDoodadDef) -> Transform {
    let [x, y, z] = crate::asset::wmo::wmo_local_to_bevy(
        doodad_def.position[0],
        doodad_def.position[1],
        doodad_def.position[2],
    );
    Transform::from_translation(Vec3::new(x, y, z))
        .with_rotation(wow_quat_to_bevy(doodad_def.rotation))
        .with_scale(Vec3::splat(doodad_def.scale))
}

pub(super) fn spawn_wmo_group_doodad(
    commands: &mut Commands,
    assets: &mut WmoAssets<'_>,
    doodad: &WmoGroupDoodad,
) -> Option<Entity> {
    let model_path = crate::asset::asset_cache::model(doodad.model_fdid)?;
    if !model_path.exists() {
        return None;
    }
    let name = model_path
        .file_stem()
        .and_then(|stem| stem.to_str())
        .unwrap_or("wmo_doodad")
        .to_owned();
    let entity = commands
        .spawn((Name::new(name), doodad.transform, Visibility::default()))
        .id();
    if !m2_spawn::spawn_m2_on_entity(
        commands,
        &mut m2_spawn::SpawnAssets {
            meshes: assets.meshes,
            materials: assets.materials,
            effect_materials: assets.effect_materials,
            skybox_materials: None,
            images: assets.images,
            inverse_bindposes: assets.inverse_bindposes,
        },
        &model_path,
        entity,
        &[0, 0, 0],
    ) {
        commands.entity(entity).despawn();
        return None;
    }
    if let Some(kind) = crate::target::classify_world_object_model(&model_path.to_string_lossy()) {
        commands
            .entity(entity)
            .insert(crate::target::WorldObjectInteraction { kind });
    }
    Some(entity)
}

pub(super) fn group_bbox(
    root: &wmo::WmoRootData,
    group_index: u16,
    group_header: &wmo::WmoGroupHeader,
) -> game_engine::culling::WmoGroup {
    let (bbox_min, bbox_max) = root
        .group_infos
        .get(group_index as usize)
        .map(|info| {
            let min = crate::asset::wmo::wmo_local_to_bevy(
                info.bbox_min[0],
                info.bbox_min[1],
                info.bbox_min[2],
            );
            let max = crate::asset::wmo::wmo_local_to_bevy(
                info.bbox_max[0],
                info.bbox_max[1],
                info.bbox_max[2],
            );
            (
                Vec3::new(min[0].min(max[0]), min[1].min(max[1]), min[2].min(max[2])),
                Vec3::new(min[0].max(max[0]), min[1].max(max[1]), min[2].max(max[2])),
            )
        })
        .unwrap_or((Vec3::splat(f32::MIN), Vec3::splat(f32::MAX)));
    game_engine::culling::WmoGroup {
        group_index,
        bbox_min,
        bbox_max,
        is_antiportal: group_is_antiportal(root, group_header),
    }
}

pub(super) fn group_is_antiportal(
    root: &wmo::WmoRootData,
    group_header: &wmo::WmoGroupHeader,
) -> bool {
    root.group_names.iter().any(|group_name| {
        group_name.is_antiportal
            && (group_name.offset == group_header.group_name_offset
                || group_name.offset == group_header.descriptive_group_name_offset)
    })
}

pub(super) fn wmo_batch_material(
    materials: &mut Assets<StandardMaterial>,
    images: &mut Assets<Image>,
    material_index: u16,
    material_props: &WmoMaterialProps,
    interior_ambient: Option<[f32; 4]>,
    has_vertex_color: bool,
) -> Handle<StandardMaterial> {
    let image = load_wmo_batch_material_image(images, material_index, &material_props);
    materials.add(wmo_standard_material(
        image,
        material_props.blend_mode,
        material_props.unculled,
        material_props.shader,
        interior_ambient,
        has_vertex_color,
        material_props.sidn_glow,
    ))
}

pub(super) fn build_wmo_interior_ambient(
    root: &wmo::WmoRootData,
    group: &wmo::WmoGroupData,
) -> Option<[f32; 4]> {
    let rgb = &root.ambient_color[..3];
    (group.header.group_flags.interior && rgb.iter().any(|channel| *channel > 0.0))
        .then_some(root.ambient_color)
}
