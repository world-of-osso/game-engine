use bevy::prelude::*;

use crate::encounter_journal_data::{
    EncounterJournalInstanceData, abilities_for_boss, builtin_encounter_journal,
    load_encounter_journal, loot_for_boss,
};
use crate::status::{
    EncounterJournalBossEntry, EncounterJournalInstanceEntry, EncounterJournalStatusSnapshot,
};

#[derive(Resource, Default)]
struct EncounterJournalLoadState {
    loaded: bool,
}

pub struct EncounterJournalPlugin;

impl Plugin for EncounterJournalPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<EncounterJournalStatusSnapshot>();
        app.init_resource::<EncounterJournalLoadState>();
        app.add_systems(Update, load_encounter_journal_snapshot_once);
    }
}

fn load_encounter_journal_snapshot_once(
    mut load_state: ResMut<EncounterJournalLoadState>,
    mut snapshot: ResMut<EncounterJournalStatusSnapshot>,
) {
    if load_state.loaded {
        return;
    }
    let loaded = match load_encounter_journal() {
        Ok(instances) => {
            snapshot.instances = instances.into_iter().map(map_instance).collect();
            snapshot.last_error = None;
            true
        }
        Err(err) => {
            snapshot.instances = builtin_encounter_journal()
                .into_iter()
                .map(map_instance)
                .collect();
            snapshot.last_error = Some(err);
            true
        }
    };
    load_state.loaded = loaded;
}

pub fn map_instance(instance: EncounterJournalInstanceData) -> EncounterJournalInstanceEntry {
    EncounterJournalInstanceEntry {
        instance_id: instance.instance_id,
        name: instance.name,
        instance_type: format!("{:?}", instance.instance_type),
        tier: instance.tier,
        source: instance.source,
        bosses: instance
            .bosses
            .into_iter()
            .map(|boss| EncounterJournalBossEntry {
                ability_count: abilities_for_boss(boss.entry).len(),
                loot_count: loot_for_boss(boss.entry).len(),
                entry: boss.entry,
                name: boss.name,
                min_level: boss.min_level,
                max_level: boss.max_level,
                rank: boss.rank,
            })
            .collect(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::encounter_journal_data::{EncounterJournalBossData, InstanceType};

    #[test]
    fn map_instance_carries_static_detail_counts() {
        let mapped = map_instance(EncounterJournalInstanceData {
            instance_id: 1,
            name: "The Deadmines".into(),
            instance_type: InstanceType::Dungeon,
            tier: "Classic".into(),
            source: "world.db:test".into(),
            bosses: vec![EncounterJournalBossData {
                entry: 639,
                name: "Edwin VanCleef".into(),
                min_level: 20,
                max_level: 20,
                rank: 1,
            }],
        });

        assert_eq!(mapped.instance_type, "Dungeon");
        assert_eq!(mapped.source, "world.db:test");
        assert_eq!(mapped.bosses[0].ability_count, 2);
        assert_eq!(mapped.bosses[0].loot_count, 2);
    }
}
