use bevy::color::LinearRgba;
use bevy::prelude::*;
use bevy_hanabi::prelude::{
    AccelModifier, AlphaMode, Attribute, ColorOverLifetimeModifier, EffectAsset, ExprWriter,
    Gradient, OrientMode, OrientModifier, ParticleEffect, SetAttributeModifier,
    SetPositionSphereModifier, ShapeDimension, SimulationSpace, SizeOverLifetimeModifier,
    SpawnerSettings,
};

use crate::camera::WowCamera;
use crate::game_state::GameState;
use crate::networking::CurrentZone;
use crate::sky_lightdata::SkyColorSet;
use crate::terrain::AdtManager;

const WEATHER_MATCH_RAIN: &[&str] = &[
    "elwynn forest",
    "duskwood",
    "stranglethorn",
    "swamp of sorrows",
    "dustwallow marsh",
    "feralas",
    "ashenvale",
    "zangarmarsh",
    "howling fjord",
    "grizzly hills",
    "tirisfal glades",
];
const WEATHER_MATCH_SNOW: &[&str] = &[
    "dun morogh",
    "winterspring",
    "icecrown",
    "storm peaks",
    "dragonblight",
    "coldarra",
    "azure span",
    "alterac mountains",
];
const WEATHER_MATCH_SANDSTORM: &[&str] = &[
    "tanaris", "silithus", "uldum", "vol'dun", "voldun", "desolace", "badlands",
];
const WEATHER_FOG_TINT_BLEND: f32 = 0.55;
const WEATHER_DIRECTIONAL_TINT_BLEND: f32 = 0.35;
const WEATHER_EFFECT_CAPACITY_HEADROOM: f32 = 1.15;

pub struct WeatherPlugin;

impl Plugin for WeatherPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<ActiveWeather>()
            .add_systems(
                Update,
                (sync_active_weather, sync_weather_effect)
                    .chain()
                    .run_if(in_state(GameState::InWorld)),
            )
            .add_systems(OnExit(GameState::InWorld), teardown_weather_effects);
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WeatherKind {
    Clear,
    Rain,
    Snow,
    Sandstorm,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum WeatherOrientation {
    FaceCamera,
    AlongVelocity,
}

#[derive(Resource, Debug, Clone, Copy)]
pub struct ActiveWeather {
    pub kind: WeatherKind,
    intensity: f32,
    fog_distance_scale: f32,
    fog_tint: [f32; 3],
    directional_tint: [f32; 3],
    spawn_rate: f32,
    lifetime: f32,
    spawn_radius: f32,
    spawn_height: f32,
    velocity: [f32; 3],
    acceleration: [f32; 3],
    size: [f32; 3],
    color: [f32; 4],
    orientation: WeatherOrientation,
}

impl Default for ActiveWeather {
    fn default() -> Self {
        Self::clear()
    }
}

impl ActiveWeather {
    pub const fn clear() -> Self {
        Self {
            kind: WeatherKind::Clear,
            intensity: 0.0,
            fog_distance_scale: 1.0,
            fog_tint: [0.0, 0.0, 0.0],
            directional_tint: [0.0, 0.0, 0.0],
            spawn_rate: 0.0,
            lifetime: 0.0,
            spawn_radius: 0.0,
            spawn_height: 0.0,
            velocity: [0.0, 0.0, 0.0],
            acceleration: [0.0, 0.0, 0.0],
            size: [0.0, 0.0, 0.0],
            color: [0.0, 0.0, 0.0, 0.0],
            orientation: WeatherOrientation::FaceCamera,
        }
    }

    fn matches(&self, other: &Self) -> bool {
        self.kind == other.kind
            && self.intensity == other.intensity
            && self.fog_distance_scale == other.fog_distance_scale
            && self.fog_tint == other.fog_tint
            && self.directional_tint == other.directional_tint
            && self.spawn_rate == other.spawn_rate
            && self.lifetime == other.lifetime
            && self.spawn_radius == other.spawn_radius
            && self.spawn_height == other.spawn_height
            && self.velocity == other.velocity
            && self.acceleration == other.acceleration
            && self.size == other.size
            && self.color == other.color
            && self.orientation == other.orientation
    }

    pub fn is_clear(&self) -> bool {
        self.kind == WeatherKind::Clear
    }

    pub(crate) fn preset(kind: WeatherKind) -> Self {
        match kind {
            WeatherKind::Clear => Self::clear(),
            WeatherKind::Rain => rain_weather(0.8),
            WeatherKind::Snow => snow_weather(0.75),
            WeatherKind::Sandstorm => sandstorm_weather(0.85),
        }
    }

    fn capacity(&self) -> u32 {
        ((self.spawn_rate * self.intensity * self.lifetime * WEATHER_EFFECT_CAPACITY_HEADROOM)
            .ceil()
            .max(32.0)) as u32
    }

    fn tint_color(&self) -> Color {
        Color::linear_rgb(self.fog_tint[0], self.fog_tint[1], self.fog_tint[2])
    }

    fn directional_tint_color(&self) -> Color {
        Color::linear_rgb(
            self.directional_tint[0],
            self.directional_tint[1],
            self.directional_tint[2],
        )
    }
}

#[derive(Component)]
struct WeatherEffectRoot;

#[derive(Default, Clone, PartialEq, Eq)]
struct WeatherSourceKey {
    zone_id: u32,
    map_name: String,
}

fn sync_active_weather(
    current_zone: Res<CurrentZone>,
    adt_manager: Res<AdtManager>,
    mut active_weather: ResMut<ActiveWeather>,
    mut last_source: Local<WeatherSourceKey>,
) {
    let next_source = WeatherSourceKey {
        zone_id: current_zone.zone_id,
        map_name: adt_manager.map_name.clone(),
    };
    if *last_source == next_source {
        return;
    }
    *last_source = next_source.clone();

    let next_weather = weather_for_zone(next_source.zone_id, &next_source.map_name);
    if !active_weather.matches(&next_weather) {
        *active_weather = next_weather;
    }
}

fn sync_weather_effect(
    mut commands: Commands,
    active_weather: Res<ActiveWeather>,
    camera_q: Query<Entity, With<WowCamera>>,
    effect_q: Query<Entity, With<WeatherEffectRoot>>,
    mut effects: ResMut<Assets<EffectAsset>>,
) {
    let existing: Vec<Entity> = effect_q.iter().collect();
    let desired_active = !active_weather.is_clear();
    let should_rebuild = active_weather.is_changed()
        || (desired_active && existing.is_empty())
        || (!desired_active && !existing.is_empty());
    if !should_rebuild {
        return;
    }
    for entity in existing {
        commands.entity(entity).despawn();
    }
    if !desired_active {
        return;
    }
    let Ok(camera_entity) = camera_q.single() else {
        return;
    };

    let effect = effects.add(build_weather_effect(&active_weather));
    let weather_entity = commands
        .spawn((
            Name::new(format!("weather::{:?}", active_weather.kind)),
            WeatherEffectRoot,
            ParticleEffect::new(effect),
            Transform::IDENTITY,
            GlobalTransform::IDENTITY,
            Visibility::Inherited,
        ))
        .id();
    commands
        .entity(weather_entity)
        .set_parent_in_place(camera_entity);
}

fn teardown_weather_effects(
    mut commands: Commands,
    effect_q: Query<Entity, With<WeatherEffectRoot>>,
    mut active_weather: ResMut<ActiveWeather>,
) {
    for entity in effect_q.iter() {
        commands.entity(entity).despawn();
    }
    *active_weather = ActiveWeather::clear();
}

fn build_weather_effect(active_weather: &ActiveWeather) -> EffectAsset {
    let (module, init_age, init_lifetime, init_position, init_velocity, acceleration) =
        weather_effect_module(active_weather);
    let effect = weather_effect_asset(
        active_weather,
        module,
        init_age,
        init_lifetime,
        init_position,
        init_velocity,
        acceleration,
    );
    effect.render(OrientModifier::new(weather_orient_mode(active_weather)))
}

fn weather_effect_module(
    active_weather: &ActiveWeather,
) -> (
    bevy_hanabi::prelude::Module,
    SetAttributeModifier,
    SetAttributeModifier,
    SetPositionSphereModifier,
    SetAttributeModifier,
    AccelModifier,
) {
    let writer = ExprWriter::new();
    let init_age = SetAttributeModifier::new(Attribute::AGE, writer.lit(0.0).expr());
    let init_lifetime = SetAttributeModifier::new(
        Attribute::LIFETIME,
        writer.lit(active_weather.lifetime).expr(),
    );
    let init_position = SetPositionSphereModifier {
        center: writer
            .lit(Vec3::new(0.0, active_weather.spawn_height, 0.0))
            .expr(),
        radius: writer.lit(active_weather.spawn_radius).expr(),
        dimension: ShapeDimension::Volume,
    };
    let init_velocity = SetAttributeModifier::new(
        Attribute::VELOCITY,
        writer.lit(Vec3::from_array(active_weather.velocity)).expr(),
    );
    let acceleration = AccelModifier::new(
        writer
            .lit(Vec3::from_array(active_weather.acceleration))
            .expr(),
    );
    let module = writer.finish();
    (
        module,
        init_age,
        init_lifetime,
        init_position,
        init_velocity,
        acceleration,
    )
}

fn weather_effect_asset(
    active_weather: &ActiveWeather,
    module: bevy_hanabi::prelude::Module,
    init_age: SetAttributeModifier,
    init_lifetime: SetAttributeModifier,
    init_position: SetPositionSphereModifier,
    init_velocity: SetAttributeModifier,
    acceleration: AccelModifier,
) -> EffectAsset {
    EffectAsset::new(
        active_weather.capacity(),
        SpawnerSettings::rate((active_weather.spawn_rate * active_weather.intensity).into()),
        module,
    )
    .with_name(format!("weather::{:?}", active_weather.kind))
    .with_alpha_mode(AlphaMode::Blend)
    .with_simulation_space(SimulationSpace::Local)
    .init(init_age)
    .init(init_lifetime)
    .init(init_position)
    .init(init_velocity)
    .update(acceleration)
    .render(SizeOverLifetimeModifier {
        gradient: Gradient::constant(Vec3::from_array(active_weather.size)),
        screen_space_size: false,
    })
    .render(ColorOverLifetimeModifier::new(weather_color_gradient(
        active_weather.color,
    )))
}

fn weather_orient_mode(active_weather: &ActiveWeather) -> OrientMode {
    match active_weather.orientation {
        WeatherOrientation::FaceCamera => OrientMode::FaceCameraPosition,
        WeatherOrientation::AlongVelocity => OrientMode::AlongVelocity,
    }
}

fn weather_color_gradient(color: [f32; 4]) -> Gradient<Vec4> {
    let mut gradient = Gradient::new();
    gradient.add_key(0.0, Vec4::new(color[0], color[1], color[2], color[3]));
    gradient.add_key(0.7, Vec4::new(color[0], color[1], color[2], color[3] * 0.9));
    gradient.add_key(1.0, Vec4::new(color[0], color[1], color[2], 0.0));
    gradient
}

pub(crate) fn weather_adjusted_fog(
    colors: &SkyColorSet,
    weather: Option<&ActiveWeather>,
) -> (Color, Color, FogFalloff) {
    let Some(weather) = weather.filter(|weather| !weather.is_clear()) else {
        return (
            colors.sky_smog,
            colors.sky_band2,
            fog_falloff(colors.fog_start, colors.fog_end),
        );
    };

    let tint_factor = (weather.intensity * WEATHER_FOG_TINT_BLEND).clamp(0.0, 1.0);
    let directional_tint_factor =
        (weather.intensity * WEATHER_DIRECTIONAL_TINT_BLEND).clamp(0.0, 1.0);
    let fog_start = colors.fog_start * weather.fog_distance_scale;
    let fog_end = colors.fog_end * weather.fog_distance_scale;

    (
        lerp_color(colors.sky_smog, weather.tint_color(), tint_factor),
        lerp_color(
            colors.sky_band2,
            weather.directional_tint_color(),
            directional_tint_factor,
        ),
        fog_falloff(fog_start, fog_end),
    )
}

fn fog_falloff(start: f32, end: f32) -> FogFalloff {
    let end = end.max(1.0);
    let start = start.clamp(0.0, end - 0.001);
    FogFalloff::Linear { start, end }
}

fn lerp_color(a: Color, b: Color, t: f32) -> Color {
    let a = a.to_linear();
    let b = b.to_linear();
    Color::linear_rgba(
        a.red + (b.red - a.red) * t,
        a.green + (b.green - a.green) * t,
        a.blue + (b.blue - a.blue) * t,
        a.alpha + (b.alpha - a.alpha) * t,
    )
}

fn weather_for_zone(zone_id: u32, map_name: &str) -> ActiveWeather {
    if zone_id == 0 {
        return map_default_weather(map_name);
    }
    let zone_name = game_engine::world_db::load_zone_name(zone_id)
        .ok()
        .flatten()
        .unwrap_or_default();
    weather_for_zone_name(&zone_name, map_name)
}

fn map_default_weather(map_name: &str) -> ActiveWeather {
    let normalized = map_name.trim().to_ascii_lowercase();
    if normalized.contains("northrend") {
        snow_weather(0.55)
    } else {
        ActiveWeather::clear()
    }
}

fn weather_for_zone_name(zone_name: &str, map_name: &str) -> ActiveWeather {
    ActiveWeather::preset(classify_weather_kind(zone_name, map_name))
}

fn classify_weather_kind(zone_name: &str, map_name: &str) -> WeatherKind {
    let normalized_zone = zone_name.trim().to_ascii_lowercase();
    if contains_any(&normalized_zone, WEATHER_MATCH_SNOW) {
        return WeatherKind::Snow;
    }
    if contains_any(&normalized_zone, WEATHER_MATCH_SANDSTORM) {
        return WeatherKind::Sandstorm;
    }
    if contains_any(&normalized_zone, WEATHER_MATCH_RAIN) {
        return WeatherKind::Rain;
    }

    let normalized_map = map_name.trim().to_ascii_lowercase();
    if normalized_map.contains("northrend") {
        WeatherKind::Snow
    } else {
        WeatherKind::Clear
    }
}

fn contains_any(zone_name: &str, matches: &[&str]) -> bool {
    matches.iter().any(|needle| zone_name.contains(needle))
}

fn rain_weather(intensity: f32) -> ActiveWeather {
    ActiveWeather {
        kind: WeatherKind::Rain,
        intensity,
        fog_distance_scale: 0.72,
        fog_tint: [0.42, 0.46, 0.54],
        directional_tint: [0.58, 0.63, 0.72],
        spawn_rate: 540.0,
        lifetime: 1.35,
        spawn_radius: 18.0,
        spawn_height: 15.0,
        velocity: [-1.2, -24.0, 0.35],
        acceleration: [-0.8, -5.0, 0.1],
        size: [0.018, 0.32, 1.0],
        color: [0.73, 0.8, 0.94, 0.5],
        orientation: WeatherOrientation::AlongVelocity,
    }
}

fn snow_weather(intensity: f32) -> ActiveWeather {
    ActiveWeather {
        kind: WeatherKind::Snow,
        intensity,
        fog_distance_scale: 0.63,
        fog_tint: [0.83, 0.87, 0.93],
        directional_tint: [0.92, 0.95, 1.0],
        spawn_rate: 150.0,
        lifetime: 6.0,
        spawn_radius: 20.0,
        spawn_height: 14.0,
        velocity: [0.0, -2.8, 0.0],
        acceleration: [0.35, 0.0, 0.18],
        size: [0.09, 0.09, 1.0],
        color: [0.97, 0.98, 1.0, 0.9],
        orientation: WeatherOrientation::FaceCamera,
    }
}

fn sandstorm_weather(intensity: f32) -> ActiveWeather {
    ActiveWeather {
        kind: WeatherKind::Sandstorm,
        intensity,
        fog_distance_scale: 0.42,
        fog_tint: [0.74, 0.62, 0.42],
        directional_tint: [0.82, 0.7, 0.52],
        spawn_rate: 380.0,
        lifetime: 3.2,
        spawn_radius: 16.0,
        spawn_height: 2.5,
        velocity: [-13.0, 1.0, 4.0],
        acceleration: [-2.5, 0.0, 0.8],
        size: [0.13, 0.08, 1.0],
        color: [0.86, 0.75, 0.54, 0.45],
        orientation: WeatherOrientation::AlongVelocity,
    }
}

fn linear_rgb_array(color: Color) -> [f32; 3] {
    let linear: LinearRgba = color.to_linear();
    [linear.red, linear.green, linear.blue]
}

#[cfg(test)]
#[path = "../../tests/unit/weather_tests.rs"]
mod tests;
