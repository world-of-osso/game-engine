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
