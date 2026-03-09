//! Text shadow and outline rendering.

use bevy::camera::visibility::RenderLayers;
use bevy::prelude::*;
use bevy::text::Font;
use bevy::text::TextFont;
use std::collections::{HashMap, HashSet};

use crate::ui::frame::WidgetData;
use crate::ui::plugin::UiState;
use crate::ui::widgets::font_string::{JustifyH, JustifyV, Outline};

use super::render::{UI_RENDER_LAYER, UiText};

/// Marker for shadow text entities.
#[derive(Component)]
pub struct UiTextShadow(pub u64);

/// Marker for outline text entities.
#[derive(Component)]
pub struct UiTextOutline(pub u64);

/// Syncs text shadows — a dark copy of text rendered behind the main text.
pub fn sync_ui_text_shadows(
    state: Res<UiState>,
    mut commands: Commands,
    mut font_assets: ResMut<Assets<Font>>,
    mut font_cache: Local<HashMap<String, Handle<Font>>>,
    mut missing_fonts: Local<HashSet<String>>,
    mut shadows: Query<(
        Entity,
        &UiTextShadow,
        &mut Text2d,
        &mut TextFont,
        &mut TextColor,
        &mut Transform,
    )>,
) {
    let screen_w = state.registry.screen_width;
    let screen_h = state.registry.screen_height;
    let mut existing: HashSet<u64> = HashSet::new();

    for (entity, shadow, mut text, mut font, mut color, mut transform) in shadows.iter_mut() {
        let Some(props) = extract_shadow(state.registry.get(shadow.0)) else {
            commands.entity(entity).despawn();
            continue;
        };
        if text.0 != props.content {
            commands.entity(entity).despawn();
            continue;
        }
        existing.insert(shadow.0);
        update_shadow_entity(
            &props,
            &mut text,
            &mut font,
            &mut color,
            &mut font_assets,
            &mut font_cache,
            &mut missing_fonts,
        );
        let _ = (&mut transform, screen_w, screen_h); // transform set at spawn
    }

    spawn_missing_shadows(
        &state,
        &existing,
        screen_w,
        screen_h,
        &mut commands,
        &mut font_assets,
        &mut font_cache,
        &mut missing_fonts,
    );
}

fn spawn_missing_shadows(
    state: &UiState,
    existing: &HashSet<u64>,
    screen_w: f32,
    screen_h: f32,
    commands: &mut Commands,
    font_assets: &mut Assets<Font>,
    font_cache: &mut HashMap<String, Handle<Font>>,
    missing_fonts: &mut HashSet<String>,
) {
    for frame in state.registry.frames_iter() {
        if existing.contains(&frame.id) {
            continue;
        }
        let Some(props) = extract_shadow(Some(frame)) else {
            continue;
        };
        let transform = shadow_transform(frame, &props, screen_w, screen_h);
        let [r, g, b, a] = props.shadow_color;
        let font = super::render_text::resolve_font_handle(
            &props.font,
            font_assets,
            font_cache,
            missing_fonts,
        )
        .unwrap_or_default();
        commands.spawn((
            Text2d::new(props.content),
            TextFont {
                font,
                font_size: props.font_size,
                ..default()
            },
            TextColor(Color::srgba(r, g, b, a * frame.effective_alpha)),
            transform,
            RenderLayers::layer(UI_RENDER_LAYER),
            UiText(frame.id),
            UiTextShadow(frame.id),
        ));
    }
}

struct ShadowProps {
    content: String,
    font: String,
    font_size: f32,
    shadow_color: [f32; 4],
    shadow_offset: [f32; 2],
    justify_h: JustifyH,
    justify_v: JustifyV,
}

fn extract_shadow(frame: Option<&crate::ui::frame::Frame>) -> Option<ShadowProps> {
    let frame = frame?;
    if !frame.visible {
        return None;
    }
    let Some(WidgetData::FontString(fs)) = &frame.widget_data else {
        return None;
    };
    if fs.text.is_empty() {
        return None;
    }
    let shadow_color = fs.shadow_color?;
    Some(ShadowProps {
        content: fs.text.clone(),
        font: fs.font.clone(),
        font_size: fs.font_size,
        shadow_color,
        shadow_offset: fs.shadow_offset,
        justify_h: fs.justify_h,
        justify_v: fs.justify_v,
    })
}

fn update_shadow_entity(
    props: &ShadowProps,
    text: &mut Text2d,
    font: &mut TextFont,
    color: &mut TextColor,
    font_assets: &mut Assets<Font>,
    font_cache: &mut HashMap<String, Handle<Font>>,
    missing_fonts: &mut HashSet<String>,
) {
    *text = Text2d::new(&props.content);
    font.font_size = props.font_size;
    if let Some(font_handle) =
        super::render_text::resolve_font_handle(&props.font, font_assets, font_cache, missing_fonts)
    {
        font.font = font_handle;
    }
    let [r, g, b, a] = props.shadow_color;
    *color = TextColor(Color::srgba(r, g, b, a));
}

fn shadow_transform(
    frame: &crate::ui::frame::Frame,
    props: &ShadowProps,
    screen_w: f32,
    screen_h: f32,
) -> Transform {
    let mut t = super::render_text::text_transform(
        frame,
        screen_w,
        screen_h,
        props.justify_h,
        props.justify_v,
    );
    t.translation.x += props.shadow_offset[0];
    t.translation.y -= props.shadow_offset[1];
    t.translation.z = 9.9;
    t
}

/// Syncs text outlines — dark copies of text at directional offsets.
pub fn sync_ui_text_outlines(
    state: Res<UiState>,
    mut commands: Commands,
    mut font_assets: ResMut<Assets<Font>>,
    mut font_cache: Local<HashMap<String, Handle<Font>>>,
    mut missing_fonts: Local<HashSet<String>>,
    outlines: Query<(Entity, &UiTextOutline, &Text2d)>,
) {
    let screen_w = state.registry.screen_width;
    let screen_h = state.registry.screen_height;

    let mut existing: HashSet<u64> = HashSet::new();
    for (entity, outline, text) in &outlines {
        if has_outline_frame(&state, outline.0) && outline_text_matches(&state, outline.0, &text.0)
        {
            existing.insert(outline.0);
        } else {
            commands.entity(entity).despawn();
        }
    }

    for frame in state.registry.frames_iter() {
        if !frame.visible || existing.contains(&frame.id) || !has_outline(frame) {
            continue;
        }
        spawn_outlines(
            frame,
            screen_w,
            screen_h,
            &mut commands,
            &mut font_assets,
            &mut font_cache,
            &mut missing_fonts,
        );
    }
}

fn has_outline_frame(state: &UiState, id: u64) -> bool {
    state
        .registry
        .get(id)
        .is_some_and(|f| f.visible && has_outline(f))
}

fn has_outline(frame: &crate::ui::frame::Frame) -> bool {
    matches!(
        &frame.widget_data,
        Some(WidgetData::FontString(fs)) if fs.outline != Outline::None && !fs.text.is_empty()
    )
}

fn outline_text_matches(state: &UiState, id: u64, text: &str) -> bool {
    state
        .registry
        .get(id)
        .is_some_and(|frame| match &frame.widget_data {
            Some(WidgetData::FontString(fs)) => fs.text == text,
            _ => false,
        })
}

fn spawn_outlines(
    frame: &crate::ui::frame::Frame,
    screen_w: f32,
    screen_h: f32,
    commands: &mut Commands,
    font_assets: &mut Assets<Font>,
    font_cache: &mut HashMap<String, Handle<Font>>,
    missing_fonts: &mut HashSet<String>,
) {
    let Some(WidgetData::FontString(fs)) = &frame.widget_data else {
        return;
    };
    let base =
        super::render_text::text_transform(frame, screen_w, screen_h, fs.justify_h, fs.justify_v);
    let alpha = frame.effective_alpha;
    let font =
        super::render_text::resolve_font_handle(&fs.font, font_assets, font_cache, missing_fonts)
            .unwrap_or_default();

    for &(dx, dy) in outline_offsets(fs.outline) {
        let mut transform = base;
        transform.translation.x += dx;
        transform.translation.y += dy;
        transform.translation.z = 9.8;
        commands.spawn((
            Text2d::new(&fs.text),
            TextFont {
                font: font.clone(),
                font_size: fs.font_size,
                ..default()
            },
            TextColor(Color::srgba(0.0, 0.0, 0.0, alpha)),
            transform,
            RenderLayers::layer(UI_RENDER_LAYER),
            UiText(frame.id),
            UiTextOutline(frame.id),
        ));
    }
}

fn outline_offsets(outline: Outline) -> &'static [(f32, f32)] {
    match outline {
        Outline::None => &[],
        Outline::Outline => &[(-1.0, 0.0), (1.0, 0.0), (0.0, -1.0), (0.0, 1.0)],
        Outline::ThickOutline => &[
            (-2.0, 0.0),
            (2.0, 0.0),
            (0.0, -2.0),
            (0.0, 2.0),
            (-1.4, -1.4),
            (1.4, -1.4),
            (-1.4, 1.4),
            (1.4, 1.4),
        ],
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ui::frame::WidgetData;
    use crate::ui::plugin::UiPlugin;
    use crate::ui::widgets::font_string::{FontStringData, Outline as FsOutline};

    fn make_test_app() -> App {
        let mut app = App::new();
        app.add_plugins(MinimalPlugins);
        app.insert_resource(Assets::<Font>::default());
        app.add_plugins(UiPlugin);
        app.update();
        app
    }

    fn make_font_string_frame(app: &mut App, name: &str, fs: FontStringData) {
        let mut ui = app.world_mut().resource_mut::<UiState>();
        let id = ui.registry.create_frame(name, None);
        let frame = ui.registry.get_mut(id).unwrap();
        frame.width = 100.0;
        frame.height = 20.0;
        frame.widget_data = Some(WidgetData::FontString(fs));
    }

    #[test]
    fn shadow_color_spawns_shadow_entity() {
        let mut app = make_test_app();
        make_font_string_frame(
            &mut app,
            "ShadowText",
            FontStringData {
                text: "Hello".into(),
                shadow_color: Some([0.0, 0.0, 0.0, 1.0]),
                ..Default::default()
            },
        );
        app.update();
        let mut q = app.world_mut().query_filtered::<(), With<UiTextShadow>>();
        assert_eq!(q.iter(app.world()).count(), 1);
    }

    #[test]
    fn no_shadow_color_spawns_no_shadow() {
        let mut app = make_test_app();
        make_font_string_frame(
            &mut app,
            "NoShadowText",
            FontStringData {
                text: "Hello".into(),
                shadow_color: None,
                ..Default::default()
            },
        );
        app.update();
        let mut q = app.world_mut().query_filtered::<(), With<UiTextShadow>>();
        assert_eq!(q.iter(app.world()).count(), 0);
    }

    #[test]
    fn outline_spawns_4_outline_entities() {
        let mut app = make_test_app();
        make_font_string_frame(
            &mut app,
            "OutlineText",
            FontStringData {
                text: "Hi".into(),
                outline: FsOutline::Outline,
                ..Default::default()
            },
        );
        app.update();
        let mut q = app.world_mut().query_filtered::<(), With<UiTextOutline>>();
        assert_eq!(q.iter(app.world()).count(), 4);
    }

    #[test]
    fn thick_outline_spawns_8_outline_entities() {
        let mut app = make_test_app();
        make_font_string_frame(
            &mut app,
            "ThickOutlineText",
            FontStringData {
                text: "Hi".into(),
                outline: FsOutline::ThickOutline,
                ..Default::default()
            },
        );
        app.update();
        let mut q = app.world_mut().query_filtered::<(), With<UiTextOutline>>();
        assert_eq!(q.iter(app.world()).count(), 8);
    }

    #[test]
    fn outline_entities_replace_text_when_source_changes() {
        let mut app = make_test_app();
        make_font_string_frame(
            &mut app,
            "OutlineText",
            FontStringData {
                text: "Old".into(),
                outline: FsOutline::Outline,
                ..Default::default()
            },
        );
        app.update();

        {
            let mut ui = app.world_mut().resource_mut::<UiState>();
            let frame_id = ui
                .registry
                .get_by_name("OutlineText")
                .expect("outline frame");
            let frame = ui.registry.get_mut(frame_id).expect("outline frame");
            let Some(WidgetData::FontString(fs)) = frame.widget_data.as_mut() else {
                panic!("expected font string");
            };
            fs.text = "New".into();
        }

        app.update();
        app.update();

        let rendered: Vec<_> = {
            let mut q = app.world_mut().query::<(&UiTextOutline, &Text2d)>();
            q.iter(app.world())
                .map(|(_, text)| text.0.clone())
                .collect()
        };
        assert_eq!(rendered.len(), 4);
        assert!(
            rendered.iter().all(|text| text == "New"),
            "outline texts were not refreshed: {rendered:?}"
        );
    }
}
