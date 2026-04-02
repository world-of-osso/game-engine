use std::path::Path;
use std::sync::OnceLock;

#[derive(Debug, Clone, PartialEq)]
pub struct LightEntry {
    pub id: u32,
    pub map_id: u32,
    pub position: [f32; 3],
    pub falloff_end: f32,
    pub light_params_ids: [u32; 8],
}

static LIGHTS: OnceLock<Vec<LightEntry>> = OnceLock::new();

pub fn resolve_light_params_id(map_id: u32, wow_position: [f32; 3]) -> Option<u32> {
    cached_lights()
        .iter()
        .filter(|row| row.map_id == map_id)
        .filter_map(|row| score_light_row(row, wow_position).map(|score| (score, row)))
        .min_by(|(a, _), (b, _)| a.total_cmp(b))
        .and_then(|(_, row)| row.light_params_ids.first().copied())
        .filter(|id| *id != 0)
}

fn cached_lights() -> &'static [LightEntry] {
    LIGHTS.get_or_init(|| load_lights(Path::new("data/Light.csv")))
}

fn score_light_row(row: &LightEntry, wow_position: [f32; 3]) -> Option<f32> {
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

fn load_lights(path: &Path) -> Vec<LightEntry> {
    let Ok(data) = std::fs::read_to_string(path) else {
        eprintln!("Light.csv not found at {}", path.display());
        return Vec::new();
    };
    let mut rows = Vec::new();
    for line in data.lines().skip(1) {
        if let Some(row) = parse_light_line(line) {
            rows.push(row);
        }
    }
    rows
}

fn parse_light_line(line: &str) -> Option<LightEntry> {
    let fields: Vec<&str> = line.split(',').collect();
    if fields.len() < 15 {
        return None;
    }
    Some(LightEntry {
        id: fields[0].parse().ok()?,
        position: [
            fields[1].parse().ok()?,
            fields[2].parse().ok()?,
            fields[3].parse().ok()?,
        ],
        falloff_end: fields[5].parse().ok()?,
        map_id: fields[6].parse().ok()?,
        light_params_ids: [
            fields[7].parse().ok()?,
            fields[8].parse().ok()?,
            fields[9].parse().ok()?,
            fields[10].parse().ok()?,
            fields[11].parse().ok()?,
            fields[12].parse().ok()?,
            fields[13].parse().ok()?,
            fields[14].parse().ok()?,
        ],
    })
}

#[cfg(test)]
mod tests {
    use super::resolve_light_params_id;

    #[test]
    fn authored_light_lookup_matches_ohnahran_scene() {
        let scene = crate::warband_scene::WarbandScenes::load()
            .scenes
            .into_iter()
            .find(|scene| scene.id == 4)
            .expect("known scene");

        let params = resolve_light_params_id(scene.map_id, scene.position);

        assert_eq!(params, Some(6577));
    }

    #[test]
    fn authored_light_lookup_matches_freywold_scene() {
        let scene = crate::warband_scene::WarbandScenes::load()
            .scenes
            .into_iter()
            .find(|scene| scene.id == 7)
            .expect("known scene");

        let params = resolve_light_params_id(scene.map_id, scene.position);

        assert_eq!(params, Some(5615));
    }
}
