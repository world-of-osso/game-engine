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

/// Original debug lookup priority: the closest applicable local sphere, then same-map global.
pub fn score_light_row(row: &LightEntry, wow_position: [f32; 3]) -> Option<f32> {
    if row.position == [0.0, 0.0, 0.0] {
        return Some(f32::MAX / 4.0);
    }
    let dx = row.position[0] - wow_position[0];
    let dy = row.position[1] - wow_position[1];
    let dz = row.position[2] - wow_position[2];
    let distance = (dx * dx + dy * dy + dz * dz).sqrt();
    if row.falloff_end > 0.0 && distance > row.falloff_end {
        return None;
    }
    Some(distance)
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

/// A ZoneLight.db2 polygon on `map_id` that applies Light `light_id` between `z_min` and
/// `z_max`. `points` are the ZoneLightPoint vertices by descending PointOrder
/// (WebWowViewerCpp `CSqliteDB.cpp:170-172`); `priority` is TransitionType (`:679`).
#[derive(Debug, Clone, PartialEq)]
pub struct ZoneLight {
    pub id: u32,
    pub map_id: u32,
    pub light_id: u32,
    pub priority: i32,
    pub z_min: f32,
    pub z_max: f32,
    pub points: Vec<[f32; 2]>,
}

/// ZoneLight.csv joined with ZoneLightPoint.csv, as exported from local CASC by
/// `scripts/export_db2_csv.py`. Every row is numeric except ZoneLight's Name.
pub fn parse_zone_lights(
    zone_lights: &str,
    zone_light_points: &str,
) -> Result<Vec<ZoneLight>, String> {
    let point_table = CsvTable::read(zone_light_points, "ZoneLightPoint.csv")?;
    let mut points: std::collections::HashMap<u32, Vec<(u32, [f32; 2])>> = Default::default();
    for row in &point_table.rows {
        let value = |column| point_table.value::<f32>(row, column);
        points
            .entry(point_table.value(row, "ZoneLightID")?)
            .or_default()
            .push((
                point_table.value(row, "PointOrder")?,
                [value("Pos_0")?, value("Pos_1")?],
            ));
    }
    let zone_table = CsvTable::read(zone_lights, "ZoneLight.csv")?;
    let mut zones = Vec::with_capacity(zone_table.rows.len());
    for row in &zone_table.rows {
        let id = zone_table.value(row, "ID")?;
        let mut vertices = points.remove(&id).unwrap_or_default();
        vertices.sort_by(|a, b| b.0.cmp(&a.0));
        zones.push(ZoneLight {
            id,
            map_id: zone_table.value(row, "MapID")?,
            light_id: zone_table.value(row, "LightID")?,
            priority: zone_table.value(row, "TransitionType")?,
            z_min: zone_table.value(row, "Zmin")?,
            z_max: zone_table.value(row, "Zmax")?,
            points: vertices.into_iter().map(|(_, point)| point).collect(),
        });
    }
    if zones.is_empty() {
        return Err("ZoneLight.csv contains no rows".into());
    }
    Ok(zones)
}

type CsvLine<'a> = (usize, Vec<&'a str>);

struct CsvTable<'a> {
    file: &'static str,
    header: Vec<&'a str>,
    rows: Vec<CsvLine<'a>>,
}

impl<'a> CsvTable<'a> {
    fn read(source: &'a str, file: &'static str) -> Result<Self, String> {
        let mut lines = source.lines().enumerate();
        let header: Vec<_> = match lines.next() {
            Some((_, header)) if !header.is_empty() => header.split(',').collect(),
            _ => return Err(format!("{file} has no header")),
        };
        let rows = lines
            .filter(|(_, line)| !line.is_empty())
            .map(|(index, line)| {
                let values: Vec<_> = line.split(',').collect();
                if values.len() != header.len() {
                    return Err(format!(
                        "{file} line {} has {} fields",
                        index + 1,
                        values.len()
                    ));
                }
                Ok((index + 1, values))
            })
            .collect::<Result<_, String>>()?;
        Ok(Self { file, header, rows })
    }

    fn value<T: std::str::FromStr>(
        &self,
        (line, values): &CsvLine<'_>,
        column: &str,
    ) -> Result<T, String>
    where
        T::Err: std::fmt::Display,
    {
        let index = self
            .header
            .iter()
            .position(|name| *name == column)
            .ok_or_else(|| format!("{} is missing column {column}", self.file))?;
        values[index]
            .parse()
            .map_err(|error| format!("{} line {line} {column}: {error}", self.file))
    }
}

/// Distance within which a zone light border fades in (`LightParamCalculate.h:104`).
const ZONE_BLEND_DISTANCE: f32 = 50.0;

/// WebWowViewerCpp `calculateLightParamBlends` (`LightParamCalculate.h:42-194`, commit
/// 1a8cccb) in overlay order: the map default at weight 1, then zone lights by priority and
/// descending Light ID, then local lights from the strongest to the weakest.
pub fn light_params_blend(
    lights: &[LightEntry],
    zone_lights: &[ZoneLight],
    map_id: u32,
    wow_position: [f32; 3],
    slot: LightParamsSlot,
) -> Vec<WeightedLightParams> {
    let Some(default) = default_light(lights, map_id) else {
        return Vec::new();
    };
    let params = |light: &LightEntry| light.light_params_ids[slot.index()];
    let mut blend = vec![WeightedLightParams {
        light_params_id: params(default),
        weight: 1.0,
    }];
    blend.extend(
        zone_light_weights(zone_lights, map_id, wow_position)
            .into_iter()
            .filter_map(|(light_id, weight)| {
                let light = lights.iter().find(|light| light.id == light_id)?;
                Some(WeightedLightParams {
                    light_params_id: params(light),
                    weight,
                })
            }),
    );
    blend.extend(
        local_light_weights(lights, map_id, wow_position)
            .into_iter()
            .map(|(light, weight)| WeightedLightParams {
                light_params_id: params(light),
                weight,
            }),
    );
    blend.retain(|light| light.light_params_id != 0);
    blend
}

/// The map's zero-position Light with the highest ID, else continent 0's
/// (`LightParamCalculate.h:72-93` over `CSqliteDB.cpp:77-96`, which orders by ID descending).
fn default_light(lights: &[LightEntry], map_id: u32) -> Option<&LightEntry> {
    let global_on = |map| {
        lights
            .iter()
            .filter(move |light| light.map_id == map && is_global_light(light))
            .max_by_key(|light| light.id)
    };
    global_on(map_id).or_else(|| global_on(0))
}

/// Zone lights within 50 yd of the position, with `clamp(-dist / 100)` weights, sorted by
/// priority then descending Light ID (`LightParamCalculate.h:106-136`, `:151-159`).
fn zone_light_weights(
    zone_lights: &[ZoneLight],
    map_id: u32,
    position: [f32; 3],
) -> Vec<(u32, f32)> {
    let mut found: Vec<(&ZoneLight, f32)> = zone_lights
        .iter()
        .filter(|zone| zone.map_id == map_id && zone.points.len() > 1)
        .filter_map(|zone| zone_blend_distance(zone, position).map(|dist| (zone, dist)))
        .collect();
    found.sort_by(|(a, _), (b, _)| {
        a.priority
            .cmp(&b.priority)
            .then(b.light_id.cmp(&a.light_id))
    });
    found
        .into_iter()
        .map(|(zone, dist)| {
            let weight = (-dist / (2.0 * ZONE_BLEND_DISTANCE)).clamp(0.0, 1.0);
            (zone.light_id, weight)
        })
        .collect()
}

/// The signed distance past the 50 yd fade band, negative when the zone applies.
fn zone_blend_distance(zone: &ZoneLight, position: [f32; 3]) -> Option<f32> {
    let mut border = border_distance(&zone.points, position);
    if inside_zone(zone, position) {
        border = -border;
    }
    let below_max = zone.z_max - position[2];
    let above_min = position[2] - zone.z_min;
    let mut height = above_min.abs().min(below_max.abs());
    if above_min > 0.0 && below_max > 0.0 {
        height = -height;
    }
    let (border, height) = (border - ZONE_BLEND_DISTANCE, height - ZONE_BLEND_DISTANCE);
    (border < 0.0 && height < 0.0).then_some(border.max(height))
}

/// `MathHelper::isPointInsideNonConvex` (`mathHelper.cpp:737-769`): strictly inside the
/// zone's box, then an odd number of polygon edges crossing the segment from the position to
/// a point beyond the box.
fn inside_zone(zone: &ZoneLight, position: [f32; 3]) -> bool {
    let (min, max) = zone.points.iter().fold(
        ([f32::INFINITY; 2], [f32::NEG_INFINITY; 2]),
        |(min, max), point| {
            (
                [min[0].min(point[0]), min[1].min(point[1])],
                [max[0].max(point[0]), max[1].max(point[1])],
            )
        },
    );
    let in_box = (0..2).all(|axis| position[axis] > min[axis] && position[axis] < max[axis])
        && position[2] > zone.z_min
        && position[2] < zone.z_max;
    if !in_box {
        return false;
    }
    let point = [position[0], position[1]];
    let outside = [max[0] + 1000.0, max[1] + 1000.0];
    let crossings = polygon_edges(&zone.points)
        .filter(|(a, b)| segments_intersect(outside, point, *a, *b))
        .count();
    crossings % 2 == 1
}

/// `MathHelper::findLeastDistanceToBorder` (`mathHelper.cpp:771-805`).
fn border_distance(points: &[[f32; 2]], position: [f32; 3]) -> f32 {
    let point = [position[0], position[1]];
    polygon_edges(points)
        .map(|(a, b)| segment_distance(a, b, point))
        .fold(999_999.0, f32::min)
}

fn polygon_edges(points: &[[f32; 2]]) -> impl Iterator<Item = ([f32; 2], [f32; 2])> + '_ {
    points
        .iter()
        .zip(points.iter().cycle().skip(1))
        .map(|(a, b)| (*a, *b))
}

fn segment_distance(v: [f32; 2], w: [f32; 2], p: [f32; 2]) -> f32 {
    let edge = [w[0] - v[0], w[1] - v[1]];
    let length_sq = edge[0] * edge[0] + edge[1] * edge[1];
    let t = if length_sq == 0.0 {
        0.0
    } else {
        (((p[0] - v[0]) * edge[0] + (p[1] - v[1]) * edge[1]) / length_sq).clamp(0.0, 1.0)
    };
    (p[0] - v[0] - t * edge[0]).hypot(p[1] - v[1] - t * edge[1])
}

/// `areIntersecting` (`mathHelper.cpp:680-735`): the endpoints of each segment lie strictly
/// on opposite sides of the other's line; collinear segments do not count.
fn segments_intersect(a1: [f32; 2], a2: [f32; 2], b1: [f32; 2], b2: [f32; 2]) -> bool {
    let side = |p1: [f32; 2], p2: [f32; 2], q: [f32; 2]| {
        (p2[1] - p1[1]) * q[0] + (p1[0] - p2[0]) * q[1] + (p2[0] * p1[1] - p1[0] * p2[1])
    };
    let straddles = |p1, p2, q1, q2| {
        let (d1, d2) = (side(p1, p2, q1), side(p1, p2, q2));
        !(d1 > 0.0 && d2 > 0.0 || d1 < 0.0 && d2 < 0.0)
    };
    let collinear = (a2[1] - a1[1]) * (b1[0] - b2[0]) - (b2[1] - b1[1]) * (a1[0] - a2[0]) == 0.0;
    straddles(a1, a2, b1, b2) && straddles(b1, b2, a1, a2) && !collinear
}

/// Local Lights on the map whose falloff sphere holds the position, weighted like
/// `CSqliteDB::getEnvInfo` (`CSqliteDB.cpp:397-431`) and ordered by descending weight
/// (`LightParamCalculate.h:176-191`); equal weights keep the query's descending ID order.
fn local_light_weights(
    lights: &[LightEntry],
    map_id: u32,
    position: [f32; 3],
) -> Vec<(&LightEntry, f32)> {
    let mut locals: Vec<(&LightEntry, f32)> = lights
        .iter()
        .filter(|light| light.map_id == map_id && !is_global_light(light))
        .map(|light| (light, distance(light.position, position)))
        .filter(|(light, distance)| *distance < light.falloff_end)
        .map(|(light, distance)| (light, local_light_weight(light, distance)))
        .filter(|(_, weight)| *weight > 0.0)
        .collect();
    locals
        .sort_by(|(a, a_weight), (b, b_weight)| b_weight.total_cmp(a_weight).then(b.id.cmp(&a.id)));
    locals
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
