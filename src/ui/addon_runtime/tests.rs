use ui_toolkit::frame::{Dimension, WidgetData};

use super::*;

fn make_registry_with_root() -> ui_toolkit::registry::FrameRegistry {
    let mut registry = ui_toolkit::registry::FrameRegistry::new(1920.0, 1080.0);
    let root_id = registry.create_frame("ParentRoot", None);
    let root = registry.get_mut(root_id).expect("root frame should exist");
    root.width = Dimension::Fixed(400.0);
    root.height = Dimension::Fixed(200.0);
    registry
}

fn font_text(registry: &ui_toolkit::registry::FrameRegistry, name: &str) -> Option<String> {
    let id = registry.get_by_name(name)?;
    let frame = registry.get(id)?;
    let WidgetData::FontString(data) = frame.widget_data.as_ref()? else {
        return None;
    };
    Some(data.text.clone())
}

fn native_addon_app() -> App {
    use bevy::asset::AssetApp;

    let mut app = App::new();
    app.add_plugins((
        MinimalPlugins,
        bevy::asset::AssetPlugin::default(),
        bevy::image::ImagePlugin::default(),
        bevy::mesh::MeshPlugin,
        bevy::window::WindowPlugin {
            primary_window: Some(Window {
                resolution: (800, 600).into(),
                ..default()
            }),
            exit_condition: bevy::window::ExitCondition::DontExit,
            ..default()
        },
        bevy::input::InputPlugin,
        bevy::transform::TransformPlugin,
        bevy::camera::CameraPlugin,
        bevy::text::TextPlugin,
        bevy::picking::DefaultPickingPlugins,
        bevy::ui::UiPlugin,
        ui_toolkit::plugin::UiPlugin,
    ));
    app.init_asset::<TextureAtlasLayout>();
    app.insert_resource(bevy::time::TimeUpdateStrategy::ManualDuration(
        std::time::Duration::from_millis(16),
    ));
    app.world_mut().resource_mut::<UiState>().registry = make_registry_with_root();
    app.finish();
    app.cleanup();
    app
}

fn script_addon(script: &str) -> LoadedAddon {
    let operations = js::run_js_addon_to_operations(script).expect("valid fixture addon");
    LoadedAddon {
        name: "native-contract".into(),
        owned_frames: collect_owned_frames(&operations),
        operations,
    }
}

fn project_addon(app: &mut App, addon: &LoadedAddon) {
    apply::apply_addon(
        addon,
        &mut app.world_mut().resource_mut::<UiState>().registry,
    );
    app.update();
    app.update();
}

fn projected_frame(world: &mut World, id: u64) -> Entity {
    let entities: Vec<_> = world
        .query::<(Entity, &ui_toolkit::native_render::RegistryNode)>()
        .iter(world)
        .filter_map(|(entity, frame)| (frame.0 == id).then_some(entity))
        .collect();
    assert_eq!(
        entities.len(),
        1,
        "one native projection for registry frame {id}"
    );
    entities[0]
}

fn native_descendants(world: &World, root: Entity) -> Vec<Entity> {
    let mut pending = vec![root];
    let mut descendants = Vec::new();
    while let Some(entity) = pending.pop() {
        if let Some(children) = world.get::<Children>(entity) {
            pending.extend(children.iter());
        }
        descendants.push(entity);
    }
    descendants
}

fn projected_text(world: &World, frame: Entity) -> Entity {
    let texts: Vec<_> = native_descendants(world, frame)
        .into_iter()
        .filter(|entity| world.get::<Text>(*entity).is_some())
        .collect();
    assert_eq!(texts.len(), 1, "fixture fontstring has one displayed text");
    texts[0]
}

fn assert_native_rect(world: &World, entity: Entity, expected: [f32; 4]) {
    let node = world
        .get::<ComputedNode>(entity)
        .expect("native layout computed");
    let transform = world.get::<UiGlobalTransform>(entity).unwrap();
    let origin = bevy::math::Affine2::from(transform).translation - node.size / 2.0;
    let actual = [origin.x, origin.y, node.size.x, node.size.y];
    for (actual, expected) in actual.into_iter().zip(expected) {
        assert!(
            (actual - expected).abs() < 0.1,
            "bounds {actual}, expected {expected}"
        );
    }
}

#[test]
fn js_addon_reuses_engine_frames_and_updates_native_bevy_output() {
    let mut app = native_addon_app();
    let (panel, label) = {
        let mut ui = app.world_mut().resource_mut::<UiState>();
        let root = ui.registry.get_by_name("ParentRoot").unwrap();
        let panel = ui.registry.create_frame("EnginePanel", Some(root));
        let label = ui.registry.create_frame("EngineLabel", Some(panel));
        let frame = ui.registry.get_mut(label).unwrap();
        frame.widget_type = ui_toolkit::frame::WidgetType::FontString;
        frame.widget_data = Some(WidgetData::FontString(
            ui_toolkit::widgets::font_string::FontStringData::default(),
        ));
        (panel, label)
    };
    let mut addon = script_addon(
        r#"
        addon.createFrame("EnginePanel", "ParentRoot");
        addon.setSize("EnginePanel", 200, 60);
        addon.setPoint("EnginePanel", "TOPLEFT", "ParentRoot", "TOPLEFT", 10, -20);
        addon.setBackgroundColor("EnginePanel", 0.1, 0.2, 0.3, 0.8);
        addon.setAlpha("EnginePanel", 0.5);
        addon.createFontString("EngineLabel", "EnginePanel", "Initial");
        addon.setSize("EngineLabel", 80, 20);
        addon.setPoint("EngineLabel", "CENTER", "EnginePanel", "CENTER", 0, 0);
        addon.setText("EngineLabel", "Updated");
        addon.setFontColor("EngineLabel", 0.9, 0.7, 0.2, 0.8);
        addon.setAlpha("EngineLabel", 0.5);
    "#,
    );
    project_addon(&mut app, &addon);
    let registry = &app.world().resource::<UiState>().registry;
    assert_eq!(registry.get_by_name("EnginePanel"), Some(panel));
    assert_eq!(registry.get_by_name("EngineLabel"), Some(label));
    assert_eq!(
        font_text(registry, "EngineLabel").as_deref(),
        Some("Updated")
    );
    let panel_entity = projected_frame(app.world_mut(), panel);
    let label_entity = projected_frame(app.world_mut(), label);
    assert_native_rect(app.world(), panel_entity, [10.0, 20.0, 200.0, 60.0]);
    assert_native_rect(app.world(), label_entity, [70.0, 40.0, 80.0, 20.0]);
    assert_eq!(
        app.world().get::<ChildOf>(label_entity).unwrap().parent(),
        panel_entity
    );
    let text = projected_text(app.world(), label_entity);
    assert_eq!(app.world().get::<Text>(text).unwrap().0, "Updated");
    assert_eq!(
        app.world().get::<TextColor>(text).unwrap().0,
        Color::srgba(0.9, 0.7, 0.2, 0.4)
    );
    let images: Vec<_> = app
        .world()
        .get::<Children>(panel_entity)
        .unwrap()
        .iter()
        .filter_map(|entity| app.world().get::<ImageNode>(entity))
        .collect();
    assert_eq!(images.len(), 1);
    assert_eq!(images[0].color, Color::srgba(0.1, 0.2, 0.3, 0.4));

    addon.operations = js::run_js_addon_to_operations(
        r#"
        addon.setSize("EnginePanel", 300, 100);
        addon.setText("EngineLabel", "Resized");
        addon.setSize("ParentRoot", 999, 999);
    "#,
    )
    .unwrap();
    project_addon(&mut app, &addon);
    assert_eq!(projected_frame(app.world_mut(), panel), panel_entity);
    assert_eq!(projected_frame(app.world_mut(), label), label_entity);
    assert_eq!(projected_text(app.world(), label_entity), text);
    assert_native_rect(app.world(), panel_entity, [10.0, 20.0, 300.0, 100.0]);
    assert_native_rect(app.world(), label_entity, [120.0, 60.0, 80.0, 20.0]);
    assert_eq!(app.world().get::<Text>(text).unwrap().0, "Resized");
    let registry = &app.world().resource::<UiState>().registry;
    assert_eq!(
        registry
            .get(registry.get_by_name("ParentRoot").unwrap())
            .unwrap()
            .width,
        Dimension::Fixed(400.0),
        "undeclared parent mutation remains rejected"
    );

    addon.operations = js::run_js_addon_to_operations("addon.hide('EnginePanel');").unwrap();
    project_addon(&mut app, &addon);
    assert!(
        !app.world()
            .resource::<UiState>()
            .registry
            .get(panel)
            .unwrap()
            .visible
    );
    assert_eq!(
        app.world().get::<ComputedNode>(panel_entity).unwrap().size,
        Vec2::ZERO
    );
    addon.operations = js::run_js_addon_to_operations("addon.show('EnginePanel');").unwrap();
    project_addon(&mut app, &addon);
    assert_eq!(projected_frame(app.world_mut(), panel), panel_entity);
    assert_native_rect(app.world(), panel_entity, [10.0, 20.0, 300.0, 100.0]);
    let text = projected_text(app.world(), label_entity);
    assert_eq!(app.world().get::<Text>(text).unwrap().0, "Resized");
}

#[test]
fn js_addon_reload_and_unload_remove_registry_and_native_subtrees() {
    let mut app = native_addon_app();
    let dir = std::env::temp_dir().join(format!(
        "native_addon_reload_{}_{:?}",
        std::process::id(),
        std::thread::current().id()
    ));
    std::fs::create_dir_all(&dir).unwrap();
    let path = dir.join("native.js");
    let script = |value: &str| {
        format!(
            r#"
        addon.createFrame("ReloadNativePanel", "ParentRoot");
        addon.setSize("ReloadNativePanel", 200, 60);
        addon.setPoint("ReloadNativePanel", "TOPLEFT", "ParentRoot", "TOPLEFT", 5, -10);
        addon.createFontString("ReloadNativeLabel", "ReloadNativePanel", "{value}");
        addon.setSize("ReloadNativeLabel", 100, 20);
    "#
        )
    };
    std::fs::write(&path, script("Before")).unwrap();
    let mut runtime = AddonRuntime {
        addon_dir: dir.clone(),
        addons: HashMap::new(),
    };
    {
        let mut ui = app.world_mut().resource_mut::<UiState>();
        runtime.reload_path(path.clone(), &mut ui.registry);
        runtime.apply(&mut ui.registry);
    }
    app.update();
    app.update();
    let old_id = app
        .world()
        .resource::<UiState>()
        .registry
        .get_by_name("ReloadNativePanel")
        .unwrap();
    let old_entity = projected_frame(app.world_mut(), old_id);
    let removed_entities = native_descendants(app.world(), old_entity);
    std::fs::write(&path, script("After")).unwrap();
    {
        let mut ui = app.world_mut().resource_mut::<UiState>();
        runtime.reload_path(path.clone(), &mut ui.registry);
        runtime.apply(&mut ui.registry);
    }
    app.update();
    app.update();
    for entity in removed_entities {
        assert!(
            app.world().get_entity(entity).is_err(),
            "old addon entity survived reload"
        );
    }
    let registry = &app.world().resource::<UiState>().registry;
    assert!(registry.get(old_id).is_none());
    let label_id = registry.get_by_name("ReloadNativeLabel").unwrap();
    let panel_id = registry.get_by_name("ReloadNativePanel").unwrap();
    let label_entity = projected_frame(app.world_mut(), label_id);
    let text = projected_text(app.world(), label_entity);
    assert_eq!(app.world().get::<Text>(text).unwrap().0, "After");
    let panel_entity = projected_frame(app.world_mut(), panel_id);
    let removed_entities = native_descendants(app.world(), panel_entity);
    runtime.unload_path(
        &path,
        &mut app.world_mut().resource_mut::<UiState>().registry,
    );
    app.update();
    app.update();
    for entity in removed_entities {
        assert!(
            app.world().get_entity(entity).is_err(),
            "addon entity survived unload"
        );
    }
    let registry = &app.world().resource::<UiState>().registry;
    assert!(registry.get_by_name("ReloadNativePanel").is_none());
    assert!(registry.get_by_name("ReloadNativeLabel").is_none());
    let root = registry
        .get_by_name("ParentRoot")
        .expect("engine root survives unload");
    projected_frame(app.world_mut(), root);
    std::fs::remove_dir_all(dir).unwrap();
}

fn run_addon_update(app: &mut App) {
    app.world_mut().run_schedule(Update);
    app.world_mut().clear_trackers();
}

fn wait_for_addon_change(signal: &AddonReloadSignal) {
    let deadline = std::time::Instant::now() + std::time::Duration::from_secs(5);
    while !signal.is_dirty() {
        assert!(
            std::time::Instant::now() < deadline,
            "addon bridge did not publish change"
        );
        std::thread::sleep(std::time::Duration::from_millis(1));
    }
}

#[test]
fn bridge_preserves_fifo_and_clears_dirty_after_drain() {
    let (sender, receiver) = std::sync::mpsc::channel();
    let signal = AddonReloadSignal::from_receiver(receiver).unwrap();
    let expected = [PathBuf::from("first.js"), PathBuf::from("second.js")];
    for path in &expected {
        sender.send(path.clone()).unwrap();
    }
    let mut received = Vec::new();
    while received.len() < expected.len() {
        wait_for_addon_change(&signal);
        received.extend(signal.take_paths());
    }
    assert_eq!(received, expected);
    assert!(!signal.is_dirty());
    sender.send(PathBuf::from("third.js")).unwrap();
    wait_for_addon_change(&signal);
    assert_eq!(signal.take_paths(), [PathBuf::from("third.js")]);
    assert!(!signal.is_dirty());
}

#[test]
fn clean_addon_updates_do_not_require_ui_state() {
    let (_sender, receiver) = std::sync::mpsc::channel();
    let mut app = App::new();
    app.add_plugins(AddonRuntimePlugin);
    app.insert_resource(UiProcessingEnabled(true));
    app.insert_resource(UiState {
        registry: make_registry_with_root(),
        event_bus: crate::ui::event::EventBus::new(),
        focused_frame: None,
    });
    let signal = AddonReloadSignal::from_receiver(receiver).unwrap();
    app.insert_resource(signal.clone());
    app.insert_resource(AddonRuntime::default());
    run_addon_update(&mut app);
    app.world_mut().remove_resource::<UiState>();
    // A clean frame must not touch the queue even while another owner holds its lock.
    let _guard = signal.0.paths.lock().unwrap();
    for _ in 0..3 {
        run_addon_update(&mut app);
    }
}

#[test]
fn addon_apply_runs_only_after_load_or_reload_and_respects_ui_enabled() {
    let dir = std::env::temp_dir().join(format!("addon_idle_{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    let path = dir.join("idle.js");
    let script =
        |text: &str| format!("addon.createFontString('IdleLabel', 'ParentRoot', '{text}');");
    std::fs::write(&path, script("Loaded")).unwrap();
    let (sender, receiver) = std::sync::mpsc::channel();
    let mut registry = make_registry_with_root();
    let mut runtime = AddonRuntime {
        addon_dir: dir.clone(),
        addons: HashMap::new(),
    };
    runtime.refresh_all(&mut registry);
    let signal = AddonReloadSignal::from_receiver(receiver).unwrap();
    let mut app = App::new();
    app.insert_resource(signal.clone());
    app.add_plugins(AddonRuntimePlugin);
    app.insert_resource(UiProcessingEnabled(true));
    app.insert_resource(UiState {
        registry,
        event_bus: crate::ui::event::EventBus::new(),
        focused_frame: None,
    });
    app.insert_resource(runtime);
    run_addon_update(&mut app);
    assert_eq!(
        font_text(&app.world().resource::<UiState>().registry, "IdleLabel").as_deref(),
        Some("Loaded")
    );

    let label = app
        .world()
        .resource::<UiState>()
        .registry
        .get_by_name("IdleLabel")
        .unwrap();
    if let Some(WidgetData::FontString(data)) = app
        .world_mut()
        .resource_mut::<UiState>()
        .registry
        .get_mut(label)
        .unwrap()
        .widget_data
        .as_mut()
    {
        data.text = "User edit".into();
    }
    for _ in 0..3 {
        run_addon_update(&mut app);
    }
    assert_eq!(
        font_text(&app.world().resource::<UiState>().registry, "IdleLabel").as_deref(),
        Some("User edit")
    );

    app.world_mut().resource_mut::<UiProcessingEnabled>().0 = false;
    std::fs::write(&path, script("Reloaded")).unwrap();
    sender.send(path.clone()).unwrap();
    wait_for_addon_change(&signal);
    run_addon_update(&mut app);
    assert_eq!(
        font_text(&app.world().resource::<UiState>().registry, "IdleLabel").as_deref(),
        Some("User edit")
    );
    app.world_mut().resource_mut::<UiProcessingEnabled>().0 = true;
    run_addon_update(&mut app);
    assert_eq!(
        font_text(&app.world().resource::<UiState>().registry, "IdleLabel").as_deref(),
        Some("Reloaded")
    );

    app.world_mut().clear_trackers();
    app.world_mut().run_schedule(Update);
    assert!(!app.world().is_resource_changed::<UiState>());
    std::fs::remove_file(&path).unwrap();
    sender.send(path).unwrap();
    wait_for_addon_change(&signal);
    run_addon_update(&mut app);
    assert!(
        app.world()
            .resource::<UiState>()
            .registry
            .get_by_name("IdleLabel")
            .is_none()
    );
    std::fs::remove_dir_all(dir).unwrap();
}

#[test]
fn js_addon_script_emits_expected_operations() {
    let script = r#"
        addon.createFrame("MyPanel", "ParentRoot");
        addon.setSize("MyPanel", 240, 64);
        addon.setPoint("MyPanel", "TOP", "ParentRoot", "BOTTOM", 12, -6);
        addon.setBackgroundColor("MyPanel", 0.1, 0.2, 0.3, 0.9);
        addon.createFontString("MyLabel", "MyPanel", "Hello");
        addon.setText("MyLabel", "Updated");
        addon.setFontColor("MyLabel", 1.0, 0.8, 0.2, 1.0);
    "#;

    let operations = js::run_js_addon_to_operations(script).expect("script should parse");
    assert_eq!(
        operations,
        vec![
            AddonOperation::CreateFrame {
                name: "MyPanel".to_string(),
                parent: Some("ParentRoot".to_string()),
            },
            AddonOperation::SetSize {
                name: "MyPanel".to_string(),
                width: 240.0,
                height: 64.0,
            },
            AddonOperation::SetPoint {
                name: "MyPanel".to_string(),
                point: AnchorPoint::Top,
                relative_to: Some("ParentRoot".to_string()),
                relative_point: AnchorPoint::Bottom,
                x: 12.0,
                y: -6.0,
            },
            AddonOperation::SetBackgroundColor {
                name: "MyPanel".to_string(),
                color: [0.1, 0.2, 0.3, 0.9],
            },
            AddonOperation::CreateFontString {
                name: "MyLabel".to_string(),
                parent: Some("MyPanel".to_string()),
                text: "Hello".to_string(),
            },
            AddonOperation::SetText {
                name: "MyLabel".to_string(),
                text: "Updated".to_string(),
            },
            AddonOperation::SetFontColor {
                name: "MyLabel".to_string(),
                color: [1.0, 0.8, 0.2, 1.0],
            },
        ]
    );
}

#[test]
fn apply_addon_creates_owned_frames_and_updates_text() {
    let operations = js::run_js_addon_to_operations(
        r#"
            addon.createFrame("MyPanel", "ParentRoot");
            addon.setSize("MyPanel", 300, 80);
            addon.setBackgroundColor("MyPanel", 0.05, 0.1, 0.15, 0.8);
            addon.createFontString("MyLabel", "MyPanel", "Ready");
            addon.setPoint("MyLabel", "CENTER", "MyPanel", "CENTER", 0, 0);
            addon.setText("MyLabel", "Loaded");
        "#,
    )
    .expect("script should parse");
    let addon = LoadedAddon {
        name: "demo".to_string(),
        owned_frames: collect_owned_frames(&operations),
        operations,
    };
    let mut registry = make_registry_with_root();

    apply::apply_addon(&addon, &mut registry);

    let panel_id = registry.get_by_name("MyPanel").expect("panel should exist");
    let panel = registry.get(panel_id).expect("panel frame");
    assert_eq!(panel.parent_id, registry.get_by_name("ParentRoot"));
    assert_eq!(panel.width, Dimension::Fixed(300.0));
    assert_eq!(panel.height, Dimension::Fixed(80.0));
    assert_eq!(panel.background_color, Some([0.05, 0.1, 0.15, 0.8]));
    assert_eq!(font_text(&registry, "MyLabel").as_deref(), Some("Loaded"));
}

#[test]
fn addon_resize_invalidates_anchored_layout_and_followers() {
    let operations = js::run_js_addon_to_operations(
        r#"
            addon.createFrame("SizedPanel", "ParentRoot");
            addon.setSize("SizedPanel", 100, 40);
            addon.setPoint("SizedPanel", "TOPLEFT", "ParentRoot", "TOPLEFT", 10, -20);
            addon.createFrame("Follower", "ParentRoot");
            addon.setSize("Follower", 20, 10);
            addon.setPoint("Follower", "TOPLEFT", "SizedPanel", "TOPRIGHT", 5, 0);
        "#,
    )
    .expect("setup script should parse");
    let mut addon = LoadedAddon {
        name: "resize-layout".to_string(),
        owned_frames: collect_owned_frames(&operations),
        operations,
    };
    let mut registry = make_registry_with_root();
    apply::apply_addon(&addon, &mut registry);
    ui_toolkit::layout::recompute_layouts(&mut registry);
    let panel = registry.get_by_name("SizedPanel").unwrap();
    let follower = registry.get_by_name("Follower").unwrap();
    let original = registry.get(panel).unwrap().layout_rect.clone().unwrap();
    assert_eq!(
        (original.x, original.y, original.width, original.height),
        (10.0, 20.0, 100.0, 40.0)
    );
    assert_eq!(
        registry
            .get(follower)
            .unwrap()
            .layout_rect
            .as_ref()
            .unwrap()
            .x,
        115.0
    );

    addon.operations = js::run_js_addon_to_operations(
        r#"
            addon.setSize("SizedPanel", 160, 70);
            addon.setSize("ParentRoot", 999, 999);
        "#,
    )
    .expect("resize script should parse");
    apply::apply_addon(&addon, &mut registry);
    // Keep another frame dirty so a full-layout fallback cannot hide missing invalidation.
    registry.create_frame("UnrelatedDirtyFrame", None);
    ui_toolkit::layout::recompute_layouts(&mut registry);

    let resized = registry.get(panel).unwrap().layout_rect.as_ref().unwrap();
    assert_eq!((resized.x, resized.y), (original.x, original.y));
    assert_eq!((resized.width, resized.height), (160.0, 70.0));
    let following = registry
        .get(follower)
        .unwrap()
        .layout_rect
        .as_ref()
        .unwrap();
    assert_eq!((following.x, following.y), (175.0, 20.0));
    let root = registry
        .get(registry.get_by_name("ParentRoot").unwrap())
        .unwrap();
    assert_eq!(
        (root.width, root.height),
        (Dimension::Fixed(400.0), Dimension::Fixed(200.0))
    );
}

#[test]
fn reload_path_replaces_owned_frames() {
    let dir = std::env::temp_dir().join(format!("codex_addon_reload_{}_{}", std::process::id(), 1));
    std::fs::create_dir_all(&dir).unwrap();
    let path = dir.join("reload.js");
    std::fs::write(
        &path,
        r#"
            addon.createFrame("ReloadPanel", "ParentRoot");
            addon.createFontString("ReloadLabel", "ReloadPanel", "One");
        "#,
    )
    .unwrap();
    let mut runtime = AddonRuntime {
        addon_dir: dir.clone(),
        addons: HashMap::new(),
    };
    let mut registry = make_registry_with_root();

    runtime.reload_path(path.clone(), &mut registry);
    runtime.apply(&mut registry);
    assert_eq!(font_text(&registry, "ReloadLabel").as_deref(), Some("One"));

    std::fs::write(
        &path,
        r#"
            addon.createFrame("ReloadPanel", "ParentRoot");
            addon.createFontString("ReloadLabel", "ReloadPanel", "Two");
            addon.setText("ReloadLabel", "Updated");
        "#,
    )
    .unwrap();

    runtime.reload_path(path.clone(), &mut registry);
    runtime.apply(&mut registry);
    assert_eq!(
        font_text(&registry, "ReloadLabel").as_deref(),
        Some("Updated")
    );

    std::fs::remove_dir_all(&dir).ok();
}
