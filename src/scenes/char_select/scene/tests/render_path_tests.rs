use super::*;

#[test]
fn selected_scene_character_id_uses_selected_index() {
    let char_list = CharacterList(vec![
        character(10, 1, 0, "First"),
        character(20, 2, 0, "Second"),
    ]);

    assert_eq!(selected_scene_character_id(&char_list, Some(1)), Some(20));
}

#[test]
fn selected_scene_character_id_falls_back_to_first_character() {
    let char_list = CharacterList(vec![
        character(10, 1, 0, "First"),
        character(20, 2, 0, "Second"),
    ]);

    assert_eq!(selected_scene_character_id(&char_list, None), Some(10));
    assert_eq!(selected_scene_character_id(&char_list, Some(99)), Some(10));
}

#[test]
fn selected_scene_character_identity_uses_selected_character_name_and_id() {
    let char_list = CharacterList(vec![
        character(10, 1, 0, "Theron"),
        character(20, 2, 1, "Elara"),
    ]);

    assert_eq!(
        selected_scene_character_identity(&char_list, Some(1)),
        (Some("Elara".to_string()), Some(20))
    );
}

#[test]
fn selected_scene_character_identity_falls_back_to_first_character() {
    let char_list = CharacterList(vec![
        character(10, 1, 0, "Theron"),
        character(20, 2, 1, "Elara"),
    ]);

    assert_eq!(
        selected_scene_character_identity(&char_list, Some(99)),
        (Some("Theron".to_string()), Some(10))
    );
}

#[test]
fn replace_scene_tree_character_node_updates_selected_character_identity() {
    let old_entity = Entity::from_bits(10);
    let new_entity = Entity::from_bits(20);
    let mut scene_tree = scene_tree::build_scene_tree(vec![
        scene_tree::background_scene_node(old_entity, "ground", 0, vec![]),
        scene_tree::character_scene_node(
            old_entity,
            "humanmale_hd.m2".to_string(),
            "Human".to_string(),
            "Male".to_string(),
            Some("Theron".to_string()),
            Some(6),
        ),
    ]);

    replace_scene_tree_character_node(
        &mut scene_tree,
        scene_tree::character_scene_node(
            new_entity,
            "humanfemale_hd.m2".to_string(),
            "Human".to_string(),
            "Female".to_string(),
            Some("Elara".to_string()),
            Some(7),
        ),
    );

    let character_node = scene_tree
        .root
        .children
        .iter()
        .find(|child| {
            matches!(
                child.props,
                game_engine::scene_tree::NodeProps::Character { .. }
            )
        })
        .expect("character node should exist");
    let game_engine::scene_tree::NodeProps::Character {
        model,
        gender,
        name,
        character_id,
        ..
    } = &character_node.props
    else {
        panic!("expected character node");
    };

    assert_eq!(character_node.entity, Some(new_entity));
    assert_eq!(model, "humanfemale_hd.m2");
    assert_eq!(gender, "Female");
    assert_eq!(name.as_deref(), Some("Elara"));
    assert_eq!(*character_id, Some(7));
}

#[test]
fn authored_char_select_skybox_path_is_enabled() {
    assert!(setup::should_spawn_authored_char_select_skybox());
}

#[test]
fn char_select_camera_gets_a_sky_dome_child() {
    let mut app = App::new();
    app.init_resource::<Assets<Mesh>>();
    app.init_resource::<Assets<crate::sky_material::SkyMaterial>>();
    app.init_resource::<Assets<Image>>();

    let cloud_maps = {
        let mut images = app.world_mut().resource_mut::<Assets<Image>>();
        crate::sky::cloud_texture::create_procedural_cloud_maps(&mut images)
    };
    app.insert_resource(cloud_maps);

    let camera = app.world_mut().spawn_empty().id();
    let dome = app
        .world_mut()
        .run_system_once(
            move |mut commands: Commands,
                  mut meshes: ResMut<Assets<Mesh>>,
                  mut sky_materials: ResMut<Assets<crate::sky_material::SkyMaterial>>,
                  mut images: ResMut<Assets<Image>>,
                  cloud_maps: Res<crate::sky::cloud_texture::ProceduralCloudMaps>| {
                setup::spawn_char_select_sky_dome(
                    &mut commands,
                    &mut meshes,
                    &mut sky_materials,
                    &mut images,
                    cloud_maps.active_handle(),
                    camera,
                )
            },
        )
        .expect("spawn sky dome");
    app.update();

    let child_of = app.world().get::<ChildOf>(dome).expect("sky dome parent");
    assert_eq!(child_of.parent(), camera);
    assert!(app.world().get::<crate::sky::SkyDome>(dome).is_some());
}

#[test]
fn setup_char_select_scene_proves_render_path_via_runtime_scene_snapshot() {
    let mut app = render_path_test_app();

    app.world_mut()
        .run_system_once(setup::setup_char_select_scene)
        .expect("char-select scene setup should run");
    app.update();

    let (snapshot, formatted, has_camera_fog) = app
        .world_mut()
        .run_system_once(
            |scene_tree: Res<game_engine::scene_tree::SceneTree>,
             transforms: Query<&Transform>,
             fog_query: Query<&DistanceFog, With<Camera3d>>| {
                let snapshot =
                    game_engine::scene_tree::snapshot_scene_tree(&scene_tree, &transforms);
                let formatted = crate::scenes::dump_tree::build_scene_snapshot(&snapshot);
                let camera_entity = scene_tree
                    .root
                    .children
                    .iter()
                    .find(|child| {
                        matches!(
                            child.props,
                            game_engine::scene_tree::NodeProps::Camera { .. }
                        )
                    })
                    .and_then(|child| child.entity)
                    .expect("scene tree should contain a camera entity");
                (snapshot, formatted, fog_query.get(camera_entity).is_ok())
            },
        )
        .expect("scene snapshot should build");

    assert_eq!(snapshot.root.label, "CharSelectScene");

    let background_node = snapshot
        .root
        .children
        .iter()
        .find(|child| {
            matches!(
                child.props,
                game_engine::scene_tree::NodeProps::Background { .. }
            )
        })
        .expect("scene snapshot should contain a background node");
    let game_engine::scene_tree::NodeProps::Background {
        model,
        doodad_count,
    } = &background_node.props
    else {
        panic!("expected background node");
    };
    assert!(
        model.starts_with("terrain:"),
        "render path proof should use the terrain-backed warband scene, got {model}"
    );
    assert!(
        *doodad_count > 0,
        "render path proof should include nearby campsite doodads"
    );
    assert!(
        background_node.children.iter().any(|child| {
            matches!(
                child.props,
                game_engine::scene_tree::NodeProps::Object { ref kind, .. } if kind == "WMO"
            )
        }),
        "terrain-backed background should include at least one WMO child"
    );
    assert!(
        background_node.children.iter().any(|child| {
            matches!(
                child.props,
                game_engine::scene_tree::NodeProps::Object { ref kind, .. } if kind == "Skybox"
            )
        }),
        "terrain-backed background should include the authored skybox child"
    );

    let character_node = snapshot
        .root
        .children
        .iter()
        .find(|child| {
            matches!(
                child.props,
                game_engine::scene_tree::NodeProps::Character { .. }
            )
        })
        .expect("scene snapshot should contain a character node");
    let game_engine::scene_tree::NodeProps::Character {
        name, character_id, ..
    } = &character_node.props
    else {
        panic!("expected character node");
    };
    assert_eq!(name.as_deref(), Some("Theron"));
    assert_eq!(*character_id, Some(6));

    assert!(
        snapshot.root.children.iter().any(|child| {
            matches!(
                child.props,
                game_engine::scene_tree::NodeProps::Camera { .. }
            )
        }),
        "scene snapshot should contain a camera node"
    );
    assert!(
        has_camera_fog,
        "char-select camera should carry distance fog"
    );
    assert!(
        snapshot.root.children.iter().any(|child| {
            matches!(
                child.props,
                game_engine::scene_tree::NodeProps::Light { ref kind, .. } if kind == "spot"
            )
        }),
        "scene snapshot should contain the primary spot light"
    );
    assert!(
        snapshot.root.children.iter().any(|child| {
            matches!(
                child.props,
                game_engine::scene_tree::NodeProps::Light { ref kind, .. } if kind == "directional"
            )
        }),
        "scene snapshot should contain the fill directional light"
    );

    assert!(
        formatted.contains("Background \"terrain:"),
        "formatted scene dump should expose the terrain-backed background: {formatted}"
    );
    assert!(
        formatted.contains("Skybox Skybox"),
        "formatted scene dump should expose the authored skybox object: {formatted}"
    );
    assert!(
        formatted.contains("Character")
            && formatted.contains("name=Theron")
            && formatted.contains("id=6"),
        "formatted scene dump should expose the selected character identity: {formatted}"
    );
    assert!(
        formatted.contains("Camera fov="),
        "formatted scene dump should expose the camera node: {formatted}"
    );
}

#[test]
fn despawning_model_wrapper_removes_model_root_child() {
    let mut app = App::new();
    let wrapper = app
        .world_mut()
        .spawn((CharSelectModelWrapper, CharSelectScene))
        .id();
    let root = app
        .world_mut()
        .spawn((CharSelectModelRoot, CharSelectModelCharacter(10)))
        .id();
    app.world_mut().entity_mut(wrapper).add_child(root);

    app.world_mut().commands().entity(wrapper).despawn();
    app.update();

    assert!(app.world().get_entity(wrapper).is_err());
    assert!(app.world().get_entity(root).is_err());
}

#[test]
fn appearance_sync_waits_until_model_has_geosets_and_materials() {
    let mut app = App::new();
    app.init_resource::<Assets<Mesh>>();
    app.init_resource::<Assets<StandardMaterial>>();
    let root = app.world_mut().spawn_empty().id();
    let child = app.world_mut().spawn_empty().id();
    app.world_mut().entity_mut(root).add_child(child);

    let ready_without_render_targets = app
        .world_mut()
        .run_system_once(
            move |parent_query: Query<&ChildOf>,
                  geoset_query: Query<(Entity, &crate::m2_spawn::GeosetMesh, &ChildOf)>,
                  material_query: Query<(
                Entity,
                &MeshMaterial3d<StandardMaterial>,
                Option<&crate::m2_spawn::GeosetMesh>,
                Option<&crate::m2_spawn::BatchTextureType>,
                &ChildOf,
            )>| {
                character_root_ready_for_appearance_sync(
                    root,
                    &parent_query,
                    &geoset_query,
                    &material_query,
                )
            },
        )
        .expect("readiness query should run");
    assert!(!ready_without_render_targets);

    let mesh = app
        .world_mut()
        .resource_mut::<Assets<Mesh>>()
        .add(Sphere::new(1.0));
    let material = app
        .world_mut()
        .resource_mut::<Assets<StandardMaterial>>()
        .add(StandardMaterial::default());
    app.world_mut().entity_mut(child).insert((
        crate::m2_spawn::GeosetMesh(101),
        Mesh3d(mesh),
        MeshMaterial3d(material),
    ));

    let ready_with_render_targets = app
        .world_mut()
        .run_system_once(
            move |parent_query: Query<&ChildOf>,
                  geoset_query: Query<(Entity, &crate::m2_spawn::GeosetMesh, &ChildOf)>,
                  material_query: Query<(
                Entity,
                &MeshMaterial3d<StandardMaterial>,
                Option<&crate::m2_spawn::GeosetMesh>,
                Option<&crate::m2_spawn::BatchTextureType>,
                &ChildOf,
            )>| {
                character_root_ready_for_appearance_sync(
                    root,
                    &parent_query,
                    &geoset_query,
                    &material_query,
                )
            },
        )
        .expect("readiness query should run");
    assert!(ready_with_render_targets);
}

#[test]
fn resolve_char_select_model_path_returns_none_when_no_characters_exist() {
    let char_list = CharacterList(Vec::new());

    assert_eq!(resolve_char_select_model_path(&char_list, None), None);
}

#[test]
fn race_model_wow_path_covers_known_playable_races_and_sex() {
    assert_eq!(
        race_model_wow_path(1, 0),
        Some("character/human/male/humanmale_hd.m2")
    );
    assert_eq!(
        race_model_wow_path(2, 0),
        Some("character/orc/male/orcmale_hd.m2")
    );
    assert_eq!(
        race_model_wow_path(10, 1),
        Some("character/bloodelf/female/bloodelffemale_hd.m2")
    );
    assert_eq!(
        race_model_wow_path(10, 0),
        Some("character/bloodelf/male/bloodelfmale_hd.m2")
    );
    assert_eq!(race_model_wow_path(99, 0), None);
}
