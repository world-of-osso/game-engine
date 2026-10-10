//! Client-local Retail QuestPOIBlob/QuestPOIPoint geometry. Custom server quests retain
//! their authored wire geometry; quests present in this catalog use only local rows.
use game_engine_core::csv_util::read_numeric_rows;
use shared::protocol::{QuestEntrySnapshot, QuestPoiPoint, QuestPoiSnapshot};
use std::collections::HashMap;
use std::path::Path;

#[derive(Default)]
pub struct QuestPoiCatalog {
    quests: HashMap<u32, Vec<QuestPoiSnapshot>>,
    pub unresolved: Vec<String>,
    pub conditional_blobs: usize,
}
impl QuestPoiCatalog {
    pub fn load(dir: &Path) -> Result<Self, String> {
        let mut points = load_points(dir)?;
        let rows = load_blob_rows(dir)?;
        let mut catalog = Self::default();
        for row in rows {
            catalog.insert_blob(row, &mut points);
        }
        Ok(catalog)
    }
    fn insert_blob(
        &mut self,
        [id, quest, map, ui_map, objective, flags, count, condition]: [i64; 8],
        points: &mut HashMap<u32, Vec<QuestPoiPoint>>,
    ) {
        let pois = self.quests.entry(quest as u32).or_default();
        // Condition evaluation is not implemented; never display a conditional shape.
        if condition != 0 {
            self.conditional_blobs += 1;
            return;
        }
        let polygon = points.remove(&(id as u32)).unwrap_or_default();
        if polygon.len() != count as usize {
            self.unresolved.push(format!(
                "QuestPOIBlob {id}: expected {count} points, found {}; excluded",
                polygon.len()
            ));
            return;
        }
        pois.push(QuestPoiSnapshot {
            objective_index: objective as i32,
            map_id: map as u32,
            world_map_area_id: ui_map as u32,
            floor: 0,
            priority: 0,
            flags: flags as u32,
            points: polygon,
        });
    }

    pub fn apply(&self, entries: &mut [QuestEntrySnapshot]) {
        for entry in entries {
            if let Some(pois) = self.quests.get(&entry.quest_id) {
                if entry.pois != *pois {
                    entry.pois.clone_from(pois);
                }
            }
        }
    }
}

fn load_points(dir: &Path) -> Result<HashMap<u32, Vec<QuestPoiPoint>>, String> {
    let mut points: HashMap<u32, Vec<QuestPoiPoint>> = HashMap::new();
    read_numeric_rows(
        &dir.join("QuestPOIPoint.csv"),
        ["QuestPOIBlobID", "X", "Y"],
        |[id, x, y]| {
            points.entry(id as u32).or_default().push(QuestPoiPoint {
                x: x as i32,
                y: y as i32,
            });
        },
    )?;
    Ok(points)
}

fn load_blob_rows(dir: &Path) -> Result<Vec<[i64; 8]>, String> {
    let mut rows = Vec::new();
    read_numeric_rows(
        &dir.join("QuestPOIBlob.csv"),
        [
            "ID",
            "QuestID",
            "MapID",
            "UiMapID",
            "ObjectiveIndex",
            "Flags",
            "NumPoints",
            "PlayerConditionID",
        ],
        |row| rows.push(row),
    )?;
    Ok(rows)
}

/// Quest title / POI controls that participate in Retail's transient highlight.
/// Numeric prefixes must end at a known suffix, not at a different quest's ID.
pub fn hovered_quest_id(name: &str) -> Option<u32> {
    let rest = name
        .strip_prefix("QuestLogTitle")
        .or_else(|| name.strip_prefix("QuestBlock"))?;
    let end = rest.find(|c: char| !c.is_ascii_digit())?;
    let suffix = &rest[end..];
    if suffix == "Text" || suffix == "HeaderText" || suffix.starts_with("POIButton") {
        rest[..end].parse().ok()
    } else {
        None
    }
}

pub fn quest_minimap_blips(
    runtime: &crate::quest_runtime::QuestRuntime,
    selected: Option<u32>,
    map_id: u32,
    view: &game_engine_core::minimap_data::MinimapView,
    map_size: f32,
) -> Vec<crate::minimap::MinimapBlip> {
    use crate::minimap::{BlipKind, MinimapBlip, edge_offset};
    runtime
        .map_entries()
        .iter()
        .enumerate()
        .filter_map(|(index, entry)| {
            let poi = entry
                .pois
                .iter()
                .find(|poi| poi.map_id == map_id && poi_is_visible(entry, poi))?;
            let position = poi_center(poi);
            let (offset, edge) = edge_offset(view, position, map_size);
            if edge && selected != Some(entry.quest_id) {
                return None;
            }
            let kind = if edge {
                BlipKind::QuestSuperTrackArrow
            } else if entry.completed {
                BlipKind::QuestPoiTurnIn
            } else {
                BlipKind::QuestObjective { number: index + 1 }
            };
            Some(MinimapBlip {
                unit: u64::from(entry.quest_id),
                kind,
                offset,
            })
        })
        .collect()
}

/// Authored world XY points expressed in engine XZ; used by pins, not to generate shapes.
fn poi_center(poi: &QuestPoiSnapshot) -> [f32; 2] {
    let count = poi.points.len() as f32;
    [
        poi.points.iter().map(|p| p.x as f32).sum::<f32>() / count,
        -poi.points.iter().map(|p| p.y as f32).sum::<f32>() / count,
    ]
}

/// Navigation markers (32) are not objectives; completed objectives disappear live.
pub fn poi_is_visible(entry: &QuestEntrySnapshot, poi: &QuestPoiSnapshot) -> bool {
    if poi.points.is_empty() {
        return false;
    }
    if entry.completed {
        return poi.objective_index == -1;
    }
    usize::try_from(poi.objective_index)
        .ok()
        .and_then(|index| entry.objectives.get(index))
        .is_some_and(|objective| !objective.completed)
}

#[cfg(test)]
mod tests {
    use super::*;
    fn worg_quest(completed: bool) -> QuestEntrySnapshot {
        QuestEntrySnapshot {
            quest_id: 28766,
            title: "Beating Them Back!".into(),
            zone: String::new(),
            completed,
            repeatability: shared::protocol::QuestRepeatability::Normal,
            level: 1,
            sort_id: 6170,
            objectives_text: String::new(),
            completion_text: String::new(),
            watched: true,
            pois: Vec::new(),
            objectives: vec![shared::protocol::QuestObjectiveSnapshot {
                text: "Blackrock Worg slain".into(),
                current: if completed { 6 } else { 0 },
                required: 6,
                completed,
                kind: shared::protocol::QuestObjectiveKind::Monster,
                object_id: 49871,
            }],
        }
    }
    fn runtime(completed: bool) -> crate::quest_runtime::QuestRuntime {
        let catalog = QuestPoiCatalog::load(
            &Path::new(env!("CARGO_MANIFEST_DIR")).join("../../data/db2/12.1.0.69933"),
        )
        .unwrap();
        let mut runtime = crate::quest_runtime::QuestRuntime::default();
        runtime.log.push(worg_quest(completed));
        runtime.watched.push(28766);
        catalog.apply(&mut runtime.log);
        runtime
    }
    #[test]
    fn minimap_real_worg_objective_changes_to_turnin_and_offscreen_supertrack_arrow() {
        use crate::minimap::BlipKind;
        use game_engine_core::minimap_data::MinimapView;
        let view = MinimapView::new([-8949.95, 132.49], 0);
        let blips = quest_minimap_blips(&runtime(false), Some(28766), 0, &view, 198.0);
        assert_eq!(blips.len(), 1);
        assert_eq!(blips[0].kind, BlipKind::QuestObjective { number: 1 });
        let complete = quest_minimap_blips(&runtime(true), Some(28766), 0, &view, 198.0);
        assert_eq!(complete[0].kind, BlipKind::QuestPoiTurnIn);
        let far = MinimapView::new([-9500.0, 132.49], 0);
        assert!(quest_minimap_blips(&runtime(false), None, 0, &far, 198.0).is_empty());
        let arrow = quest_minimap_blips(&runtime(false), Some(28766), 0, &far, 198.0);
        assert_eq!(arrow[0].kind, BlipKind::QuestSuperTrackArrow);
        let [x, y] = arrow[0].offset;
        assert!((x.hypot(y) - (0.5 - 8.0 / 198.0)).abs() < 0.0001);
        assert!(y < 0.0, "Northshire is north of the player");
    }
    #[test]
    fn server_only_kobold_camp_cleanup_keeps_its_authored_geometry() {
        let catalog = QuestPoiCatalog::load(
            &Path::new(env!("CARGO_MANIFEST_DIR")).join("../../data/db2/12.1.0.69933"),
        )
        .unwrap();
        let mut quest = worg_quest(false);
        quest.quest_id = 7;
        quest.title = "Kobold Camp Cleanup".into();
        quest.objectives[0].object_id = 6;
        quest.objectives[0].required = 8;
        quest.objectives[0].text = "Kobold Vermin slain".into();
        // Actual content_quest_poi_points quest7/blob0, ordered by Idx2.
        let points = [
            (-8797, -259),
            (-8766, -253),
            (-8754, -193),
            (-8751, -160),
            (-8750, -115),
            (-8766, -93),
            (-8795, -117),
            (-8811, -217),
            (-8809, -234),
            (-8806, -244),
        ]
        .into_iter()
        .map(|(x, y)| QuestPoiPoint { x, y })
        .collect();
        quest.pois = vec![QuestPoiSnapshot {
            objective_index: 0,
            map_id: 0,
            world_map_area_id: 30,
            floor: 0,
            priority: 0,
            flags: 1,
            points,
        }];
        let authored = quest.pois.clone();
        let mut entries = [quest];
        catalog.apply(&mut entries);
        assert_eq!(
            entries[0].pois, authored,
            "a quest absent from Retail must retain server POIs"
        );
    }

    #[test]
    fn local_beating_them_back_has_the_authored_seven_point_area_and_turnin() {
        let dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../data/db2/12.1.0.69933");
        let catalog = QuestPoiCatalog::load(&dir).unwrap();
        let pois = catalog.quests.get(&28766).expect("local quest 28766");
        let area = pois.iter().find(|p| p.objective_index == 0).unwrap();
        assert_eq!(area.points.len(), 7);
        assert_eq!((area.points[0].x, area.points[0].y), (-8894, -138));
        let turnin = pois.iter().find(|p| p.objective_index == -1).unwrap();
        assert_eq!((turnin.points[0].x, turnin.points[0].y), (-8913, -137));
    }
}
