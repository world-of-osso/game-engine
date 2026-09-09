use super::*;
use bevy::asset::{AssetApp, AssetEvent, AssetPlugin};
use bevy::ecs::message::Messages;
use bevy::render::render_resource::encase::UniformBuffer;
use std::time::Duration;

fn settings_fixture() -> WaterSettings {
    WaterSettings {
        base_color: Vec4::new(0.1, 0.2, 0.3, 0.4),
        scroll_speed_1: Vec2::new(0.125, -0.25),
        scroll_speed_2: Vec2::new(-0.5, 0.75),
        normal_scale: 0.625,
        fresnel_power: 2.5,
        specular_strength: 1.25,
        time: 0.0,
        sky_color: Vec4::new(0.9, 0.8, 0.7, 0.6),
        wave_amplitude: 0.2,
        wave_frequency: 1.75,
        wave_speed: 0.875,
        foam_intensity: 0.45,
    }
}

fn encode_settings(settings: &WaterSettings) -> Vec<u8> {
    let mut buffer = UniformBuffer::new(Vec::new());
    buffer
        .write(settings)
        .expect("water settings encode as uniform");
    buffer.into_inner()
}

fn take_modified_water(app: &mut App) -> Vec<bevy::asset::AssetId<WaterMaterial>> {
    app.world_mut()
        .resource_mut::<Messages<AssetEvent<WaterMaterial>>>()
        .drain()
        .filter_map(|event| match event {
            AssetEvent::Modified { id } => Some(id),
            _ => None,
        })
        .collect()
}

#[test]
fn water_shared_clock_does_not_modify_material_assets() {
    let mut app = App::new();
    app.add_plugins((bevy::app::TaskPoolPlugin::default(), AssetPlugin::default()));
    app.init_asset::<Mesh>();
    app.insert_resource(Time::<()>::default());
    app.add_plugins(WaterMaterialPlugin);
    let settings = settings_fixture();
    let expected_bytes = encode_settings(&settings);
    let handle = app
        .world_mut()
        .resource_mut::<Assets<WaterMaterial>>()
        .add(WaterMaterial {
            settings,
            normal_map: Handle::default(),
        });
    app.update();
    take_modified_water(&mut app);

    for delta in [
        Duration::from_millis(250),
        Duration::from_secs(1),
        Duration::from_secs(3600),
    ] {
        app.world_mut().resource_mut::<Time>().advance_by(delta);
        app.update();
        assert!(
            take_modified_water(&mut app).is_empty(),
            "clock-only updates must not invalidate water materials"
        );
        let material = app
            .world()
            .resource::<Assets<WaterMaterial>>()
            .get(&handle)
            .unwrap();
        assert_eq!(
            encode_settings(&material.settings),
            expected_bytes,
            "individual water parameters must remain unchanged"
        );
    }

    // Actual appearance changes must still notify consumers normally.
    app.world_mut()
        .resource_mut::<Assets<WaterMaterial>>()
        .get_mut(&handle)
        .unwrap()
        .settings
        .normal_scale = 0.5;
    app.update();
    assert_eq!(take_modified_water(&mut app), vec![handle.id()]);
    assert_eq!(
        app.world()
            .resource::<Assets<WaterMaterial>>()
            .get(&handle)
            .unwrap()
            .settings
            .normal_scale,
        0.5
    );
}

#[test]
fn water_shared_clock_uniform_keeps_vector_alignment() {
    let settings = settings_fixture();
    let bytes = encode_settings(&settings);
    let float_at =
        |offset: usize| f32::from_le_bytes(bytes[offset..offset + 4].try_into().unwrap());
    assert_eq!(bytes.len(), 80);
    for (offset, values) in [
        (0, settings.base_color.to_array().to_vec()),
        (16, settings.scroll_speed_1.to_array().to_vec()),
        (24, settings.scroll_speed_2.to_array().to_vec()),
        (
            32,
            vec![
                settings.normal_scale,
                settings.fresnel_power,
                settings.specular_strength,
            ],
        ),
        (48, settings.sky_color.to_array().to_vec()),
        (
            64,
            vec![
                settings.wave_amplitude,
                settings.wave_frequency,
                settings.wave_speed,
                settings.foam_intensity,
            ],
        ),
    ] {
        for (index, expected) in values.into_iter().enumerate() {
            assert_eq!(
                float_at(offset + index * 4),
                expected,
                "uniform field offset {}",
                offset + index * 4
            );
        }
    }
}
