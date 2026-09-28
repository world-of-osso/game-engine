//! Bevy-free Light.csv lookup types and authored blend calculations.

/// Light.csv stores multiple LightParams circumstances, not fallback candidates.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum LightParamsSlot {
    Clear,
    ClearUnderwater,
    Storm,
    StormUnderwater,
    Death,
}

impl LightParamsSlot {
    pub const fn index(self) -> usize {
        match self {
            Self::Clear => 0,
            Self::ClearUnderwater => 1,
            Self::Storm => 2,
            Self::StormUnderwater => 3,
            Self::Death => 4,
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct LightEntry {
    pub id: u32,
    pub map_id: u32,
    pub position: [f32; 3],
    pub falloff_start: f32,
    pub falloff_end: f32,
    pub light_params_ids: [u32; 8],
}

/// One LightParams contributing to a position, with its overlay weight.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct WeightedLightParams {
    pub light_params_id: u32,
    pub weight: f32,
}

pub fn map_name_to_id(map_name: &str) -> Option<u32> {
    let normalized = normalize_map_name(map_name);
    if let Ok(id) = normalized.parse() {
        return Some(id);
    }
    match normalized.as_str() {
        "azeroth" => Some(0),
        "kalimdor" => Some(1),
        "expansion01" | "outland" => Some(530),
        "northrend" => Some(571),
        "deepholm" => Some(646),
        "pandaria" => Some(870),
        "draenor" => Some(1116),
        "brokenisles" | "brokenshorecontinent" => Some(1220),
        "argus" => Some(1669),
        "kultiras" | "kultirascontinent" => Some(1643),
        "zandalar" => Some(1642),
        "zandalarcontinentfinale" => Some(1642),
        "nazjatar" => Some(1355),
        "shadowlands" => Some(2222),
        "dragonisles" => Some(2444),
        "khazalgar" => Some(2552),
        _ => None,
    }
}

fn normalize_map_name(map_name: &str) -> String {
    let normalized = map_name.trim().replace('\\', "/").to_ascii_lowercase();
    let map_segment = normalized
        .split("world/maps/")
        .nth(1)
        .and_then(|tail| tail.split('/').next())
        .filter(|segment| !segment.is_empty())
        .unwrap_or(normalized.as_str());

    map_segment
        .chars()
        .filter(|ch| ch.is_ascii_alphanumeric())
        .collect()
}

/// The map's last zero-position row, else authored Light ID 1, followed by
/// overlapping local lights from farthest to nearest in overlay order.
/// Source: wowdev DB/Light and solarityclient `sampling.rs`.
pub fn light_params_blend(
    lights: &[LightEntry],
    map_id: u32,
    wow_position: [f32; 3],
    slot: LightParamsSlot,
) -> Vec<WeightedLightParams> {
    let Some(global) = lights
        .iter()
        .rev()
        .find(|light| light.map_id == map_id && is_global_light(light))
        .or_else(|| lights.iter().find(|light| light.id == 1))
    else {
        return Vec::new();
    };
    let mut locals: Vec<(&LightEntry, f32)> = lights
        .iter()
        .filter(|light| {
            light.map_id == map_id
                && !is_global_light(light)
                && light.light_params_ids[slot.index()] != 0
        })
        .map(|light| (light, distance(light.position, wow_position)))
        .filter(|(light, distance)| *distance < light.falloff_end)
        .collect();
    locals.sort_by(|(a, a_distance), (b, b_distance)| {
        if distance(a.position, b.position) < 1.0 / 3.0 {
            b.falloff_start.total_cmp(&a.falloff_start)
        } else {
            b_distance.total_cmp(a_distance)
        }
    });
    let mut blend = vec![WeightedLightParams {
        light_params_id: global.light_params_ids[slot.index()],
        weight: 1.0,
    }];
    blend.extend(locals.into_iter().filter_map(|(light, distance)| {
        let weight = local_light_weight(light, distance);
        (weight > 0.0).then_some(WeightedLightParams {
            light_params_id: light.light_params_ids[slot.index()],
            weight,
        })
    }));
    blend.retain(|light| light.light_params_id != 0);
    blend
}

fn is_global_light(light: &LightEntry) -> bool {
    light.position == [0.0, 0.0, 0.0]
}

fn local_light_weight(light: &LightEntry, distance: f32) -> f32 {
    if distance <= light.falloff_start {
        return 1.0;
    }
    let width = light.falloff_end - light.falloff_start;
    (1.0 - (distance - light.falloff_start) / width).clamp(0.0, 1.0)
}

fn distance(a: [f32; 3], b: [f32; 3]) -> f32 {
    let dx = a[0] - b[0];
    let dy = a[1] - b[1];
    let dz = a[2] - b[2];
    (dx * dx + dy * dy + dz * dz).sqrt()
}
