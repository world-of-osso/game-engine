//! Sample the same decoded fill/overlay pixels and tints used by the native projection.
use super::*;
use game_engine_ui_model::damage_meter_component::damage_meter_screen;
use game_engine_ui_model::damage_meter_data::{DamageMeterRow, DamageMeterView, class_color};
use ui_toolkit::atlas::ActiveSkin;
use ui_toolkit::frame::WidgetData;
use ui_toolkit::screen::{Screen, SharedContext};

fn sample_texture(texture: &ui_toolkit::widgets::texture::TextureData, x: f32, y: f32) -> [f32; 4] {
    let pixel = match &texture.source {
        TextureSource::SolidColor(color) => *color,
        TextureSource::File(path) => sample_file(path, texture.tex_coords, x, y),
        TextureSource::FileDataId(fdid) => sample_file(
            &format!("data/textures/{fdid}.blp"),
            texture.tex_coords,
            x,
            y,
        ),
        other => panic!("unsupported fill source {other:?}"),
    };
    std::array::from_fn(|channel| pixel[channel] * texture.vertex_color[channel])
}

fn sample_file(path: &str, [left, right, top, bottom]: [f32; 4], x: f32, y: f32) -> [f32; 4] {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..");
    let image = decode_file(path, &fs::read(root.join(path)).unwrap()).unwrap();
    let u = left + (right - left) * x;
    let v = top + (bottom - top) * y;
    let px = ((u * image.width as f32) as usize).min(image.width as usize - 1);
    let py = ((v * image.height as f32) as usize).min(image.height as usize - 1);
    let offset = (py * image.width as usize + px) * 4;
    std::array::from_fn(|channel| image.rgba[offset + channel] as f32 / 255.0)
}

fn fill_pixel(registry: &FrameRegistry, x: f32, y: f32) -> [f32; 3] {
    let sample = |name| {
        let frame = registry.get(registry.get_by_name(name).unwrap()).unwrap();
        let Some(WidgetData::Texture(texture)) = &frame.widget_data else {
            panic!("{name} is not a texture")
        };
        sample_texture(texture, x, y)
    };
    let fill_frame = registry
        .get(registry.get_by_name("DamageMeterEntry1StatusBar").unwrap())
        .unwrap();
    let fill = match fill_frame.background_color {
        Some(color) => color,
        None => sample("DamageMeterEntry1StatusBar"),
    };
    let overlay = sample("DamageMeterEntry1Gradient");
    std::array::from_fn(|channel| {
        fill[channel] * (1.0 - overlay[3]) + overlay[channel] * overlay[3]
    })
}

#[test]
fn metergaps_forever_fill_stops_brighten_each_class_without_vertical_highlight() {
    game_engine_ui_model::paths::set_data_root(
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../data"),
    )
    .unwrap();
    for class_id in [1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13] {
        let color = class_color(class_id);
        let mut shared = SharedContext::new();
        shared.insert(ActiveSkin::Forever);
        shared.insert(DamageMeterView {
            rows: vec![DamageMeterRow {
                name_text: "1. Player".into(),
                value_text: "100".into(),
                fraction: 1.0,
                color,
                class_id,
                is_local_player: true,
            }],
            ..Default::default()
        });
        let mut registry = FrameRegistry::new(1920.0, 1080.0);
        Screen::new(damage_meter_screen).sync(&shared, &mut registry);
        for (x, brightness) in [(0.0, 0.4855), (1.0, 1.0)] {
            let pixel = fill_pixel(&registry, x, 0.5);
            for channel in 0..3 {
                assert!(
                    (pixel[channel] - color[channel] * brightness).abs() < 0.005,
                    "class {class_id} stop {x}: {pixel:?}, expected {brightness} * {color:?}"
                );
            }
            assert_eq!(
                fill_pixel(&registry, x, 0.25),
                fill_pixel(&registry, x, 0.75),
                "vertical highlight"
            );
        }
    }
}
