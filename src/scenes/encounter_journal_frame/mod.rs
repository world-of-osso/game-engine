use bevy::prelude::*;
use game_engine::encounter_journal_data::{
    AbilityDef, LootEntry, abilities_for_boss, loot_for_boss,
};
use game_engine::status::EncounterJournalStatusSnapshot;
use game_engine::ui::plugin::{UiState, sync_registry_to_primary_window};
use game_engine::ui::screens::encounter_journal_component::{
    BossAbility, BossEntry, EJTab, EncounterJournalState, InstanceEntry, LootItem,
    encounter_journal_screen,
};
use ui_toolkit::screen::{Screen, SharedContext};

use crate::game_state::GameState;

#[derive(Resource, Default)]
pub struct EncounterJournalFrameOpen(pub bool);

struct EncounterJournalFrameRes {
    screen: Screen,
    shared: SharedContext,
}

unsafe impl Send for EncounterJournalFrameRes {}
unsafe impl Sync for EncounterJournalFrameRes {}

#[derive(Resource)]
struct EncounterJournalFrameWrap(EncounterJournalFrameRes);

#[derive(Resource, Clone, PartialEq)]
struct EncounterJournalFrameModel(EncounterJournalState);

pub struct EncounterJournalFramePlugin;

impl Plugin for EncounterJournalFramePlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<EncounterJournalFrameOpen>();
        app.add_systems(
            OnEnter(GameState::InWorld),
            build_encounter_journal_frame_ui,
        );
        app.add_systems(
            OnExit(GameState::InWorld),
            teardown_encounter_journal_frame_ui,
        );
        app.add_systems(
            Update,
            (
                toggle_encounter_journal_frame,
                sync_encounter_journal_frame_state,
            )
                .run_if(in_state(GameState::InWorld)),
        );
    }
}

fn build_encounter_journal_frame_ui(
    mut ui: ResMut<UiState>,
    mut commands: Commands,
    windows: Query<&Window, With<bevy::window::PrimaryWindow>>,
    snapshot: Option<Res<EncounterJournalStatusSnapshot>>,
    open: Res<EncounterJournalFrameOpen>,
) {
    sync_registry_to_primary_window(&mut ui.registry, &windows);
    let state = build_state(snapshot.as_deref(), &open);
    let mut shared = SharedContext::new();
    shared.insert(state.clone());
    let mut screen = Screen::new(encounter_journal_screen);
    screen.sync(&shared, &mut ui.registry);
    commands.insert_resource(EncounterJournalFrameWrap(EncounterJournalFrameRes {
        screen,
        shared,
    }));
    commands.insert_resource(EncounterJournalFrameModel(state));
}

fn teardown_encounter_journal_frame_ui(
    mut ui: ResMut<UiState>,
    mut commands: Commands,
    mut wrap: Option<ResMut<EncounterJournalFrameWrap>>,
) {
    if let Some(res) = wrap.as_mut() {
        res.0.screen.teardown(&mut ui.registry);
    }
    commands.remove_resource::<EncounterJournalFrameWrap>();
    commands.remove_resource::<EncounterJournalFrameModel>();
}

fn toggle_encounter_journal_frame(
    keys: Res<ButtonInput<KeyCode>>,
    reconnect: Option<Res<crate::networking::ReconnectState>>,
    modal_open: Option<Res<crate::scenes::game_menu::UiModalOpen>>,
    mut open: ResMut<EncounterJournalFrameOpen>,
) {
    if !crate::networking::gameplay_input_allowed(reconnect) || modal_open.is_some() {
        return;
    }
    if keys.just_pressed(KeyCode::KeyJ) {
        open.0 = !open.0;
    }
}

fn sync_encounter_journal_frame_state(
    mut ui: ResMut<UiState>,
    mut wrap: Option<ResMut<EncounterJournalFrameWrap>>,
    mut last_model: Option<ResMut<EncounterJournalFrameModel>>,
    snapshot: Option<Res<EncounterJournalStatusSnapshot>>,
    open: Res<EncounterJournalFrameOpen>,
) {
    let (Some(mut wrap), Some(mut last_model)) = (wrap.take(), last_model.take()) else {
        return;
    };
    let state = build_state(snapshot.as_deref(), &open);
    if last_model.0 == state {
        return;
    }
    last_model.0 = state.clone();
    let res = &mut wrap.0;
    res.shared.insert(state);
    res.screen.sync(&res.shared, &mut ui.registry);
}

fn build_state(
    snapshot: Option<&EncounterJournalStatusSnapshot>,
    open: &EncounterJournalFrameOpen,
) -> EncounterJournalState {
    let tabs = vec![
        EJTab {
            name: "Dungeons".into(),
            active: true,
        },
        EJTab {
            name: "Raids".into(),
            active: false,
        },
        EJTab {
            name: "Tier".into(),
            active: false,
        },
    ];
    let Some(snapshot) = snapshot else {
        return EncounterJournalState {
            visible: open.0,
            tabs,
            ..EncounterJournalState::default()
        };
    };
    let visible_instances = snapshot
        .instances
        .iter()
        .filter(|instance| instance.instance_type == "Dungeon")
        .collect::<Vec<_>>();
    let selected_instance = visible_instances
        .iter()
        .find(|instance| instance.bosses.iter().any(|boss| boss.ability_count > 0))
        .copied()
        .or_else(|| visible_instances.first().copied());
    let selected_boss = selected_instance.and_then(|instance| {
        instance
            .bosses
            .iter()
            .find(|boss| boss.ability_count > 0 || boss.loot_count > 0)
            .or_else(|| instance.bosses.first())
    });
    let abilities = selected_boss
        .map(|boss| map_abilities(abilities_for_boss(boss.entry)))
        .unwrap_or_default();
    let loot_items = selected_boss
        .map(|boss| map_loot(loot_for_boss(boss.entry)))
        .unwrap_or_default();
    EncounterJournalState {
        visible: open.0,
        tabs,
        instances: visible_instances
            .iter()
            .map(|instance| InstanceEntry {
                name: instance.name.clone(),
                selected: Some(instance.instance_id)
                    == selected_instance.map(|instance| instance.instance_id),
            })
            .collect(),
        bosses: selected_instance
            .map(|instance| {
                instance
                    .bosses
                    .iter()
                    .map(|boss| BossEntry {
                        name: boss.name.clone(),
                        selected: Some(boss.entry) == selected_boss.map(|boss| boss.entry),
                    })
                    .collect()
            })
            .unwrap_or_default(),
        selected_boss_name: selected_boss
            .map(|boss| boss.name.clone())
            .unwrap_or_default(),
        abilities,
        loot_items,
        loot_slot_filter: "All Slots".into(),
        loot_class_filter: "All Classes".into(),
    }
}

fn map_abilities(entries: Vec<&AbilityDef>) -> Vec<BossAbility> {
    entries
        .into_iter()
        .map(|entry| BossAbility {
            name: entry.name.to_string(),
            description: entry.description.to_string(),
            icon_fdid: entry.icon_fdid,
        })
        .collect()
}

fn map_loot(entries: Vec<&LootEntry>) -> Vec<LootItem> {
    entries
        .into_iter()
        .map(|entry| LootItem {
            name: entry.item_name.to_string(),
            slot: entry.slot.to_string(),
            drop_pct: format!("{}%", entry.drop_pct),
            icon_fdid: entry.icon_fdid,
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use game_engine::status::{EncounterJournalBossEntry, EncounterJournalInstanceEntry};

    #[test]
    fn build_state_prefers_dungeon_boss_with_detail_data() {
        let state = build_state(
            Some(&EncounterJournalStatusSnapshot {
                instances: vec![
                    EncounterJournalInstanceEntry {
                        instance_id: 1,
                        name: "The Deadmines".into(),
                        instance_type: "Dungeon".into(),
                        tier: "Classic".into(),
                        source: "world.db:test".into(),
                        bosses: vec![
                            EncounterJournalBossEntry {
                                entry: 639,
                                name: "Edwin VanCleef".into(),
                                min_level: 20,
                                max_level: 20,
                                rank: 1,
                                ability_count: 2,
                                loot_count: 2,
                            },
                            EncounterJournalBossEntry {
                                entry: 645,
                                name: "Cookie".into(),
                                min_level: 20,
                                max_level: 20,
                                rank: 1,
                                ability_count: 1,
                                loot_count: 1,
                            },
                        ],
                    },
                    EncounterJournalInstanceEntry {
                        instance_id: 3,
                        name: "Molten Core".into(),
                        instance_type: "Raid".into(),
                        tier: "Classic".into(),
                        source: "world.db:test".into(),
                        bosses: vec![],
                    },
                ],
                last_error: None,
            }),
            &EncounterJournalFrameOpen(true),
        );

        assert!(state.visible);
        assert_eq!(state.selected_boss_name, "Edwin VanCleef");
        assert_eq!(state.instances.len(), 1);
        assert_eq!(state.bosses.len(), 2);
        assert_eq!(state.abilities.len(), 2);
        assert_eq!(state.loot_items.len(), 2);
    }
}
