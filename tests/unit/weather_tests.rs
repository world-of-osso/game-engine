use super::*;
use crate::client_options::GraphicsOptions;
use bevy::state::app::StatesPlugin;

#[test]
fn particles_disabled_preserves_weather_without_hanabi_assets() {
    let mut app = App::new();
    app.add_plugins((MinimalPlugins, StatesPlugin));
    app.init_state::<GameState>();
    app.insert_state(GameState::InWorld);
    app.insert_resource(GraphicsOptions {
        particle_effects_enabled: false,
        ..default()
    });
    app.init_resource::<CurrentZone>();
    let mut terrain = AdtManager::default();
    terrain.map_name = "northrend".into();
    app.insert_resource(terrain);
    app.add_plugins(WeatherPlugin);
    app.world_mut()
        .spawn((WowCamera::default(), Transform::default()));

    app.update();
    assert!(!app.world().contains_resource::<Assets<EffectAsset>>());
    assert_eq!(
        app.world().resource::<ActiveWeather>().kind,
        WeatherKind::Snow
    );
    let colors = crate::sky_lightdata::default_sky_colors();
    let (_, _, falloff) =
        weather_adjusted_fog(&colors, Some(app.world().resource::<ActiveWeather>()));
    assert!(matches!(falloff, FogFalloff::Linear { end, .. } if end < colors.fog_end));
    assert_eq!(
        app.world_mut()
            .query::<&ParticleEffect>()
            .iter(app.world())
            .count(),
        0
    );

    app.world_mut().resource_mut::<AdtManager>().map_name = "azeroth".into();
    app.update();
    assert!(app.world().resource::<ActiveWeather>().is_clear());
    assert!(!app.world().contains_resource::<Assets<EffectAsset>>());
    assert_eq!(
        app.world_mut()
            .query::<&ParticleEffect>()
            .iter(app.world())
            .count(),
        0
    );
}

#[test]
fn classify_weather_kind_matches_known_zone_names() {
    assert_eq!(
        classify_weather_kind("Elwynn Forest", "azeroth"),
        WeatherKind::Rain
    );
    assert_eq!(
        classify_weather_kind("Dun Morogh", "azeroth"),
        WeatherKind::Snow
    );
    assert_eq!(
        classify_weather_kind("Tanaris", "kalimdor"),
        WeatherKind::Sandstorm
    );
    assert_eq!(
        classify_weather_kind("Stormwind City", "azeroth"),
        WeatherKind::Clear
    );
}

#[test]
fn classify_weather_kind_uses_map_fallback_for_northrend() {
    assert_eq!(
        classify_weather_kind("Unknown", "northrend"),
        WeatherKind::Snow
    );
}

#[test]
fn weather_adjusted_fog_tints_and_shortens_base_fog() {
    let colors = crate::sky_lightdata::default_sky_colors();
    let weather = sandstorm_weather(0.85);

    let (fog_color, directional_color, falloff) = weather_adjusted_fog(&colors, Some(&weather));

    assert_ne!(fog_color.to_srgba(), colors.sky_smog.to_srgba());
    assert_ne!(directional_color.to_srgba(), colors.sky_band2.to_srgba());
    assert!(matches!(
        falloff,
        FogFalloff::Linear { start, end }
        if end < colors.fog_end && start < colors.fog_start
    ));
}

#[test]
fn weather_for_zone_name_returns_clear_for_unknown_zone() {
    let weather = weather_for_zone_name("Some Safe Interior", "azeroth");
    assert!(weather.is_clear());
}

#[test]
fn rain_weather_uses_velocity_oriented_streaks() {
    let weather = rain_weather(0.8);
    assert_eq!(weather.orientation, WeatherOrientation::AlongVelocity);
    assert!(weather.velocity[1] < -20.0);
    assert!(weather.size[1] > weather.size[0]);
}
