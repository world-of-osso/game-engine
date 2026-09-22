use super::super::*;
use crate::ui::frame::WidgetData;
use crate::ui::registry::FrameRegistry;
use crate::ui::widgets::texture::TextureSource;
use ui_toolkit::screen::{Screen, SharedContext};

#[path = "../menu_character_layout_test_support.rs"]
mod layout_support;

fn projected_screen(mode: CharCreateMode) -> FrameRegistry {
    let mut registry = FrameRegistry::new(1920.0, 1080.0);
    let mut shared = SharedContext::new();
    let state = CharCreateUiState {
        mode,
        ..Default::default()
    };
    shared.insert(state);
    Screen::new(char_create_screen).sync(&shared, &mut registry);
    layout_support::compute_layout(&mut registry);
    registry
}

fn atlas(registry: &FrameRegistry, name: &str) -> String {
    let id = registry
        .get_by_name(name)
        .unwrap_or_else(|| panic!("missing {name}"));
    let frame = registry.get(id).unwrap();
    match &frame.widget_data {
        Some(WidgetData::Texture(texture)) => match &texture.source {
            TextureSource::Atlas(atlas) => atlas.clone(),
            other => panic!("{name} uses {other:?}, expected atlas"),
        },
        other => panic!("{name} is {other:?}, expected texture"),
    }
}

#[cfg(feature = "casc")]
#[test]
fn authored_navigation_parts_project_the_original_local_casc_pixels() {
    use bevy::ecs::system::RunSystemOnce;
    use bevy::prelude::*;
    use std::path::Path;
    use ui_toolkit::native_render::RegistryNode;
    use ui_toolkit::plugin::UiState;
    use ui_toolkit::render_texture::{BlpLoader, BlpLoaderRes};

    struct LocalBlpLoader;
    impl BlpLoader for LocalBlpLoader {
        fn load_blp_to_image(&self, path: &Path) -> Result<Image, String> {
            crate::asset::blp::load_blp_to_image(path)
        }
        fn load_blp_gpu_image(&self, path: &Path) -> Result<Image, String> {
            crate::asset::blp::load_blp_gpu_image(path)
        }
        fn ensure_texture(&self, fdid: u32) -> Option<std::path::PathBuf> {
            crate::asset::asset_cache::texture(fdid)
        }
    }

    fn assert_crop(app: &mut App, name: &str, source: &Image) {
        let region = ui_toolkit::atlas::get_region(&atlas(
            &app.world().resource::<UiState>().registry,
            name,
        ))
        .expect("supported Retail atlas part");
        let id = app
            .world()
            .resource::<UiState>()
            .registry
            .get_by_name(name)
            .unwrap();
        let entity = app
            .world_mut()
            .query::<(Entity, &RegistryNode)>()
            .iter(app.world())
            .find(|(_, node)| node.0 == id)
            .expect("native frame")
            .0;
        let images: Vec<_> = app
            .world_mut()
            .query::<(&ChildOf, &ImageNode)>()
            .iter(app.world())
            .filter(|(parent, _)| parent.parent() == entity)
            .map(|(_, image)| image.image.clone())
            .collect();
        assert_eq!(images.len(), 1, "{name}: exactly one projected atlas part");
        let assets = app.world().resource::<Assets<Image>>();
        let painted = assets
            .get(&images[0])
            .expect("part image loaded from local CASC");
        let x0 = (region.left * source.width() as f32).round() as usize;
        let x1 = (region.right * source.width() as f32).round() as usize;
        let y0 = (region.top * source.height() as f32).round() as usize;
        let y1 = (region.bottom * source.height() as f32).round() as usize;
        assert_eq!(
            (painted.width(), painted.height()),
            ((x1 - x0) as u32, (y1 - y0) as u32)
        );
        let bytes = source.data.as_ref().expect("decoded RGBA source pixels");
        let stride = source.width() as usize;
        let mut original = Vec::with_capacity((x1 - x0) * (y1 - y0) * 4);
        for y in y0..y1 {
            original.extend_from_slice(&bytes[(y * stride + x0) * 4..(y * stride + x1) * 4]);
        }
        for pixel in original.chunks_exact_mut(4) {
            if pixel[3] == 0 {
                pixel[..3].fill(0);
            }
        }
        assert_eq!(
            painted.data.as_deref(),
            Some(original.as_slice()),
            "{name}: exact original atlas pixels"
        );
        assert!(
            original.chunks_exact(4).any(|pixel| pixel[3] > 0),
            "{name}: not blank"
        );
    }

    let path =
        crate::asset::asset_cache::texture(7_367_529).expect("authored Retail atlas in local CASC");
    let source = crate::asset::blp::load_blp_to_image(&path).expect("decodable atlas");
    let mut app = layout_support::layout_app(1920.0, 1080.0);
    app.insert_resource(BlpLoaderRes(Box::new(LocalBlpLoader)));
    app.finish();
    app.cleanup();
    app.world_mut().resource_mut::<UiState>().registry =
        projected_screen(CharCreateMode::RaceClass);
    for _ in 0..3 {
        app.update();
    }
    for suffix in ["Left", "Center", "Right"] {
        assert_crop(&mut app, &format!("CharCreateBack_{suffix}"), &source);
    }
    let label_id = app
        .world()
        .resource::<UiState>()
        .registry
        .get_by_name("CharCreateBack_Label")
        .unwrap();
    let text_entity = app
        .world_mut()
        .query::<(Entity, &ui_toolkit::native_render::RegistryText)>()
        .iter(app.world())
        .find(|(_, text)| text.frame_id == label_id)
        .expect("navigation label projects to native UI text")
        .0;
    let text_bounds = app.world().get::<ChildOf>(text_entity).unwrap().parent();
    let text_z = app.world().get::<GlobalZIndex>(text_bounds).unwrap().0;
    for suffix in ["Left", "Center", "Right"] {
        let part_name = format!("CharCreateBack_{suffix}");
        let part_id = app
            .world()
            .resource::<UiState>()
            .registry
            .get_by_name(&part_name)
            .unwrap();
        let part_entity = app
            .world_mut()
            .query::<(Entity, &RegistryNode)>()
            .iter(app.world())
            .find(|(_, node)| node.0 == part_id)
            .unwrap()
            .0;
        let image_entity = app
            .world_mut()
            .query::<(Entity, &ChildOf, &ImageNode)>()
            .iter(app.world())
            .find(|(_, parent, _)| parent.parent() == part_entity)
            .unwrap()
            .0;
        let art_z = app.world().get::<GlobalZIndex>(image_entity).unwrap().0;
        assert!(
            text_z > art_z,
            "{part_name}: navigation label z={text_z} must appear above art z={art_z}"
        );
    }
    let id = app
        .world()
        .resource::<UiState>()
        .registry
        .get_by_name(BACK_BUTTON.0)
        .unwrap();
    if let Some(WidgetData::Button(button)) = &mut app
        .world_mut()
        .resource_mut::<UiState>()
        .registry
        .get_mut(id)
        .unwrap()
        .widget_data
    {
        button.highlight_locked = true;
    }
    app.world_mut()
        .run_system_once(super::sync_navigation_art)
        .unwrap();
    for _ in 0..2 {
        app.update();
    }
    assert_crop(&mut app, "CharCreateBack_Highlight", &source);
    let highlight_id = app
        .world()
        .resource::<UiState>()
        .registry
        .get_by_name("CharCreateBack_Highlight")
        .unwrap();
    let highlight_frame = app
        .world_mut()
        .query::<(Entity, &RegistryNode)>()
        .iter(app.world())
        .find(|(_, node)| node.0 == highlight_id)
        .unwrap()
        .0;
    let highlight_image = app
        .world_mut()
        .query::<(Entity, &ChildOf, &ImageNode)>()
        .iter(app.world())
        .find(|(_, parent, _)| parent.parent() == highlight_frame)
        .unwrap()
        .0;
    let highlight_z = app.world().get::<GlobalZIndex>(highlight_image).unwrap().0;
    assert!(
        text_z > highlight_z,
        "navigation label must stay above the hover overlay"
    );
}

#[test]
fn navigation_parts_follow_button_state_without_idle_registry_writes() {
    use crate::ui::event::EventBus;
    use crate::ui::plugin::UiState;
    use crate::ui::widgets::button::ButtonState;
    use bevy::ecs::system::RunSystemOnce;
    use bevy::prelude::World;

    let registry = projected_screen(CharCreateMode::RaceClass);
    let mut world = World::new();
    world.insert_resource(UiState {
        registry,
        event_bus: EventBus::new(),
        focused_frame: None,
    });
    let back = world
        .resource::<UiState>()
        .registry
        .get_by_name(BACK_BUTTON.0)
        .unwrap();
    {
        let mut ui = world.resource_mut::<UiState>();
        let Some(WidgetData::Button(button)) = &mut ui.registry.get_mut(back).unwrap().widget_data
        else {
            panic!("navigation must remain interactive")
        };
        button.state = ButtonState::Pushed;
    }
    world.run_system_once(super::sync_navigation_art).unwrap();
    let reg = &world.resource::<UiState>().registry;
    let label_id = reg
        .get_by_name("CharCreateBack_Label")
        .expect("visible navigation label");
    let Some(WidgetData::FontString(label)) = &reg.get(label_id).unwrap().widget_data else {
        panic!("navigation label is text")
    };
    assert_eq!(
        label.color,
        [0.8, 0.65, 0.0, 1.0],
        "pressed label color matches the original button"
    );
    for (part, expected) in [
        ("Left", "128-redbutton-left-pressed"),
        ("Center", "_128-redbutton-center-pressed"),
        ("Right", "128-redbutton-right-pressed"),
    ] {
        assert_eq!(atlas(reg, &format!("{}_{}", BACK_BUTTON.0, part)), expected);
    }
    assert!(
        reg.get(reg.get_by_name("CharCreateBack_Highlight").unwrap())
            .unwrap()
            .hidden
    );

    {
        let mut ui = world.resource_mut::<UiState>();
        let Some(WidgetData::Button(button)) = &mut ui.registry.get_mut(back).unwrap().widget_data
        else {
            unreachable!()
        };
        button.state = ButtonState::Normal;
        button.hovered = true;
    }
    world.run_system_once(super::sync_navigation_art).unwrap();
    let reg = &world.resource::<UiState>().registry;
    assert_eq!(atlas(reg, "CharCreateBack_Left"), "128-redbutton-left");
    assert!(
        !reg.get(reg.get_by_name("CharCreateBack_Highlight").unwrap())
            .unwrap()
            .hidden
    );
    assert_eq!(
        atlas(reg, "CharCreateBack_Highlight"),
        "retail-128-redbutton-highlight"
    );

    {
        let mut ui = world.resource_mut::<UiState>();
        let Some(WidgetData::Button(button)) = &mut ui.registry.get_mut(back).unwrap().widget_data
        else {
            unreachable!()
        };
        button.state = ButtonState::Disabled;
    }
    world.run_system_once(super::sync_navigation_art).unwrap();
    let reg = &world.resource::<UiState>().registry;
    assert_eq!(
        atlas(reg, "CharCreateBack_Right"),
        "128-redbutton-right-disabled"
    );
    let label_id = reg.get_by_name("CharCreateBack_Label").unwrap();
    let Some(WidgetData::FontString(label)) = &reg.get(label_id).unwrap().widget_data else {
        panic!("navigation label is text")
    };
    assert_eq!(label.color, [0.5, 0.5, 0.5, 1.0]);
    assert!(
        reg.get(reg.get_by_name("CharCreateBack_Highlight").unwrap())
            .unwrap()
            .hidden
    );

    world
        .resource_mut::<UiState>()
        .registry
        .render_dirty
        .clear();
    world.run_system_once(super::sync_navigation_art).unwrap();
    let mut ui = world.resource_mut::<UiState>();
    ui.registry.resolve_pending_writes();
    assert!(
        ui.registry.render_dirty.is_empty(),
        "settled navigation art must not publish idle writes"
    );
}

#[test]
fn character_create_navigation_uses_three_authored_red_parts_without_square_skin() {
    let scale = 66.0 / 128.0;
    let widths = [
        114.0 * scale,
        250.0 - (114.0 + 292.0) * scale,
        292.0 * scale,
    ];
    for (mode, forward, forward_action, forward_label) in [
        (
            CharCreateMode::RaceClass,
            NEXT_BUTTON.0,
            "next_mode",
            "Customize",
        ),
        (
            CharCreateMode::Customize,
            CREATE_BUTTON.0,
            "create_confirm",
            "Create Character",
        ),
    ] {
        let registry = projected_screen(mode);
        for (name, x, action, label) in [
            (BACK_BUTTON.0, 46.0, "back", "Back"),
            (forward, 1624.0, forward_action, forward_label),
        ] {
            let id = registry.get_by_name(name).expect("navigation button");
            let frame = registry.get(id).unwrap();
            assert_eq!(frame.onclick.as_deref(), Some(action));
            assert_eq!(
                frame.nine_slice, None,
                "{name}: implicit nine-slice must not cover the authored parts"
            );
            let Some(WidgetData::Button(button)) = &frame.widget_data else {
                panic!("{name} must remain a button")
            };
            assert!(
                !button.use_default_skin,
                "{name}: authored art must not gain a square default skin"
            );
            assert!(button.normal_texture.is_none());
            let label_id = registry
                .get_by_name(&format!("{name}_Label"))
                .expect("visible navigation wording");
            let Some(WidgetData::FontString(text)) = &registry.get(label_id).unwrap().widget_data
            else {
                panic!("navigation wording must render as text")
            };
            assert_eq!(text.text, label);
            assert_eq!(
                text.font,
                crate::ui::widgets::font_string::GameFont::FrizQuadrata
            );
            assert_eq!(text.font_size, 22.0);
            assert_eq!(text.color, [1.0, 0.82, 0.0, 1.0]);
            assert_eq!(
                text.justify_h,
                crate::ui::widgets::font_string::JustifyH::Center
            );
            assert_eq!(
                text.justify_v,
                crate::ui::widgets::font_string::JustifyV::Middle
            );
            let mut left = x;
            for (suffix, source, width) in [
                ("Left", "128-redbutton-left", widths[0]),
                ("Center", "_128-redbutton-center", widths[1]),
                ("Right", "128-redbutton-right", widths[2]),
            ] {
                let child = format!("{name}_{suffix}");
                assert_eq!(atlas(&registry, &child), source);
                let part = registry.get(registry.get_by_name(&child).unwrap()).unwrap();
                let rect = part.layout_rect.as_ref().expect("native part bounds");
                assert!(
                    (part.width.value() - width).abs() < 0.001,
                    "{child}: authored width must retain the atlas cap ratio"
                );
                // Bevy rounds each fractional edge for rasterization; authored cap width is exact above.
                assert!(
                    (rect.x - left).abs() < 1.0,
                    "{child}: x={} expected {left}",
                    rect.x
                );
                assert!((rect.y - 986.0).abs() < 1.0);
                assert!(
                    (rect.width - width).abs() < 1.0,
                    "{child}: width={} expected {width}",
                    rect.width
                );
                assert!((rect.height - 66.0).abs() < 1.0);
                left += width;
            }
            assert!((left - (x + 250.0)).abs() < 0.02);
        }
    }
}
