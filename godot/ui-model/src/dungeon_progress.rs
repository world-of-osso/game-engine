//! Copy-scoped replacement snapshots; optionality is displayed only when verified.
use crate::objective_tracker_component::{DungeonBlock, ObjectiveLine, ObjectiveLineStyle};
use shared::protocol::DungeonProgress;

#[derive(Clone, Debug, Default, PartialEq)]
pub struct DungeonObjectives {
    pub block: Option<DungeonBlock>,
    map: Option<u32>,
    copy: Option<(u32, u32)>,
}
impl DungeonObjectives {
    pub fn begin_map(&mut self, map_id: u32) {
        *self = Self {
            map: Some(map_id),
            ..Self::default()
        };
    }
    pub fn apply(&mut self, snapshot: DungeonProgress, name: String) -> bool {
        if self.map.is_some_and(|map| map != snapshot.map_id)
            || self
                .copy
                .is_some_and(|copy| copy != (snapshot.instance_id, snapshot.difficulty_id))
        {
            return false;
        }
        self.map = Some(snapshot.map_id);
        self.copy = Some((snapshot.instance_id, snapshot.difficulty_id));
        self.block = (!snapshot.encounters.is_empty()).then(|| DungeonBlock {
            name,
            bosses: snapshot
                .encounters
                .into_iter()
                .map(|boss| ObjectiveLine {
                    text: if boss.optional == Some(true) {
                        format!("{} (Optional)", boss.name)
                    } else {
                        boss.name
                    },
                    style: if boss.defeated {
                        ObjectiveLineStyle::Completed
                    } else {
                        ObjectiveLineStyle::InProgress
                    },
                })
                .collect(),
        });
        true
    }
}
