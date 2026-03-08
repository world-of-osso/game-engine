use bevy::prelude::*;

use crate::ui::frame::{WidgetData, WidgetType};
use crate::ui::plugin::{UiPlugin, UiState};
use crate::ui::render::{frame_sprite_params, texture_tint};
use crate::ui::render_nine_slice::UiNineSlicePart;
use crate::ui::render_text::extract_button_text;
use crate::ui::widgets::button::{ButtonData, ButtonState};
use crate::ui::widgets::edit_box::EditBoxData;
use crate::ui::widgets::font_string::FontStringData;
use crate::ui::widgets::texture::TextureSource;

fn setup_app() -> App {
    let mut app = App::new();
    app.add_plugins(MinimalPlugins);
    app.add_plugins(bevy::asset::AssetPlugin::default());
    app.init_asset::<Image>();
    app.init_asset::<bevy::text::Font>();
    app.add_plugins(UiPlugin);
    app.update();
    app
}

fn create_button(app: &mut App, name: &str, btn: ButtonData) -> u64 {
    let mut ui = app.world_mut().resource_mut::<UiState>();
    let id = ui.registry.create_frame(name, None);
    let frame = ui.registry.get_mut(id).unwrap();
    frame.width = 120.0;
    frame.height = 40.0;
    frame.widget_type = WidgetType::Button;
    frame.widget_data = Some(WidgetData::Button(btn));
    id
}

// --- Button nine-slice tests ---

#[test]
fn button_with_texture_gets_nine_slice() {
    let mut app = setup_app();
    let btn = ButtonData {
        normal_texture: Some(TextureSource::File("btn.blp".into())),
        ..Default::default()
    };
    let id = create_button(&mut app, "Btn", btn);
    app.update();
    let ui = app.world().resource::<UiState>();
    let frame = ui.registry.get(id).unwrap();
    assert!(frame.nine_slice.is_some(), "button with texture should get nine_slice");
}

#[test]
fn button_without_texture_no_nine_slice() {
    let mut app = setup_app();
    let id = create_button(&mut app, "BtnPlain", ButtonData::default());
    app.update();
    let ui = app.world().resource::<UiState>();
    let frame = ui.registry.get(id).unwrap();
    assert!(frame.nine_slice.is_none(), "button without texture should have no nine_slice");
}

#[test]
fn button_nine_slice_spawns_all_9_parts() {
    let mut app = setup_app();
    let btn = ButtonData {
        normal_texture: Some(TextureSource::File("btn.blp".into())),
        ..Default::default()
    };
    let id = create_button(&mut app, "Btn9", btn);
    app.update();
    let mut q = app.world_mut().query::<&UiNineSlicePart>();
    let parts: Vec<u8> = q
        .iter(app.world())
        .filter(|p| p.0 == id)
        .map(|p| p.1)
        .collect();
    assert_eq!(parts.len(), 9, "expected 9 nine-slice parts, got {}", parts.len());
    for i in 0..9u8 {
        assert!(parts.contains(&i), "missing nine-slice part {i}");
    }
}

#[test]
fn button_pushed_state_updates_nine_slice_texture() {
    let mut app = setup_app();
    let btn = ButtonData {
        state: ButtonState::Normal,
        normal_texture: Some(TextureSource::File("normal.blp".into())),
        pushed_texture: Some(TextureSource::File("pushed.blp".into())),
        ..Default::default()
    };
    let id = create_button(&mut app, "BtnPush", btn);
    app.update();
    // Switch to pushed state
    {
        let mut ui = app.world_mut().resource_mut::<UiState>();
        let frame = ui.registry.get_mut(id).unwrap();
        if let Some(WidgetData::Button(btn)) = &mut frame.widget_data {
            btn.state = ButtonState::Pushed;
        }
        frame.nine_slice = None; // clear so sync re-creates
    }
    app.update();
    let ui = app.world().resource::<UiState>();
    let frame = ui.registry.get(id).unwrap();
    let ns = frame.nine_slice.as_ref().expect("should have nine_slice");
    assert!(
        matches!(&ns.texture, Some(TextureSource::File(p)) if p == "pushed.blp"),
        "pushed state should use pushed texture"
    );
}

// --- EditBox sizing and font tests ---

#[test]
fn edit_box_preserves_dimensions() {
    let mut app = setup_app();
    let id = {
        let mut ui = app.world_mut().resource_mut::<UiState>();
        let id = ui.registry.create_frame("EditBox1", None);
        let frame = ui.registry.get_mut(id).unwrap();
        frame.width = 250.0;
        frame.height = 32.0;
        frame.widget_type = WidgetType::EditBox;
        frame.widget_data = Some(WidgetData::EditBox(EditBoxData::default()));
        id
    };
    app.update();
    let ui = app.world().resource::<UiState>();
    let frame = ui.registry.get(id).unwrap();
    assert_eq!(frame.width, 250.0);
    assert_eq!(frame.height, 32.0);
}

#[test]
fn edit_box_font_flows_to_text_props() {
    let mut frame = crate::ui::frame::Frame::new(1, None, WidgetType::EditBox);
    frame.width = 200.0;
    frame.height = 30.0;
    frame.effective_alpha = 1.0;
    frame.widget_data = Some(WidgetData::EditBox(EditBoxData {
        text: "hello".into(),
        font: "data/fonts/custom.ttf".into(),
        font_size: 18.0,
        ..Default::default()
    }));
    let props = crate::ui::render_text::extract_text_props_pub(&frame);
    assert_eq!(props.font, "data/fonts/custom.ttf");
    assert_eq!(props.font_size, 18.0);
    assert_eq!(props.content, "hello");
}

#[test]
fn edit_box_password_masks_text() {
    let mut frame = crate::ui::frame::Frame::new(1, None, WidgetType::EditBox);
    frame.effective_alpha = 1.0;
    frame.widget_data = Some(WidgetData::EditBox(EditBoxData {
        text: "secret".into(),
        password: true,
        ..Default::default()
    }));
    let props = crate::ui::render_text::extract_text_props_pub(&frame);
    assert_eq!(props.content, "******");
}

// --- Font propagation tests ---

#[test]
fn font_string_font_flows_to_text_props() {
    let mut frame = crate::ui::frame::Frame::new(1, None, WidgetType::FontString);
    frame.effective_alpha = 1.0;
    frame.widget_data = Some(WidgetData::FontString(FontStringData {
        text: "Title".into(),
        font: "data/fonts/friz.ttf".into(),
        font_size: 24.0,
        ..Default::default()
    }));
    let props = crate::ui::render_text::extract_text_props_pub(&frame);
    assert_eq!(props.font, "data/fonts/friz.ttf");
    assert_eq!(props.font_size, 24.0);
    assert_eq!(props.content, "Title");
}

#[test]
fn button_font_size_flows_to_text_props() {
    let btn = ButtonData {
        text: "Click".into(),
        font_size: 20.0,
        ..Default::default()
    };
    let props = extract_button_text(&btn, 1.0);
    assert_eq!(props.font_size, 20.0);
    assert_eq!(props.content, "Click");
}

// --- Alpha tests ---

#[test]
fn edit_box_alpha_applied_to_text_color() {
    let mut frame = crate::ui::frame::Frame::new(1, None, WidgetType::EditBox);
    frame.effective_alpha = 0.4;
    frame.widget_data = Some(WidgetData::EditBox(EditBoxData {
        text: "faded".into(),
        text_color: [1.0, 1.0, 1.0, 1.0],
        ..Default::default()
    }));
    let props = crate::ui::render_text::extract_text_props_pub(&frame);
    let Color::Srgba(srgba) = props.color else {
        panic!("expected srgba");
    };
    assert!((srgba.alpha - 0.4).abs() < 0.001, "alpha should be 0.4, got {}", srgba.alpha);
}

#[test]
fn button_alpha_applied_to_text_color() {
    let btn = ButtonData {
        text: "Test".into(),
        ..Default::default()
    };
    let props = extract_button_text(&btn, 0.3);
    let Color::Srgba(srgba) = props.color else {
        panic!("expected srgba");
    };
    assert!((srgba.alpha - 0.3).abs() < 0.001, "alpha should be 0.3, got {}", srgba.alpha);
}

#[test]
fn texture_tint_applies_effective_alpha() {
    let mut frame = crate::ui::frame::Frame::new(1, None, WidgetType::Texture);
    frame.effective_alpha = 0.6;
    frame.widget_data = Some(WidgetData::Texture(
        crate::ui::widgets::texture::TextureData {
            vertex_color: [0.8, 0.5, 0.3, 1.0],
            ..Default::default()
        },
    ));
    let color = texture_tint(&frame);
    let Color::Srgba(srgba) = color else {
        panic!("expected srgba");
    };
    assert!((srgba.red - 0.8).abs() < 0.001);
    assert!((srgba.green - 0.5).abs() < 0.001);
    assert!((srgba.blue - 0.3).abs() < 0.001);
    assert!((srgba.alpha - 0.6).abs() < 0.001);
}

// --- Sprite sizing tests ---

#[test]
fn frame_sprite_params_uses_full_dimensions() {
    let mut frame = crate::ui::frame::Frame::new(1, None, WidgetType::Frame);
    frame.width = 200.0;
    frame.height = 100.0;
    let (size, offset) = frame_sprite_params(&frame);
    assert_eq!(size, Vec2::new(200.0, 100.0));
    assert_eq!(offset, Vec2::ZERO);
}
