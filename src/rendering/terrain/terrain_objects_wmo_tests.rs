use super::*;
use crate::asset::wmo_format::parser::{WmoLiquidHeader, WmoLiquidTile, WmoLiquidVertex};
use bevy::ecs::system::RunSystemOnce;

#[test]
fn build_wmo_adt_metadata_preserves_modf_sets() {
    let placement = adt_obj::WmoPlacement {
        name_id: 1,
        unique_id: 77,
        position: [0.0, 0.0, 0.0],
        rotation: [0.0, 0.0, 0.0],
        extents_min: [10.0, 20.0, 30.0],
        extents_max: [40.0, 50.0, 60.0],
        flags: 0,
        doodad_set: 3,
        name_set: 9,
        scale: 1.0,
        fdid: Some(123),
        path: None,
    };

    assert_eq!(
        build_wmo_adt_metadata(&placement),
        WmoAdtMetadata {
            unique_id: 77,
            doodad_set: 3,
            name_set: 9,
        }
    );
}

#[test]
fn build_wmo_root_bounds_converts_modf_extents() {
    let placement = adt_obj::WmoPlacement {
        name_id: 1,
        unique_id: 77,
        position: [0.0, 0.0, 0.0],
        rotation: [0.0, 0.0, 0.0],
        extents_min: [40.0, 50.0, 60.0],
        extents_max: [10.0, 20.0, 30.0],
        flags: 0,
        doodad_set: 3,
        name_set: 9,
        scale: 1.0,
        fdid: Some(123),
        path: None,
    };

    let expected_min = Vec3::from(placement_to_bevy_absolute(placement.extents_min)).min(
        Vec3::from(placement_to_bevy_absolute(placement.extents_max)),
    );
    let expected_max = Vec3::from(placement_to_bevy_absolute(placement.extents_min)).max(
        Vec3::from(placement_to_bevy_absolute(placement.extents_max)),
    );

    assert_eq!(
        build_wmo_root_bounds(&placement),
        game_engine::culling::WmoRootBounds {
            world_min: expected_min,
            world_max: expected_max,
        }
    );
}

#[test]
fn spawn_wmo_root_entity_attaches_adt_metadata() {
    let mut app = App::new();
    let metadata = WmoAdtMetadata {
        unique_id: 88,
        doodad_set: 4,
        name_set: 6,
    };
    let bounds = game_engine::culling::WmoRootBounds {
        world_min: Vec3::new(-1.0, -2.0, -3.0),
        world_max: Vec3::new(1.0, 2.0, 3.0),
    };

    let entity = app
        .world_mut()
        .run_system_once(move |mut commands: Commands| {
            spawn_wmo_root_entity(
                &mut commands,
                12345,
                Transform::IDENTITY,
                game_engine::culling::WmoPortalGraph {
                    adjacency: Vec::new(),
                    portal_verts: Vec::new(),
                },
                metadata,
                Some(game_engine::culling::ChunkRefs {
                    chunk_indices: vec![4, 8],
                }),
                bounds,
                Some(WmoFootstepSurface {
                    surface: FootstepSurface::Wood,
                }),
                None,
            )
        });
    app.update();
    let entity = entity.expect("entity should spawn");

    let stored = app
        .world()
        .get::<WmoAdtMetadata>(entity)
        .copied()
        .expect("metadata component");
    assert_eq!(stored, metadata);
    let stored_bounds = app
        .world()
        .get::<game_engine::culling::WmoRootBounds>(entity)
        .copied()
        .expect("bounds component");
    assert_eq!(stored_bounds, bounds);
    let stored_chunk_refs = app
        .world()
        .get::<game_engine::culling::ChunkRefs>(entity)
        .cloned()
        .expect("chunk refs component");
    assert_eq!(stored_chunk_refs.chunk_indices, vec![4, 8]);
    let stored_surface = app
        .world()
        .get::<WmoFootstepSurface>(entity)
        .copied()
        .expect("footstep surface component");
    assert_eq!(
        stored_surface,
        WmoFootstepSurface {
            surface: FootstepSurface::Wood,
        }
    );
}

#[test]
fn wmo_debug_label_includes_non_default_name_set() {
    assert_eq!(
        wmo_debug_label("world/wmo/test.wmo".into(), 0),
        "world/wmo/test.wmo"
    );
    assert_eq!(
        wmo_debug_label("world/wmo/test.wmo".into(), 6),
        "world/wmo/test.wmo nameSet=6"
    );
}

#[test]
fn build_wmo_footstep_surface_prefers_ground_typed_materials() {
    let root = wmo::WmoRootData {
        n_groups: 0,
        flags: wmo::WmoRootFlags::default(),
        ambient_color: [0.0; 4],
        bbox_min: [0.0; 3],
        bbox_max: [0.0; 3],
        materials: vec![
            wmo::WmoMaterialDef {
                texture_fdid: 124134,
                texture_2_fdid: 0,
                texture_3_fdid: 0,
                flags: 0,
                material_flags: wmo::WmoMaterialFlags::default(),
                sidn_color: [0.0; 4],
                diff_color: [0.0; 4],
                ground_type: 0,
                blend_mode: 0,
                shader: 0,
                uv_translation_speed: None,
            },
            wmo::WmoMaterialDef {
                texture_fdid: 123010,
                texture_2_fdid: 0,
                texture_3_fdid: 0,
                flags: 0,
                material_flags: wmo::WmoMaterialFlags::default(),
                sidn_color: [0.0; 4],
                diff_color: [0.0; 4],
                ground_type: 5,
                blend_mode: 0,
                shader: 0,
                uv_translation_speed: None,
            },
        ],
        lights: Vec::new(),
        doodad_sets: Vec::new(),
        group_names: Vec::new(),
        doodad_names: Vec::new(),
        doodad_file_ids: Vec::new(),
        doodad_defs: Vec::new(),
        fogs: Vec::new(),
        visible_block_vertices: Vec::new(),
        visible_blocks: Vec::new(),
        convex_volume_planes: Vec::new(),
        group_file_data_ids: Vec::new(),
        global_ambient_volumes: Vec::new(),
        ambient_volumes: Vec::new(),
        baked_ambient_box_volumes: Vec::new(),
        dynamic_lights: Vec::new(),
        portals: Vec::new(),
        portal_refs: Vec::new(),
        group_infos: Vec::new(),
        skybox_wow_path: None,
    };

    assert_eq!(
        build_wmo_footstep_surface(&root),
        Some(WmoFootstepSurface {
            surface: FootstepSurface::Wood,
        })
    );
}

#[test]
fn collect_group_doodads_filters_to_default_and_selected_set_refs() {
    let group = wmo::WmoGroupData {
        header: wmo::WmoGroupHeader {
            group_name_offset: 0,
            descriptive_group_name_offset: 0,
            flags: 0,
            group_flags: Default::default(),
            bbox_min: [0.0; 3],
            bbox_max: [0.0; 3],
            portal_start: 0,
            portal_count: 0,
            trans_batch_count: 0,
            int_batch_count: 0,
            ext_batch_count: 0,
            batch_type_d: 0,
            fog_ids: [0; 4],
            group_liquid: 0,
            unique_id: 0,
            flags2: 0,
            parent_split_group_index: -1,
            next_split_child_group_index: -1,
        },
        doodad_refs: vec![0, 2, 3, 4],
        light_refs: Vec::new(),
        bsp_nodes: Vec::new(),
        bsp_face_refs: Vec::new(),
        liquid: None,
        batches: Vec::new(),
    };
    let root = wmo::WmoRootData {
        n_groups: 1,
        flags: wmo::WmoRootFlags::default(),
        ambient_color: [0.0; 4],
        bbox_min: [0.0; 3],
        bbox_max: [0.0; 3],
        materials: Vec::new(),
        lights: Vec::new(),
        doodad_sets: vec![
            wmo::WmoDoodadSet {
                name: "$DefaultGlobal".into(),
                start_doodad: 0,
                n_doodads: 2,
            },
            wmo::WmoDoodadSet {
                name: "InnProps".into(),
                start_doodad: 2,
                n_doodads: 2,
            },
        ],
        group_names: Vec::new(),
        doodad_names: vec![
            wmo::WmoDoodadName {
                offset: 0,
                name: "world/generic/passive_doodad_0.m2".into(),
            },
            wmo::WmoDoodadName {
                offset: 1,
                name: "world/generic/passive_doodad_1.m2".into(),
            },
            wmo::WmoDoodadName {
                offset: 2,
                name: "world/generic/selected_doodad_2.m2".into(),
            },
            wmo::WmoDoodadName {
                offset: 3,
                name: "world/generic/selected_doodad_3.m2".into(),
            },
            wmo::WmoDoodadName {
                offset: 4,
                name: "world/generic/unused_doodad_4.m2".into(),
            },
        ],
        doodad_file_ids: vec![100, 101, 102, 103, 104],
        doodad_defs: vec![
            wmo::WmoDoodadDef {
                name_offset: 0,
                flags: 0,
                position: [1.0, 2.0, 3.0],
                rotation: [0.0, 0.0, 0.0, 1.0],
                scale: 1.0,
                color: [1.0; 4],
            },
            wmo::WmoDoodadDef {
                name_offset: 1,
                flags: 0,
                position: [0.0; 3],
                rotation: [0.0, 0.0, 0.0, 1.0],
                scale: 1.0,
                color: [1.0; 4],
            },
            wmo::WmoDoodadDef {
                name_offset: 2,
                flags: 0,
                position: [4.0, 5.0, 6.0],
                rotation: [0.0, 0.0, 0.0, 1.0],
                scale: 0.5,
                color: [1.0; 4],
            },
            wmo::WmoDoodadDef {
                name_offset: 3,
                flags: 0,
                position: [7.0, 8.0, 9.0],
                rotation: [0.0, 0.0, 0.0, 1.0],
                scale: 2.0,
                color: [1.0; 4],
            },
            wmo::WmoDoodadDef {
                name_offset: 4,
                flags: 0,
                position: [10.0, 11.0, 12.0],
                rotation: [0.0, 0.0, 0.0, 1.0],
                scale: 3.0,
                color: [1.0; 4],
            },
        ],
        fogs: Vec::new(),
        visible_block_vertices: Vec::new(),
        visible_blocks: Vec::new(),
        convex_volume_planes: Vec::new(),
        group_file_data_ids: Vec::new(),
        global_ambient_volumes: Vec::new(),
        ambient_volumes: Vec::new(),
        baked_ambient_box_volumes: Vec::new(),
        dynamic_lights: Vec::new(),
        portals: Vec::new(),
        portal_refs: Vec::new(),
        group_infos: Vec::new(),
        skybox_wow_path: None,
    };

    let doodads = collect_group_doodads(&root, &group, 1);
    assert_eq!(doodads.len(), 3);
    assert_eq!(
        doodads
            .iter()
            .map(|doodad| doodad.model_fdid)
            .collect::<Vec<_>>(),
        vec![100, 102, 103]
    );
    assert_eq!(doodads[0].transform.translation, Vec3::new(-1.0, 3.0, 2.0));
    assert_eq!(doodads[1].transform.scale, Vec3::splat(0.5));
    assert_eq!(doodads[2].transform.scale, Vec3::splat(2.0));
}

#[test]
fn collect_group_lights_filters_to_group_light_refs() {
    let group = wmo::WmoGroupData {
        header: wmo::WmoGroupHeader {
            group_name_offset: 0,
            descriptive_group_name_offset: 0,
            flags: 0,
            group_flags: Default::default(),
            bbox_min: [0.0; 3],
            bbox_max: [0.0; 3],
            portal_start: 0,
            portal_count: 0,
            trans_batch_count: 0,
            int_batch_count: 0,
            ext_batch_count: 0,
            batch_type_d: 0,
            fog_ids: [0; 4],
            group_liquid: 0,
            unique_id: 0,
            flags2: 0,
            parent_split_group_index: -1,
            next_split_child_group_index: -1,
        },
        doodad_refs: Vec::new(),
        light_refs: vec![0, 2, 9],
        bsp_nodes: Vec::new(),
        bsp_face_refs: Vec::new(),
        liquid: None,
        batches: Vec::new(),
    };
    let root = wmo::WmoRootData {
        n_groups: 1,
        flags: wmo::WmoRootFlags::default(),
        ambient_color: [0.0; 4],
        bbox_min: [0.0; 3],
        bbox_max: [0.0; 3],
        materials: Vec::new(),
        lights: vec![
            wmo::WmoLight {
                light_type: wmo::WmoLightType::Omni,
                use_attenuation: true,
                color: [1.0, 0.0, 0.0, 1.0],
                position: [1.0, 2.0, 3.0],
                intensity: 4.0,
                rotation: [0.0, 0.0, 0.0, 1.0],
                attenuation_start: 1.0,
                attenuation_end: 5.0,
            },
            wmo::WmoLight {
                light_type: wmo::WmoLightType::Ambient,
                use_attenuation: false,
                color: [0.0, 1.0, 0.0, 1.0],
                position: [4.0, 5.0, 6.0],
                intensity: 2.0,
                rotation: [0.0, 0.0, 0.0, 1.0],
                attenuation_start: 0.0,
                attenuation_end: 0.0,
            },
            wmo::WmoLight {
                light_type: wmo::WmoLightType::Spot,
                use_attenuation: true,
                color: [0.0, 0.0, 1.0, 1.0],
                position: [7.0, 8.0, 9.0],
                intensity: 6.0,
                rotation: [0.0, 0.0, 0.0, 1.0],
                attenuation_start: 2.0,
                attenuation_end: 10.0,
            },
        ],
        doodad_sets: Vec::new(),
        group_names: Vec::new(),
        doodad_names: Vec::new(),
        doodad_file_ids: Vec::new(),
        doodad_defs: Vec::new(),
        fogs: Vec::new(),
        visible_block_vertices: Vec::new(),
        visible_blocks: Vec::new(),
        convex_volume_planes: Vec::new(),
        group_file_data_ids: Vec::new(),
        global_ambient_volumes: Vec::new(),
        ambient_volumes: Vec::new(),
        baked_ambient_box_volumes: Vec::new(),
        dynamic_lights: Vec::new(),
        portals: Vec::new(),
        portal_refs: Vec::new(),
        group_infos: Vec::new(),
        skybox_wow_path: None,
    };

    let lights = collect_group_lights(&root, &group);
    assert_eq!(lights.len(), 2);
    assert_eq!(lights[0].0, 0);
    assert_eq!(lights[1].0, 2);
    assert_eq!(lights[0].1.position, [1.0, 2.0, 3.0]);
    assert_eq!(lights[1].1.position, [7.0, 8.0, 9.0]);
}

#[test]
fn collect_group_fogs_filters_to_valid_unique_group_fog_ids() {
    let group = wmo::WmoGroupData {
        header: wmo::WmoGroupHeader {
            group_name_offset: 0,
            descriptive_group_name_offset: 0,
            flags: 0,
            group_flags: Default::default(),
            bbox_min: [0.0; 3],
            bbox_max: [0.0; 3],
            portal_start: 0,
            portal_count: 0,
            trans_batch_count: 0,
            int_batch_count: 0,
            ext_batch_count: 0,
            batch_type_d: 0,
            fog_ids: [1, 3, 1, 9],
            group_liquid: 0,
            unique_id: 0,
            flags2: 0,
            parent_split_group_index: -1,
            next_split_child_group_index: -1,
        },
        doodad_refs: Vec::new(),
        light_refs: Vec::new(),
        bsp_nodes: Vec::new(),
        bsp_face_refs: Vec::new(),
        liquid: None,
        batches: Vec::new(),
    };
    let root = wmo::WmoRootData {
        n_groups: 1,
        flags: wmo::WmoRootFlags::default(),
        ambient_color: [0.0; 4],
        bbox_min: [0.0; 3],
        bbox_max: [0.0; 3],
        materials: Vec::new(),
        lights: Vec::new(),
        doodad_sets: Vec::new(),
        group_names: Vec::new(),
        doodad_names: Vec::new(),
        doodad_file_ids: Vec::new(),
        doodad_defs: Vec::new(),
        fogs: vec![
            wmo::WmoFog {
                flags: 0,
                position: [1.0, 2.0, 3.0],
                smaller_radius: 4.0,
                larger_radius: 5.0,
                fog_end: 6.0,
                fog_start_multiplier: 0.2,
                color_1: [0.1, 0.2, 0.3, 1.0],
                underwater_fog_end: 7.0,
                underwater_fog_start_multiplier: 0.3,
                color_2: [0.4, 0.5, 0.6, 1.0],
            },
            wmo::WmoFog {
                flags: 1,
                position: [10.0, 20.0, 30.0],
                smaller_radius: 40.0,
                larger_radius: 50.0,
                fog_end: 60.0,
                fog_start_multiplier: 0.4,
                color_1: [0.7, 0.2, 0.3, 1.0],
                underwater_fog_end: 70.0,
                underwater_fog_start_multiplier: 0.5,
                color_2: [0.4, 0.8, 0.6, 1.0],
            },
            wmo::WmoFog {
                flags: 2,
                position: [100.0, 200.0, 300.0],
                smaller_radius: 400.0,
                larger_radius: 500.0,
                fog_end: 600.0,
                fog_start_multiplier: 0.6,
                color_1: [0.1, 0.9, 0.3, 1.0],
                underwater_fog_end: 700.0,
                underwater_fog_start_multiplier: 0.7,
                color_2: [0.4, 0.5, 0.9, 1.0],
            },
            wmo::WmoFog {
                flags: 3,
                position: [11.0, 22.0, 33.0],
                smaller_radius: 44.0,
                larger_radius: 55.0,
                fog_end: 66.0,
                fog_start_multiplier: 0.8,
                color_1: [0.8, 0.2, 0.3, 1.0],
                underwater_fog_end: 77.0,
                underwater_fog_start_multiplier: 0.9,
                color_2: [0.4, 0.8, 0.9, 1.0],
            },
        ],
        visible_block_vertices: Vec::new(),
        visible_blocks: Vec::new(),
        convex_volume_planes: Vec::new(),
        group_file_data_ids: Vec::new(),
        global_ambient_volumes: Vec::new(),
        ambient_volumes: Vec::new(),
        baked_ambient_box_volumes: Vec::new(),
        dynamic_lights: Vec::new(),
        portals: Vec::new(),
        portal_refs: Vec::new(),
        group_infos: Vec::new(),
        skybox_wow_path: None,
    };

    let fogs = collect_group_fogs(&root, &group);
    assert_eq!(fogs.len(), 2);
    assert_eq!(fogs[0].0, 1);
    assert_eq!(fogs[1].0, 3);
    assert_eq!(fogs[0].1.position, [10.0, 20.0, 30.0]);
    assert_eq!(fogs[1].1.position, [11.0, 22.0, 33.0]);
}

#[test]
fn group_bbox_marks_antiportal_groups_from_authored_name_offsets() {
    let root = wmo::WmoRootData {
        n_groups: 1,
        flags: wmo::WmoRootFlags::default(),
        ambient_color: [0.0; 4],
        bbox_min: [0.0; 3],
        bbox_max: [0.0; 3],
        materials: Vec::new(),
        lights: Vec::new(),
        doodad_sets: Vec::new(),
        group_names: vec![
            crate::asset::wmo_format::parser::WmoGroupName {
                offset: 0,
                name: "EntryHall".into(),
                is_antiportal: false,
            },
            crate::asset::wmo_format::parser::WmoGroupName {
                offset: 24,
                name: "antiportal01".into(),
                is_antiportal: true,
            },
        ],
        doodad_names: Vec::new(),
        doodad_file_ids: Vec::new(),
        doodad_defs: Vec::new(),
        fogs: Vec::new(),
        visible_block_vertices: Vec::new(),
        visible_blocks: Vec::new(),
        convex_volume_planes: Vec::new(),
        group_file_data_ids: Vec::new(),
        global_ambient_volumes: Vec::new(),
        ambient_volumes: Vec::new(),
        baked_ambient_box_volumes: Vec::new(),
        dynamic_lights: Vec::new(),
        portals: Vec::new(),
        portal_refs: Vec::new(),
        group_infos: vec![wmo::WmoGroupInfo {
            flags: 0,
            bbox_min: [1.0, 2.0, 3.0],
            bbox_max: [4.0, 5.0, 6.0],
        }],
        skybox_wow_path: None,
    };
    let group_header = wmo::WmoGroupHeader {
        group_name_offset: 0,
        descriptive_group_name_offset: 24,
        flags: 0,
        group_flags: Default::default(),
        bbox_min: [0.0; 3],
        bbox_max: [0.0; 3],
        portal_start: 0,
        portal_count: 0,
        trans_batch_count: 0,
        int_batch_count: 0,
        ext_batch_count: 0,
        batch_type_d: 0,
        fog_ids: [0; 4],
        group_liquid: 0,
        unique_id: 0,
        flags2: 0,
        parent_split_group_index: -1,
        next_split_child_group_index: -1,
    };

    let bbox = group_bbox(&root, 0, &group_header);

    assert!(bbox.is_antiportal);
    assert_eq!(bbox.bbox_min, Vec3::new(-4.0, 3.0, 2.0));
    assert_eq!(bbox.bbox_max, Vec3::new(-1.0, 6.0, 5.0));
}

#[test]
fn spawn_wmo_group_fog_preserves_authored_fog_fields() {
    let mut app = App::new();
    let fog = wmo::WmoFog {
        flags: 0,
        position: [10.0, 20.0, 30.0],
        smaller_radius: 4.0,
        larger_radius: 12.0,
        fog_end: 80.0,
        fog_start_multiplier: 0.25,
        color_1: [0.1, 0.2, 0.3, 1.0],
        underwater_fog_end: 90.0,
        underwater_fog_start_multiplier: 0.5,
        color_2: [0.7, 0.8, 0.9, 1.0],
    };

    let entity = app
        .world_mut()
        .run_system_once(move |mut commands: Commands| spawn_wmo_group_fog(&mut commands, 2, &fog))
        .expect("fog entity should spawn");
    app.update();

    let component = app
        .world()
        .get::<WmoGroupFogVolume>(entity)
        .copied()
        .expect("fog component");
    assert_eq!(
        component,
        WmoGroupFogVolume {
            fog_index: 2,
            smaller_radius: 4.0,
            larger_radius: 12.0,
            fog_end: 80.0,
            fog_start_multiplier: 0.25,
            color_1: [0.1, 0.2, 0.3, 1.0],
            underwater_fog_end: 90.0,
            underwater_fog_start_multiplier: 0.5,
            color_2: [0.7, 0.8, 0.9, 1.0],
        }
    );
    let transform = app.world().get::<Transform>(entity).expect("fog transform");
    assert_eq!(transform.translation, Vec3::new(-10.0, 30.0, 20.0));
    assert_eq!(transform.scale, Vec3::splat(12.0));
}

#[test]
fn spawn_wmo_group_batches_marks_mesh_children_for_collision() {
    let mut app = App::new();
    app.world_mut().init_resource::<Assets<Mesh>>();
    app.world_mut().init_resource::<Assets<StandardMaterial>>();
    app.world_mut().init_resource::<Assets<WaterMaterial>>();
    app.world_mut().init_resource::<Assets<Image>>();
    app.world_mut().init_resource::<Assets<M2EffectMaterial>>();
    app.world_mut()
        .init_resource::<Assets<SkinnedMeshInverseBindposes>>();
    let group_entity = app.world_mut().spawn_empty().id();

    let _ = app.world_mut().run_system_once(
        move |mut commands: Commands,
              mut meshes: ResMut<Assets<Mesh>>,
              mut materials: ResMut<Assets<StandardMaterial>>,
              mut water_materials: ResMut<Assets<WaterMaterial>>,
              mut images: ResMut<Assets<Image>>,
              mut effect_materials: ResMut<Assets<M2EffectMaterial>>,
              mut inverse_bindposes: ResMut<Assets<SkinnedMeshInverseBindposes>>| {
            let root = wmo::WmoRootData {
                n_groups: 1,
                flags: wmo::WmoRootFlags::default(),
                ambient_color: [0.0; 4],
                bbox_min: [0.0; 3],
                bbox_max: [0.0; 3],
                materials: Vec::new(),
                lights: Vec::new(),
                doodad_sets: Vec::new(),
                group_names: Vec::new(),
                doodad_names: Vec::new(),
                doodad_file_ids: Vec::new(),
                doodad_defs: Vec::new(),
                fogs: Vec::new(),
                visible_block_vertices: Vec::new(),
                visible_blocks: Vec::new(),
                convex_volume_planes: Vec::new(),
                group_file_data_ids: Vec::new(),
                global_ambient_volumes: Vec::new(),
                ambient_volumes: Vec::new(),
                baked_ambient_box_volumes: Vec::new(),
                dynamic_lights: Vec::new(),
                portals: Vec::new(),
                portal_refs: Vec::new(),
                group_infos: Vec::new(),
                skybox_wow_path: None,
            };
            let mut assets = WmoAssets {
                meshes: &mut meshes,
                materials: &mut materials,
                water_materials: &mut water_materials,
                images: &mut images,
                effect_materials: &mut effect_materials,
                inverse_bindposes: &mut inverse_bindposes,
            };
            spawn_wmo_group_batches(
                &mut commands,
                &mut assets,
                &root,
                None,
                group_entity,
                vec![wmo::WmoGroupBatch {
                    mesh: Mesh::new(
                        PrimitiveTopology::TriangleList,
                        RenderAssetUsages::default(),
                    ),
                    material_index: 0,
                    batch_type: wmo::WmoBatchType::WholeGroup,
                    uses_second_color_blend_alpha: false,
                    uses_second_uv_set: false,
                    uses_third_uv_set: false,
                    uses_generated_tangents: false,
                    has_vertex_color: false,
                }],
            );
        },
    );
    app.update();

    let children = app
        .world()
        .get::<Children>(group_entity)
        .expect("spawned batch child");
    let batch_entity = children[0];
    assert!(
        app.world().get::<WmoCollisionMesh>(batch_entity).is_some(),
        "WMO batch mesh should block player movement"
    );
}

#[test]
fn build_wmo_liquid_mesh_skips_empty_tiles_and_uses_vertex_heights() {
    let liquid = wmo::WmoLiquid {
        header: WmoLiquidHeader {
            x_verts: 3,
            y_verts: 2,
            x_tiles: 2,
            y_tiles: 1,
            position: [10.0, 20.0, 30.0],
            material_id: 7,
        },
        vertices: vec![
            WmoLiquidVertex {
                raw: [0; 4],
                height: 30.0,
            },
            WmoLiquidVertex {
                raw: [0; 4],
                height: 31.0,
            },
            WmoLiquidVertex {
                raw: [0; 4],
                height: 32.0,
            },
            WmoLiquidVertex {
                raw: [0; 4],
                height: 33.0,
            },
            WmoLiquidVertex {
                raw: [0; 4],
                height: 34.0,
            },
            WmoLiquidVertex {
                raw: [0; 4],
                height: 35.0,
            },
        ],
        tiles: vec![
            WmoLiquidTile {
                liquid_type: 3,
                fishable: false,
                shared: false,
            },
            WmoLiquidTile {
                liquid_type: 0x0F,
                fishable: false,
                shared: false,
            },
        ],
    };

    let mesh = build_wmo_liquid_mesh(&liquid);
    let Some(bevy::mesh::VertexAttributeValues::Float32x3(positions)) =
        mesh.attribute(Mesh::ATTRIBUTE_POSITION)
    else {
        panic!("expected wmo liquid positions");
    };
    let Some(bevy::mesh::VertexAttributeValues::Float32x4(colors)) =
        mesh.attribute(Mesh::ATTRIBUTE_COLOR)
    else {
        panic!("expected wmo liquid colors");
    };
    assert_eq!(positions.len(), 4);
    assert_eq!(colors.len(), 4);
    assert_eq!(positions[0], [-10.0, 29.0, 20.0]);
    assert_eq!(positions[1], [-(10.0 + WMO_LIQUID_TILE_SIZE), 30.0, 20.0]);
    assert_eq!(positions[2], [-10.0, 32.0, 20.0 + WMO_LIQUID_TILE_SIZE]);
    assert_eq!(colors[0], [1.0, 1.0, 1.0, 1.0]);
    assert_eq!(mesh.indices().unwrap().len(), 6);
}

#[test]
fn resolve_wmo_group_fdids_uses_gfid_when_available() {
    let gfid = vec![100, 200, 300];
    let result = resolve_wmo_group_fdids(999, 3, &gfid);
    assert_eq!(result, vec![Some(100), Some(200), Some(300)]);
}

#[test]
fn resolve_wmo_group_fdids_treats_zero_gfid_as_none() {
    let gfid = vec![100, 0, 300];
    let result = resolve_wmo_group_fdids(999, 3, &gfid);
    assert_eq!(result, vec![Some(100), None, Some(300)]);
}

#[test]
fn resolve_wmo_group_fdids_truncates_gfid_to_n_groups() {
    let gfid = vec![100, 200, 300, 400];
    let result = resolve_wmo_group_fdids(999, 2, &gfid);
    assert_eq!(result, vec![Some(100), Some(200)]);
}

#[test]
fn resolve_wmo_doodad_fdid_prefers_modi_over_modn() {
    let root = wmo::WmoRootData {
        doodad_names: vec![
            wmo::WmoDoodadName { offset: 0, name: "torch.m2".into() },
            wmo::WmoDoodadName { offset: 9, name: "barrel.m2".into() },
        ],
        doodad_file_ids: vec![1001, 2002],
        ..minimal_root()
    };
    // name_offset=0 → name index 0 → MODI[0] = 1001
    assert_eq!(resolve_wmo_doodad_fdid(&root, 0), Some(1001));
    // name_offset=9 → name index 1 → MODI[1] = 2002
    assert_eq!(resolve_wmo_doodad_fdid(&root, 9), Some(2002));
}

#[test]
fn resolve_wmo_doodad_fdid_skips_zero_modi_entry() {
    let root = wmo::WmoRootData {
        doodad_names: vec![
            wmo::WmoDoodadName { offset: 0, name: "torch.m2".into() },
        ],
        doodad_file_ids: vec![0],
        ..minimal_root()
    };
    // MODI has 0 → should fall through (listfile won't resolve in tests, returns None)
    assert_eq!(resolve_wmo_doodad_fdid(&root, 0), None);
}

#[test]
fn resolve_wmo_doodad_fdid_returns_none_for_unknown_offset() {
    let root = wmo::WmoRootData {
        doodad_names: vec![
            wmo::WmoDoodadName { offset: 0, name: "torch.m2".into() },
        ],
        doodad_file_ids: vec![1001],
        ..minimal_root()
    };
    // name_offset=99 doesn't match any doodad_names entry
    assert_eq!(resolve_wmo_doodad_fdid(&root, 99), None);
}

fn minimal_root() -> wmo::WmoRootData {
    wmo::WmoRootData {
        n_groups: 0,
        flags: Default::default(),
        ambient_color: [0.0; 4],
        bbox_min: [0.0; 3],
        bbox_max: [0.0; 3],
        materials: Vec::new(),
        lights: Vec::new(),
        doodad_sets: Vec::new(),
        group_names: Vec::new(),
        doodad_names: Vec::new(),
        doodad_file_ids: Vec::new(),
        doodad_defs: Vec::new(),
        fogs: Vec::new(),
        visible_block_vertices: Vec::new(),
        visible_blocks: Vec::new(),
        convex_volume_planes: Vec::new(),
        group_file_data_ids: Vec::new(),
        global_ambient_volumes: Vec::new(),
        ambient_volumes: Vec::new(),
        baked_ambient_box_volumes: Vec::new(),
        dynamic_lights: Vec::new(),
        portals: Vec::new(),
        portal_refs: Vec::new(),
        group_infos: Vec::new(),
        skybox_wow_path: None,
    }
}
