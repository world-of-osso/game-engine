//! Real local-CASC atlas crops through the native registry projection.
use super::{GameBlpLoader, layout_support};
use bevy::prelude::*;
use ui_toolkit::frame::{Dimension, WidgetData, WidgetType};
use ui_toolkit::native_render::RegistryNode;
use ui_toolkit::plugin::UiState;
use ui_toolkit::render_texture::{BlpLoader, BlpLoaderRes};
use ui_toolkit::widgets::texture::{TextureData, TextureSource};

// Pixel rectangles of the existing authored UV regions on the local 2048x2048 atlas.
const CUSTOMIZATION_CROPS: &[(&str, [usize; 4])] = &[
    ("charactercreate-customize-backbutton", [1723, 419, 76, 76]),
    (
        "charactercreate-customize-backbutton-down",
        [1881, 419, 76, 76],
    ),
    (
        "charactercreate-customize-backbutton-disabled",
        [1801, 419, 76, 76],
    ),
    ("charactercreate-customize-nextbutton", [1959, 419, 76, 76]),
    (
        "charactercreate-customize-nextbutton-down",
        [359, 1881, 76, 76],
    ),
    (
        "charactercreate-customize-nextbutton-disabled",
        [281, 1881, 76, 76],
    ),
    ("charactercreate-customize-palette", [1923, 213, 84, 20]),
    (
        "charactercreate-customize-palette-selected",
        [1819, 213, 102, 40],
    ),
];

fn load_original_customization_atlas() -> Image {
    let resolver = game_engine::asset::asset_resolver::resolver();
    let fdid = 1_253_496;
    assert_eq!(
        resolver.lookup_path("Interface/GLUES/CHARACTERCREATE/CharacterCreate.BLP"),
        Some(fdid)
    );
    let bytes = resolver
        .resolve_bytes(fdid)
        .expect("local CASC atlas bytes");
    let loader = GameBlpLoader;
    let path = loader.ensure_texture(fdid).expect("resolved atlas cache");
    assert_eq!(std::fs::read(&path).unwrap(), bytes);
    let atlas = loader.load_blp_to_image(&path).expect("decoded atlas");
    assert_eq!(atlas.size(), UVec2::new(2048, 2048));
    assert_eq!(atlas.data.as_ref().unwrap().len(), 2048 * 2048 * 4);
    atlas
}

fn expected_crop(atlas: &Image, [x, y, width, height]: [usize; 4]) -> Vec<u8> {
    let source = atlas.data.as_ref().unwrap();
    let mut pixels = Vec::with_capacity(width * height * 4);
    for row in y..y + height {
        let start = (row * atlas.width() as usize + x) * 4;
        for pixel in source[start..start + width * 4].chunks_exact(4) {
            // Invisible RGB is deliberately zeroed when atlas crops are materialized.
            pixels.extend_from_slice(if pixel[3] == 0 { &[0, 0, 0, 0] } else { pixel });
        }
    }
    assert!(pixels.chunks_exact(4).any(|pixel| pixel[3] > 0));
    assert!(pixels.chunks_exact(4).any(|pixel| pixel != &pixels[..4]));
    pixels
}

#[test]
fn charcreate_customization_atlas_projects_original_casc_crops() {
    let atlas = load_original_customization_atlas();
    let mut app = layout_support::layout_app(1920.0, 1080.0);
    app.insert_resource(BlpLoaderRes(Box::new(GameBlpLoader)));
    app.finish();
    app.cleanup();
    let frames: Vec<_> = {
        let mut ui = app.world_mut().resource_mut::<UiState>();
        CUSTOMIZATION_CROPS
            .iter()
            .map(|&(name, rect)| {
                let id = ui.registry.create_frame(name, None);
                let frame = ui.registry.get_mut(id).unwrap();
                frame.widget_type = WidgetType::Texture;
                frame.width = Dimension::Fixed(rect[2] as f32);
                frame.height = Dimension::Fixed(rect[3] as f32);
                frame.widget_data = Some(WidgetData::Texture(TextureData {
                    source: TextureSource::Atlas(name.into()),
                    ..default()
                }));
                (id, name, rect)
            })
            .collect()
    };
    for _ in 0..3 {
        app.update();
    }
    let entities: std::collections::HashMap<_, _> = app
        .world_mut()
        .query::<(Entity, &RegistryNode)>()
        .iter(app.world())
        .map(|(entity, node)| (node.0, entity))
        .collect();
    let projected: std::collections::HashMap<_, _> = app
        .world_mut()
        .query::<(&ChildOf, &ImageNode)>()
        .iter(app.world())
        .map(|(parent, image)| (parent.parent(), image.clone()))
        .collect();
    let images = app.world().resource::<Assets<Image>>();
    let mut missing = Vec::new();
    for (id, name, rect) in frames {
        let expected = expected_crop(&atlas, rect);
        let Some(node) = projected.get(&entities[&id]) else {
            missing.push(name);
            continue;
        };
        let actual = images.get(&node.image).expect("native image asset");
        assert_eq!(
            actual.size(),
            UVec2::new(rect[2] as u32, rect[3] as u32),
            "{name}"
        );
        assert_eq!(
            actual.texture_descriptor.format, atlas.texture_descriptor.format,
            "{name}"
        );
        assert_eq!(actual.data.as_deref(), Some(expected.as_slice()), "{name}");
        assert!(
            node.rect.is_none(),
            "{name}: materialized crop must not be cropped twice"
        );
        println!(
            "{name}: {}x{}, {} source-matched RGBA bytes",
            rect[2],
            rect[3],
            expected.len()
        );
    }
    assert!(
        missing.is_empty(),
        "customization regions have no native image output: {missing:?}"
    );
}
