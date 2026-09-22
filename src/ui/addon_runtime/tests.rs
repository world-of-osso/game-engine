use ui_toolkit::frame::{Dimension, WidgetData, WidgetType};

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
    let mut app = crate::ui::screens::menu_character_layout_test_support::layout_app(800.0, 600.0);
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
            "bounds {actual}, expected {expected}; node={:?}; transform={transform:?}",
            world.get::<Node>(entity)
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
        addon.setPosType("EnginePanel", "absolute");
        addon.setPos("EnginePanel", 10, 20);
        addon.setBackgroundColor("EnginePanel", 0.1, 0.2, 0.3, 0.8);
        addon.setAlpha("EnginePanel", 0.5);
        addon.createFontString("EngineLabel", "EnginePanel", "Initial");
        addon.setSize("EngineLabel", 80, 20);
        addon.setPosType("EngineLabel", "absolute");
        addon.setPos("EngineLabel", 60, 20);
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
    assert_native_rect(app.world(), label_entity, [70.0, 40.0, 80.0, 20.0]);
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
        addon.setPosType("ReloadNativePanel", "absolute");
        addon.setPos("ReloadNativePanel", 5, 10);
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
fn js_layout_api_rejects_bad_arity_types_targets_and_nonfinite_positions() {
    for script in [
        "addon.setPos('Panel', 1);",
        "addon.setPos('Panel', 1, 2, 3);",
        "addon.setPos('Panel', '1', 2);",
        "addon.setPos('Panel', null, 2);",
        "addon.setPos('Panel', NaN, 2);",
        "addon.setPos('Panel', 1, Infinity);",
        "addon.setPos('Panel', -Infinity, 2);",
        "addon.setPos('Panel', 1e100, 2);",
        "addon.setPosType('Panel');",
        "addon.setPosType('Panel', 'absolute', 'extra');",
        "addon.setPosType('Panel', 'fixed');",
        "addon.setPosType('Panel', null);",
        "addon.setAnchor();",
        "addon.setAnchor('Panel', 'parent', 'extra');",
        "addon.setAnchor('Panel', 'Sibling');",
        "addon.setAnchor('Panel', null);",
        "addon.setPoint('Panel', 'TOPLEFT', 'ParentRoot', 'TOPLEFT', 0, 0);",
    ] {
        assert!(
            js::run_js_addon_to_operations(script).is_err(),
            "invalid layout operation was accepted: {script}"
        );
    }
    assert_eq!(
        js::run_js_addon_to_operations("addon.setAnchor('Panel', undefined);").unwrap(),
        vec![AddonOperation::SetAnchor {
            name: "Panel".into(),
            target: AnchorTarget::Parent,
        }]
    );
}

#[test]
fn js_layout_target_changes_native_parent_without_changing_logical_ownership() {
    let mut app = native_addon_app();
    let root = app
        .world()
        .resource::<UiState>()
        .registry
        .get_by_name("ParentRoot")
        .unwrap();
    {
        let mut ui = app.world_mut().resource_mut::<UiState>();
        ui.registry
            .set_pos_type(root, PositionType::Absolute)
            .unwrap();
        ui.registry.set_pos(root, 100.0, 80.0).unwrap();
    }
    let mut addon = script_addon(
        r#"
        addon.createFrame("ScreenSibling", "ParentRoot");
        addon.setAnchor("ScreenSibling", "screen");
        addon.setSize("ScreenSibling", 40, 20);
        addon.createFrame("TargetPanel", "ParentRoot");
        addon.setSize("TargetPanel", 60, 30);
        addon.setPosType("TargetPanel", "absolute");
        addon.setPos("TargetPanel", 15, 25);
    "#,
    );
    project_addon(&mut app, &addon);
    let id = app
        .world()
        .resource::<UiState>()
        .registry
        .get_by_name("TargetPanel")
        .unwrap();
    let entity = projected_frame(app.world_mut(), id);
    let root_entity = projected_frame(app.world_mut(), root);
    assert_native_rect(app.world(), entity, [115.0, 105.0, 60.0, 30.0]);
    assert_eq!(
        app.world().get::<ChildOf>(entity).unwrap().parent(),
        root_entity
    );

    addon.operations =
        js::run_js_addon_to_operations("addon.setAnchor('TargetPanel', 'screen');").unwrap();
    project_addon(&mut app, &addon);
    assert_eq!(projected_frame(app.world_mut(), id), entity);
    assert_native_rect(app.world(), entity, [15.0, 25.0, 60.0, 30.0]);
    assert_ne!(
        app.world().get::<ChildOf>(entity).unwrap().parent(),
        root_entity
    );
    let frame = app.world().resource::<UiState>().registry.get(id).unwrap();
    assert_eq!(frame.parent_id, Some(root));
    assert_eq!(frame.position_type, PositionType::Absolute);
    assert_eq!(frame.anchor, AnchorTarget::Screen);

    addon.operations =
        js::run_js_addon_to_operations("addon.setPosType('TargetPanel', 'relative');").unwrap();
    project_addon(&mut app, &addon);
    let sibling_id = app
        .world()
        .resource::<UiState>()
        .registry
        .get_by_name("ScreenSibling")
        .expect("flow sibling exists");
    let sibling = projected_frame(app.world_mut(), sibling_id);
    assert_eq!(
        app.world().get::<Node>(sibling).unwrap().position_type,
        PositionType::Relative
    );
    assert_native_rect(app.world(), sibling, [0.0, 0.0, 40.0, 20.0]);
    assert_native_rect(app.world(), entity, [55.0, 25.0, 60.0, 30.0]);
    assert_eq!(
        app.world()
            .resource::<UiState>()
            .registry
            .get(id)
            .unwrap()
            .parent_id,
        Some(root)
    );
    addon.operations =
        js::run_js_addon_to_operations("addon.setPosType('TargetPanel', 'absolute');").unwrap();
    project_addon(&mut app, &addon);
    assert_native_rect(app.world(), entity, [15.0, 25.0, 60.0, 30.0]);

    addon.operations = js::run_js_addon_to_operations("addon.setAnchor('TargetPanel');").unwrap();
    project_addon(&mut app, &addon);
    assert_eq!(projected_frame(app.world_mut(), id), entity);
    assert_native_rect(app.world(), entity, [115.0, 105.0, 60.0, 30.0]);
    assert_eq!(
        app.world().get::<ChildOf>(entity).unwrap().parent(),
        root_entity
    );
    addon.operations =
        js::run_js_addon_to_operations("addon.setAnchor('TargetPanel', 'screen');").unwrap();
    project_addon(&mut app, &addon);
    apply::remove_owned_frames(
        &mut app.world_mut().resource_mut::<UiState>().registry,
        &addon.owned_frames,
    );
    app.update();
    app.update();
    assert!(app.world().get_entity(entity).is_err());
    assert!(app.world().get_entity(root_entity).is_ok());
}

#[test]
fn js_relative_positions_offset_layout_flow_while_absolute_positions_leave_it() {
    let mut app = native_addon_app();
    let mut addon = script_addon(
        r#"
        addon.createFrame("FirstFlowChild", "ParentRoot");
        addon.setSize("FirstFlowChild", 100, 20);
        addon.createFrame("SecondFlowChild", "ParentRoot");
        addon.setSize("SecondFlowChild", 50, 20);
        addon.setPosType("SecondFlowChild", "relative");
        addon.setPos("SecondFlowChild", 12, 8);
    "#,
    );
    project_addon(&mut app, &addon);
    let second = app
        .world()
        .resource::<UiState>()
        .registry
        .get_by_name("SecondFlowChild")
        .unwrap();
    let entity = projected_frame(app.world_mut(), second);
    assert_native_rect(app.world(), entity, [112.0, 8.0, 50.0, 20.0]);
    addon.operations =
        js::run_js_addon_to_operations("addon.setPosType('SecondFlowChild', 'absolute');").unwrap();
    project_addon(&mut app, &addon);
    assert_eq!(projected_frame(app.world_mut(), second), entity);
    assert_native_rect(app.world(), entity, [12.0, 8.0, 50.0, 20.0]);
    let frame = app
        .world()
        .resource::<UiState>()
        .registry
        .get(second)
        .unwrap();
    assert_eq!(frame.position.left, px(12));
    assert_eq!(frame.position.top, px(8));
    assert_eq!(frame.anchor, AnchorTarget::Parent);
}

#[test]
fn js_addon_script_emits_expected_operations() {
    let script = r#"
        addon.createFrame("MyPanel", "ParentRoot");
        addon.setSize("MyPanel", 240, 64);
        addon.setPos("MyPanel", 12, 6);
        addon.setPosType("MyPanel", "absolute");
        addon.setAnchor("MyPanel", "screen");
        addon.setAnchor("MyPanel");
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
            AddonOperation::SetPos {
                name: "MyPanel".to_string(),
                x: 12.0,
                y: 6.0,
            },
            AddonOperation::SetPosType {
                name: "MyPanel".to_string(),
                position_type: PositionType::Absolute,
            },
            AddonOperation::SetAnchor {
                name: "MyPanel".to_string(),
                target: AnchorTarget::Screen,
            },
            AddonOperation::SetAnchor {
                name: "MyPanel".to_string(),
                target: AnchorTarget::Parent,
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
            addon.setPos("MyLabel", 0, 0);
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
fn addon_position_updates_preserve_target_mode_and_logical_parent() {
    let operations = js::run_js_addon_to_operations(
        r#"
            addon.createFrame("SizedPanel", "ParentRoot");
            addon.setSize("SizedPanel", 100, 40);
            addon.setPosType("SizedPanel", "absolute");
            addon.setAnchor("SizedPanel", "screen");
            addon.setPos("SizedPanel", 10, 20);
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
    let panel = registry.get_by_name("SizedPanel").unwrap();
    let parent = registry.get_by_name("ParentRoot").unwrap();
    {
        let frame = registry.get_mut(panel).unwrap();
        frame.position.right = px(5);
        frame.position.bottom = px(7);
    }

    addon.operations = js::run_js_addon_to_operations(
        r#"
            addon.setSize("SizedPanel", 160, 70);
            addon.setPos("SizedPanel", -12, 23);
            addon.setSize("ParentRoot", 999, 999);
        "#,
    )
    .expect("resize script should parse");
    apply::apply_addon(&addon, &mut registry);
    let frame = registry.get(panel).unwrap();
    assert_eq!(frame.position.left, px(-12));
    assert_eq!(frame.position.top, px(23));
    assert_eq!(frame.position.right, Val::Auto);
    assert_eq!(frame.position.bottom, Val::Auto);
    assert_eq!(frame.position_type, PositionType::Absolute);
    assert_eq!(frame.anchor, AnchorTarget::Screen);
    assert_eq!(frame.parent_id, Some(parent));
    assert_eq!(frame.width, Dimension::Fixed(160.0));
    assert_eq!(frame.height, Dimension::Fixed(70.0));
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

fn addon_file(tag: &str, script: &str) -> (PathBuf, PathBuf) {
    let dir = std::env::temp_dir().join(format!("addon_{tag}_{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    let path = dir.join("main.js");
    std::fs::write(&path, script).unwrap();
    (dir, path)
}

fn reload_and_apply(app: &mut App, runtime: &mut AddonRuntime, path: &Path) {
    let mut ui = app.world_mut().resource_mut::<UiState>();
    runtime.reload_path(path.to_path_buf(), &mut ui.registry);
    runtime.apply(&mut ui.registry);
    app.update();
    app.update();
}

/// Run app updates until deferred despawn commands for `removed` are applied.
/// Returns false when entities survive four updates (commands never applied).
fn settle_removed(app: &mut App, removed: &[Entity]) -> bool {
    for _ in 0..4 {
        app.update();
        if removed
            .iter()
            .all(|entity| app.world().get_entity(*entity).is_err())
        {
            return true;
        }
    }
    false
}

/// Native entities still projecting removed registry frames (frame nodes and text parts).
/// Image parts use ui-toolkit-private `RegistryImage` and are covered by subtree capture.
fn stale_native_entities(world: &mut World, frame_ids: &[u64]) -> Vec<Entity> {
    let mut stale = Vec::new();
    let mut nodes = world.query::<(Entity, &ui_toolkit::native_render::RegistryNode)>();
    for (entity, node) in nodes.iter(world) {
        if frame_ids.contains(&node.0) {
            stale.push(entity);
        }
    }
    let mut texts = world.query::<(Entity, &ui_toolkit::native_render::RegistryText)>();
    for (entity, text) in texts.iter(world) {
        if frame_ids.contains(&text.frame_id) {
            stale.push(entity);
        }
    }
    stale
}

fn reparent_lifecycle_script(panel_anchor: &str, label_text: &str) -> String {
    format!(
        r#"
        addon.createFrame("ReparentPanel", "ParentRoot");
        addon.setBackgroundColor("ReparentPanel", 0.2, 0.3, 0.4, 1.0);
        addon.setSize("ReparentPanel", 120, 60);
        addon.setPosType("ReparentPanel", "absolute");
        addon.setPos("ReparentPanel", 30, 40);
        addon.createFontString("ReparentLabel", "ReparentPanel", "{label_text}");
        addon.setSize("ReparentLabel", 60, 20);
        {panel_anchor}
        "#
    )
}

fn reparent_removal_assertions(
    app: &mut App,
    removed_entities: &[Entity],
    removed_ids: &[u64],
    former_parent: Entity,
    context: &str,
) {
    assert!(
        settle_removed(app, removed_entities),
        "{context}: despawn commands were not applied"
    );
    for entity in removed_entities {
        assert!(
            app.world().get_entity(*entity).is_err(),
            "{context}: native entity {entity:?} survived removal"
        );
    }
    assert!(
        stale_native_entities(app.world_mut(), removed_ids).is_empty(),
        "{context}: native projection remains for removed registry frames"
    );
    assert!(
        app.world().get_entity(former_parent).is_ok(),
        "{context}: former native parent must survive"
    );
    let orphans: Vec<Entity> = native_descendants(app.world(), former_parent)
        .into_iter()
        .filter(|entity| removed_entities.contains(entity))
        .collect();
    assert!(
        orphans.is_empty(),
        "{context}: orphaned addon entities still attached to former native parent"
    );
}

#[test]
fn reload_after_screen_reparent_removes_full_native_subtree() {
    let mut app = native_addon_app();
    let script = reparent_lifecycle_script("", "Before");
    let (dir, path) = addon_file("reparent_reload", &script);
    let mut runtime = AddonRuntime {
        addon_dir: dir.clone(),
        addons: HashMap::new(),
    };
    reload_and_apply(&mut app, &mut runtime, &path);
    let (root, panel_id, label_id) = {
        let registry = &app.world().resource::<UiState>().registry;
        (
            registry.get_by_name("ParentRoot").unwrap(),
            registry.get_by_name("ReparentPanel").unwrap(),
            registry.get_by_name("ReparentLabel").unwrap(),
        )
    };
    let root_entity = projected_frame(app.world_mut(), root);
    let panel_entity = projected_frame(app.world_mut(), panel_id);
    let label_entity = projected_frame(app.world_mut(), label_id);
    let text = projected_text(app.world(), label_entity);
    assert_eq!(
        app.world().get::<ChildOf>(panel_entity).unwrap().parent(),
        root_entity,
        "fixture panel starts anchored to its parent"
    );

    // Reparent the projection to screen scope through the addon operation path.
    let mut tweak = script_addon(&script);
    tweak.operations =
        js::run_js_addon_to_operations("addon.setAnchor('ReparentPanel', 'screen');").unwrap();
    apply::apply_addon(
        &tweak,
        &mut app.world_mut().resource_mut::<UiState>().registry,
    );
    app.update();
    app.update();
    assert_eq!(
        projected_frame(app.world_mut(), panel_id),
        panel_entity,
        "reparent preserves projection identity"
    );
    let former_parent = app.world().get::<ChildOf>(panel_entity).unwrap().parent();
    assert_ne!(
        former_parent, root_entity,
        "screen anchor reparents natively"
    );

    let removed_entities = native_descendants(app.world(), panel_entity);
    assert!(
        removed_entities.contains(&label_entity) && removed_entities.contains(&text),
        "fixture subtree covers frame, images and text"
    );

    std::fs::write(
        &path,
        "addon.createFrame('KeptPanel', 'ParentRoot');\naddon.setSize('KeptPanel', 40, 40);",
    )
    .unwrap();
    reload_and_apply(&mut app, &mut runtime, &path);
    assert_eq!(
        app.world()
            .resource::<UiState>()
            .registry
            .get_by_name("KeptPanel")
            .is_some(),
        true,
        "reloaded addon applied"
    );
    assert_eq!(
        app.world()
            .resource::<UiState>()
            .registry
            .get_by_name("ReparentPanel"),
        None
    );
    reparent_removal_assertions(
        &mut app,
        &removed_entities,
        &[panel_id, label_id],
        former_parent,
        "reload after screen reparent",
    );
    std::fs::remove_dir_all(&dir).ok();
}

#[test]
fn unload_after_parent_reparent_removes_full_native_subtree() {
    let mut app = native_addon_app();
    let script =
        reparent_lifecycle_script("addon.setAnchor(\"ReparentPanel\", \"screen\");", "Before");
    let (dir, path) = addon_file("reparent_unload", &script);
    let mut runtime = AddonRuntime {
        addon_dir: dir.clone(),
        addons: HashMap::new(),
    };
    reload_and_apply(&mut app, &mut runtime, &path);
    let (root, panel_id, label_id) = {
        let registry = &app.world().resource::<UiState>().registry;
        (
            registry.get_by_name("ParentRoot").unwrap(),
            registry.get_by_name("ReparentPanel").unwrap(),
            registry.get_by_name("ReparentLabel").unwrap(),
        )
    };
    let root_entity = projected_frame(app.world_mut(), root);
    let panel_entity = projected_frame(app.world_mut(), panel_id);
    let label_entity = projected_frame(app.world_mut(), label_id);
    let text = projected_text(app.world(), label_entity);
    let former_parent = app.world().get::<ChildOf>(panel_entity).unwrap().parent();
    assert_ne!(
        former_parent, root_entity,
        "fixture panel starts anchored to screen scope"
    );

    // Switch the projection back under its logical parent through the addon path.
    let mut tweak = script_addon(&script);
    tweak.operations = js::run_js_addon_to_operations("addon.setAnchor('ReparentPanel');").unwrap();
    apply::apply_addon(
        &tweak,
        &mut app.world_mut().resource_mut::<UiState>().registry,
    );
    app.update();
    app.update();
    assert_eq!(
        app.world().get::<ChildOf>(panel_entity).unwrap().parent(),
        root_entity,
        "parent anchor reparents natively"
    );
    let removed_entities = native_descendants(app.world(), panel_entity);
    assert!(removed_entities.contains(&label_entity));
    assert!(removed_entities.contains(&text));

    {
        let mut ui = app.world_mut().resource_mut::<UiState>();
        runtime.unload_path(&path, &mut ui.registry);
    }
    assert_eq!(
        app.world()
            .resource::<UiState>()
            .registry
            .get_by_name("ReparentLabel"),
        None
    );
    reparent_removal_assertions(
        &mut app,
        &removed_entities,
        &[panel_id, label_id],
        root_entity,
        "unload after parent reparent",
    );
    std::fs::remove_dir_all(&dir).ok();
}

#[test]
fn reload_recreated_frames_keep_stable_native_identity_after_deferred_removal() {
    let mut app = native_addon_app();
    let (dir, path) = addon_file(
        "sweep_reload",
        r#"
        addon.createFrame("SweepPanel", "ParentRoot");
        addon.setBackgroundColor("SweepPanel", 0.1, 0.1, 0.1, 1.0);
        addon.setSize("SweepPanel", 100, 50);
        addon.createFontString("SweepLabel", "SweepPanel", "One");
        addon.setSize("SweepLabel", 50, 20);
        addon.createFrame("SweepExtra", "ParentRoot");
        addon.setBackgroundColor("SweepExtra", 0.9, 0.1, 0.1, 1.0);
        addon.setSize("SweepExtra", 30, 30);
        "#,
    );
    let mut runtime = AddonRuntime {
        addon_dir: dir.clone(),
        addons: HashMap::new(),
    };
    reload_and_apply(&mut app, &mut runtime, &path);
    let old_ids: Vec<u64> = {
        let registry = &app.world().resource::<UiState>().registry;
        ["SweepPanel", "SweepLabel", "SweepExtra"]
            .iter()
            .map(|name| registry.get_by_name(name).unwrap())
            .collect()
    };
    let mut removed_entities = Vec::new();
    for id in &old_ids {
        let entity = projected_frame(app.world_mut(), *id);
        removed_entities.extend(native_descendants(app.world(), entity));
    }

    std::fs::write(
        &path,
        r#"
        addon.createFrame("SweepPanel", "ParentRoot");
        addon.setBackgroundColor("SweepPanel", 0.1, 0.1, 0.1, 1.0);
        addon.setSize("SweepPanel", 100, 50);
        addon.createFontString("SweepLabel", "SweepPanel", "Two");
        addon.setSize("SweepLabel", 50, 20);
        "#,
    )
    .unwrap();
    {
        let mut ui = app.world_mut().resource_mut::<UiState>();
        runtime.reload_path(path.clone(), &mut ui.registry);
        runtime.apply(&mut ui.registry);
    }
    assert!(
        settle_removed(&mut app, &removed_entities),
        "reload despawn commands were not applied"
    );
    assert!(
        stale_native_entities(app.world_mut(), &old_ids).is_empty(),
        "no native entities may remain for removed registry frames"
    );

    let (panel_id, label_id) = {
        let registry = &app.world().resource::<UiState>().registry;
        (
            registry.get_by_name("SweepPanel").unwrap(),
            registry.get_by_name("SweepLabel").unwrap(),
        )
    };
    let panel_entity = projected_frame(app.world_mut(), panel_id);
    let text = projected_text(app.world(), panel_entity);
    for _ in 0..3 {
        app.update();
    }
    {
        let registry = &app.world().resource::<UiState>().registry;
        assert_eq!(registry.get_by_name("SweepPanel"), Some(panel_id));
        assert_eq!(registry.get_by_name("SweepLabel"), Some(label_id));
    }
    assert_eq!(projected_frame(app.world_mut(), panel_id), panel_entity);
    assert_eq!(projected_text(app.world(), panel_entity), text);
    std::fs::remove_dir_all(&dir).ok();
}

fn editbox_addon(names: &[&str]) -> LoadedAddon {
    LoadedAddon {
        name: "editbox-fixture".into(),
        operations: Vec::new(),
        owned_frames: names.iter().map(|name| (*name).to_string()).collect(),
    }
}

fn create_addon_editbox(app: &mut App, addon: &LoadedAddon, name: &str, x: f32, y: f32) -> u64 {
    let id = {
        let mut ui = app.world_mut().resource_mut::<UiState>();
        let id = apply::ensure_owned_frame(
            addon,
            &mut ui.registry,
            name,
            Some("ParentRoot"),
            WidgetType::EditBox,
        )
        .expect("owned editbox should be created");
        let frame = ui.registry.get_mut(id).unwrap();
        frame.width = Dimension::Fixed(120.0);
        frame.height = Dimension::Fixed(30.0);
        if let Some(WidgetData::EditBox(eb)) = frame.widget_data.as_mut() {
            eb.text = "typed".into();
        }
        ui.registry
            .set_pos_type(id, PositionType::Absolute)
            .unwrap();
        ui.registry.set_pos(id, x, y).unwrap();
        id
    };
    app.update();
    app.update();
    app.update();
    id
}

/// Registry input routing mirroring scenes/login/input.rs::handle_mouse_press:
/// hit-test through the registry, click the hit frame, mirror registry focus into
/// UiState; a miss clears the mirrored focus.
fn click_at(ui: &mut UiState, x: f32, y: f32) -> Option<u64> {
    let hit = ui_toolkit::input::find_frame_at(&ui.registry, x, y);
    match hit {
        Some(id) => {
            ui.registry.click_frame(id);
            ui.focused_frame = ui.registry.focused_frame;
        }
        None => ui.focused_frame = None,
    }
    hit
}

fn editbox_center(ui: &UiState, id: u64) -> (f32, f32) {
    let frame = ui.registry.get(id).unwrap();
    let rect = frame.layout_rect.as_ref().expect("editbox layout computed");
    (rect.x + rect.width / 2.0, rect.y + rect.height / 2.0)
}

#[test]
fn addon_owned_editbox_click_focuses_via_registry_input() {
    let mut app = native_addon_app();
    let addon = editbox_addon(&["AddonInput"]);
    let editbox = create_addon_editbox(&mut app, &addon, "AddonInput", 50.0, 40.0);
    let (cx, cy) = {
        let ui = app.world().resource::<UiState>();
        assert!(
            ui.registry.get(editbox).unwrap().is_editbox(),
            "addon-owned editbox must carry editbox widget data"
        );
        editbox_center(ui, editbox)
    };
    let mut ui = app.world_mut().resource_mut::<UiState>();
    let hit = click_at(&mut ui, cx, cy);
    assert_eq!(hit, Some(editbox), "hit-test finds the addon editbox");
    assert_eq!(ui.registry.focused_frame, Some(editbox));
    assert_eq!(ui.focused_frame, Some(editbox));
    let frame = ui.registry.get(editbox).unwrap();
    let Some(WidgetData::EditBox(eb)) = frame.widget_data.as_ref() else {
        panic!("editbox widget data missing");
    };
    assert_eq!(
        eb.cursor_position,
        eb.text.len(),
        "clicking an editbox selects all text (cursor to end)"
    );
}

#[test]
fn addon_editbox_focus_redirects_and_clears_on_outside_click() {
    let mut app = native_addon_app();
    let addon = editbox_addon(&["AddonInputA", "AddonInputB"]);
    let first = create_addon_editbox(&mut app, &addon, "AddonInputA", 50.0, 40.0);
    let second = create_addon_editbox(&mut app, &addon, "AddonInputB", 50.0, 90.0);
    let (first_center, second_center) = {
        let ui = app.world().resource::<UiState>();
        (editbox_center(ui, first), editbox_center(ui, second))
    };

    {
        let mut ui = app.world_mut().resource_mut::<UiState>();
        assert_eq!(
            click_at(&mut ui, first_center.0, first_center.1),
            Some(first)
        );
        assert_eq!(ui.registry.focused_frame, Some(first));
        assert_eq!(ui.focused_frame, Some(first));
    }
    {
        let mut ui = app.world_mut().resource_mut::<UiState>();
        assert_eq!(
            click_at(&mut ui, second_center.0, second_center.1),
            Some(second),
            "a second editbox receives the click"
        );
        assert_eq!(
            ui.registry.focused_frame,
            Some(second),
            "focus redirects to the clicked editbox"
        );
        assert_eq!(ui.focused_frame, Some(second));
    }
    {
        let mut ui = app.world_mut().resource_mut::<UiState>();
        assert_eq!(
            ui_toolkit::input::find_frame_at(&ui.registry, 700.0, 560.0),
            None,
            "click outside every frame hits nothing"
        );
        assert_eq!(click_at(&mut ui, 700.0, 560.0), None);
        assert_eq!(
            ui.focused_frame, None,
            "outside click clears the mirrored focus"
        );
    }
}
