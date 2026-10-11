//! Copy-scoped ScenarioObjectiveTracker replacement snapshots.
use shared::protocol::{ScenarioProgress, ScenarioStage};

#[derive(Clone, Debug, Default, PartialEq)]
pub struct ScenarioObjectives {
    pub block: Option<ScenarioStage>,
    map: Option<u32>,
    copy: Option<(u32, u32)>,
}

impl ScenarioObjectives {
    pub fn begin_map(&mut self, map_id: u32) {
        *self = Self {
            map: Some(map_id),
            ..Self::default()
        };
    }

    pub fn apply(&mut self, snapshot: ScenarioProgress) -> bool {
        let copy = (snapshot.instance_id, snapshot.difficulty_id);
        if self.map.is_some_and(|map| map != snapshot.map_id)
            || self.copy.is_some_and(|previous| previous != copy)
        {
            return false;
        }
        self.map = Some(snapshot.map_id);
        self.copy = Some(copy);
        self.block = snapshot.scenario;
        true
    }
}
