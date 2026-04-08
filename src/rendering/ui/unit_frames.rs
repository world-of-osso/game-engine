use bevy::prelude::*;
use shared::components::{Health as NetHealth, Mana as NetMana, Npc, Player as NetPlayer};

use crate::client_options::HudVisibilityToggles;
use crate::game_state::GameState;
use crate::networking::LocalPlayer;
use game_engine::status::{CharacterStatsSnapshot, RestAreaKindEntry};
use game_engine::targeting::CurrentTarget;
use game_engine::ui::plugin::{UiState, sync_registry_to_primary_window};
use game_engine::ui::screens::inworld_unit_frames_component::{
    InWorldUnitFramesState, PLAYER_HEALTH_BAR_W, TARGET_HEALTH_BAR_W, TARGET_MANA_BAR_W,
    UnitFrameState, default_player_frame_state, fallback_target_frame_state, fill_width,
    format_value_text, inworld_unit_frames_screen, missing_target_name,
};
use ui_toolkit::screen::{Screen, SharedContext};

type UnitComponents<'a> = (
    Option<&'a NetPlayer>,
    Option<&'a NetHealth>,
    Option<&'a NetMana>,
    Option<&'a Npc>,
    Option<&'a Name>,
);

struct InWorldUnitFramesRes {
    screen: Screen,
    shared: SharedContext,
}

unsafe impl Send for InWorldUnitFramesRes {}
unsafe impl Sync for InWorldUnitFramesRes {}

#[derive(Resource)]
struct InWorldUnitFramesWrap(InWorldUnitFramesRes);

#[derive(Resource, Clone, PartialEq)]
struct InWorldUnitFramesModel(InWorldUnitFramesState);

pub struct InWorldUnitFramesPlugin;

impl Plugin for InWorldUnitFramesPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(OnEnter(GameState::InWorld), build_inworld_unit_frames_ui);
        app.add_systems(OnExit(GameState::InWorld), teardown_inworld_unit_frames_ui);
        app.add_systems(
            Update,
            (
                sync_inworld_unit_frames_root_size,
                sync_inworld_unit_frames_ui,
            )
                .run_if(in_state(GameState::InWorld)),
        );
    }
}

fn build_inworld_unit_frames_ui(
    mut ui: ResMut<UiState>,
    mut commands: Commands,
    windows: Query<&Window, With<bevy::window::PrimaryWindow>>,
    player_query: Query<UnitComponents, With<LocalPlayer>>,
    entity_query: Query<UnitComponents>,
    character_stats: Option<Res<CharacterStatsSnapshot>>,
    current_target: Res<CurrentTarget>,
    hud_visibility: Option<Res<HudVisibilityToggles>>,
) {
    sync_registry_to_primary_window(&mut ui.registry, &windows);
    let state = build_state(
        character_stats.as_deref(),
        &current_target,
        &player_query,
        &entity_query,
        hud_visibility.as_deref(),
    );
    let mut shared = SharedContext::new();
    shared.insert(state.clone());
    let mut screen = Screen::new(inworld_unit_frames_screen);
    screen.sync(&shared, &mut ui.registry);
    commands.insert_resource(InWorldUnitFramesWrap(InWorldUnitFramesRes {
        screen,
        shared,
    }));
    commands.insert_resource(InWorldUnitFramesModel(state));
}

fn teardown_inworld_unit_frames_ui(
    mut ui: ResMut<UiState>,
    mut commands: Commands,
    mut screen: Option<ResMut<InWorldUnitFramesWrap>>,
) {
    if let Some(res) = screen.as_mut() {
        res.0.screen.teardown(&mut ui.registry);
    }
    commands.remove_resource::<InWorldUnitFramesWrap>();
    commands.remove_resource::<InWorldUnitFramesModel>();
}

fn sync_inworld_unit_frames_root_size(
    mut ui: ResMut<UiState>,
    windows: Query<&Window, With<bevy::window::PrimaryWindow>>,
) {
    sync_registry_to_primary_window(&mut ui.registry, &windows);
}

fn sync_inworld_unit_frames_ui(
    mut ui: ResMut<UiState>,
    mut screen_wrap: Option<ResMut<InWorldUnitFramesWrap>>,
    mut last_model: Option<ResMut<InWorldUnitFramesModel>>,
    player_query: Query<UnitComponents, With<LocalPlayer>>,
    entity_query: Query<UnitComponents>,
    character_stats: Option<Res<CharacterStatsSnapshot>>,
    current_target: Res<CurrentTarget>,
    hud_visibility: Option<Res<HudVisibilityToggles>>,
) {
    let (Some(mut screen_wrap), Some(mut last_model)) = (screen_wrap.take(), last_model.take())
    else {
        return;
    };
    let state = build_state(
        character_stats.as_deref(),
        &current_target,
        &player_query,
        &entity_query,
        hud_visibility.as_deref(),
    );
    if last_model.0 == state {
        return;
    }
    last_model.0 = state.clone();
    let res = &mut screen_wrap.0;
    res.shared.insert(state);
    res.screen.sync(&res.shared, &mut ui.registry);
}

fn build_state(
    character_stats: Option<&CharacterStatsSnapshot>,
    current_target: &CurrentTarget,
    player_query: &Query<UnitComponents, With<LocalPlayer>>,
    entity_query: &Query<UnitComponents>,
    hud_visibility: Option<&HudVisibilityToggles>,
) -> InWorldUnitFramesState {
    let visibility = hud_visibility.cloned().unwrap_or_default();
    let player = player_query
        .iter()
        .next()
        .map(|unit| build_player_state(character_stats, unit))
        .unwrap_or_else(default_player_frame_state);
    let target = current_target
        .0
        .and_then(|entity| entity_query.get(entity).ok())
        .map(build_target_state);
    InWorldUnitFramesState {
        show_player_frame: visibility.show_player_frame,
        show_target_frame: visibility.show_target_frame,
        player,
        target,
    }
}

fn build_player_state(
    character_stats: Option<&CharacterStatsSnapshot>,
    (player, health, mana, _npc, name): UnitComponents,
) -> UnitFrameState {
    let mut state = default_player_frame_state();
    state.name = player
        .map(|player| player.name.clone())
        .or_else(|| character_stats.and_then(|stats| stats.name.clone()))
        .or_else(|| name.map(|name| name.as_str().to_string()))
        .unwrap_or_else(|| "Player".to_string());
    state.level_text = character_stats
        .and_then(|stats| stats.level)
        .map(|level| level.to_string())
        .unwrap_or_default();
    state.resting_text = character_stats.map(resting_text).unwrap_or_default();
    state.health_text = format_value_text(
        health.map(|health| health.current),
        health.map(|health| health.max),
    );
    state.mana_text = format_value_text(mana.map(|mana| mana.current), mana.map(|mana| mana.max));
    state.health_fill_width = fill_width(
        PLAYER_HEALTH_BAR_W,
        health.map(|health| health.current),
        health.map(|health| health.max),
    );
    state.mana_fill_width = fill_width(
        PLAYER_HEALTH_BAR_W,
        mana.map(|mana| mana.current),
        mana.map(|mana| mana.max),
    );
    state.has_mana = mana.is_some();
    state.show_combat_icon = character_stats.is_some_and(|stats| stats.in_combat);
    state.show_resting_icon = character_stats.is_some_and(|stats| stats.in_rest_area);
    state
}

fn build_target_state((player, health, mana, npc, name): UnitComponents) -> UnitFrameState {
    let mut state = fallback_target_frame_state();
    state.name = player
        .map(|player| player.name.clone())
        .or_else(|| npc.map(|npc| format!("Creature {}", npc.template_id)))
        .or_else(|| name.map(|name| name.as_str().to_string()))
        .unwrap_or_else(|| missing_target_name().to_string());
    state.health_text = format_value_text(
        health.map(|health| health.current),
        health.map(|health| health.max),
    );
    state.mana_text = format_value_text(mana.map(|mana| mana.current), mana.map(|mana| mana.max));
    state.health_fill_width = fill_width(
        TARGET_HEALTH_BAR_W,
        health.map(|health| health.current),
        health.map(|health| health.max),
    );
    state.mana_fill_width = fill_width(
        TARGET_MANA_BAR_W,
        mana.map(|mana| mana.current),
        mana.map(|mana| mana.max),
    );
    state.has_mana = mana.is_some();
    state
}

fn resting_text(stats: &CharacterStatsSnapshot) -> String {
    if stats.in_rest_area {
        return "Resting".into();
    }
    if stats.rested_xp > 0 {
        return match stats.rest_area_kind {
            Some(RestAreaKindEntry::City) => "Rested (city)".into(),
            Some(RestAreaKindEntry::Inn) => "Rested (inn)".into(),
            None => "Rested".into(),
        };
    }
    String::new()
}

#[cfg(test)]
mod tests {
    use super::*;
    use bevy::window::PrimaryWindow;
    use game_engine::targeting::CurrentTarget;
    use game_engine::ui::plugin::UiState;
    use game_engine::ui::{event::EventBus, registry::FrameRegistry};

    #[test]
    fn target_state_uses_player_name_when_available() {
        let player = NetPlayer {
            name: "Thrall".to_string(),
            race: 0,
            class: 0,
            appearance: default(),
        };
        let state = build_target_state((Some(&player), None, None, None, None));
        assert_eq!(state.name, "Thrall");
    }

    #[test]
    fn target_state_falls_back_to_npc_template_label() {
        let npc = Npc { template_id: 42 };
        let state = build_target_state((None, None, None, Some(&npc), None));
        assert_eq!(state.name, "Creature 42");
    }

    #[test]
    fn player_state_shows_combat_icon_when_snapshot_is_in_combat() {
        let player = NetPlayer {
            name: "Thrall".to_string(),
            race: 0,
            class: 0,
            appearance: default(),
        };
        let health = NetHealth {
            current: 100.0,
            max: 100.0,
        };
        let stats = CharacterStatsSnapshot {
            in_combat: true,
            ..Default::default()
        };

        let state = build_player_state(
            Some(&stats),
            (Some(&player), Some(&health), None, None, None),
        );

        assert!(state.show_combat_icon);
    }

    #[test]
    fn inworld_target_frame_unhides_for_self_target() {
        let mut app = App::new();
        app.add_plugins((MinimalPlugins, bevy::state::app::StatesPlugin));
        app.init_state::<GameState>();
        app.insert_state(GameState::InWorld);
        app.insert_resource(UiState {
            registry: FrameRegistry::new(1920.0, 1080.0),
            event_bus: EventBus::new(),
            focused_frame: None,
        });
        app.insert_resource(CurrentTarget::default());
        app.insert_resource(HudVisibilityToggles::default());
        app.add_plugins(InWorldUnitFramesPlugin);
        let player = app
            .world_mut()
            .spawn((
                LocalPlayer,
                NetPlayer {
                    name: "Theron".to_string(),
                    race: 0,
                    class: 0,
                    appearance: default(),
                },
                NetHealth {
                    current: 100.0,
                    max: 100.0,
                },
                Name::new("Theron"),
            ))
            .id();
        app.world_mut().spawn((
            Window {
                resolution: (1920, 1080).into(),
                ..default()
            },
            PrimaryWindow,
        ));

        app.update();
        assert!(
            target_frame_hidden(&app),
            "target frame should start hidden"
        );

        app.world_mut().resource_mut::<CurrentTarget>().0 = Some(player);
        app.update();

        assert!(
            !target_frame_hidden(&app),
            "target frame should unhide after self-targeting the local player"
        );

        app.world_mut()
            .resource_mut::<HudVisibilityToggles>()
            .show_target_frame = false;
        app.update();

        assert!(
            target_frame_hidden(&app),
            "target frame should hide when HUD toggle is off"
        );
    }

    fn target_frame_hidden(app: &App) -> bool {
        let ui = app.world().resource::<UiState>();
        let target_frame = ui
            .registry
            .get_by_name("TargetFrame")
            .expect("TargetFrame should exist");
        ui.registry
            .get(target_frame)
            .expect("TargetFrame should resolve")
            .hidden
    }
}
