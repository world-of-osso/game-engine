use super::*;

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

    let min_corner = Vec3::from(placement_to_bevy_absolute(placement.extents_min));
    let max_corner = Vec3::from(placement_to_bevy_absolute(placement.extents_max));
    let expected_min = min_corner.min(max_corner);
    let expected_max = min_corner.max(max_corner);

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
        materials: vec![
            wmo::WmoMaterialDef {
                texture_fdid: 124134,
                ground_type: 0,
                ..minimal_mat()
            },
            wmo::WmoMaterialDef {
                texture_fdid: 123010,
                ground_type: 5,
                ..minimal_mat()
            },
        ],
        ..minimal_root()
    };

    assert_eq!(
        build_wmo_footstep_surface(&root),
        Some(WmoFootstepSurface {
            surface: FootstepSurface::Wood,
        })
    );
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
                ..minimal_root()
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
