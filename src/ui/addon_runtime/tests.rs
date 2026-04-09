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
        watcher: None,
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
