use bevy::prelude::*;
use game_engine::casting_data::{ActiveCast, CastType, CastingState};
use game_engine::ui::plugin::{UiState, sync_registry_to_primary_window};
use game_engine::ui::screens::casting_bar_frame_component::{
    CastingBarState, casting_bar_frame_screen,
};
use shared::casting::CastState as ReplicatedCast;
use ui_toolkit::screen::{Screen, SharedContext};

use crate::game::inworld_scene_stage::inworld_scene_stage_allows_ui;
use crate::game_state::GameState;
use crate::networking::LocalPlayer;

struct CastingBarFrameRes {
    screen: Screen,
    shared: SharedContext,
}

unsafe impl Send for CastingBarFrameRes {}
unsafe impl Sync for CastingBarFrameRes {}

#[derive(Resource)]
struct CastingBarFrameWrap(CastingBarFrameRes);

#[derive(Resource, Clone, PartialEq)]
struct CastingBarFrameModel(CastingBarState);

pub struct CastingBarFramePlugin;

impl Plugin for CastingBarFramePlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<CastingState>();
        app.add_systems(
            OnEnter(GameState::InWorld),
            build_casting_bar_ui.run_if(inworld_scene_stage_allows_ui),
        );
        app.add_systems(OnExit(GameState::InWorld), teardown_casting_bar_ui);
        app.add_systems(
            Update,
            (
                sync_replicated_local_cast,
                tick_casting_state,
                sync_casting_bar_state,
            )
                .chain()
                .run_if(in_state(GameState::InWorld))
                .run_if(inworld_scene_stage_allows_ui),
        );
    }
}

fn build_casting_bar_ui(
    mut ui: ResMut<UiState>,
    mut commands: Commands,
    windows: Query<&Window, With<bevy::window::PrimaryWindow>>,
    casting: Res<CastingState>,
) {
    sync_registry_to_primary_window(&mut ui.registry, &windows);
    let state = build_state(&casting);
    let mut shared = SharedContext::new();
    shared.insert(state.clone());
    let mut screen = Screen::new(casting_bar_frame_screen);
    screen.sync(&shared, &mut ui.registry);
    commands.insert_resource(CastingBarFrameWrap(CastingBarFrameRes { screen, shared }));
    commands.insert_resource(CastingBarFrameModel(state));
}

fn teardown_casting_bar_ui(
    mut ui: ResMut<UiState>,
    mut commands: Commands,
    mut wrap: Option<ResMut<CastingBarFrameWrap>>,
) {
    if let Some(res) = wrap.as_mut() {
        res.0.screen.teardown(&mut ui.registry);
    }
    commands.remove_resource::<CastingBarFrameWrap>();
    commands.remove_resource::<CastingBarFrameModel>();
}

/// Server casts arrive as the local player's replicated `CastState`; between snapshots the
/// bar advances locally. Gather casts are client-started and survive while no server cast exists.
fn sync_replicated_local_cast(
    local: Query<Option<Ref<ReplicatedCast>>, With<LocalPlayer>>,
    mut casting: ResMut<CastingState>,
) {
    match local.single().ok().flatten() {
        Some(cast) if cast.is_changed() => {
            casting.apply_server_cast(ActiveCast::from_replicated(&cast));
        }
        Some(_) => {}
        None => casting.end_server_cast(),
    }
}

fn tick_casting_state(time: Res<Time>, mut casting: ResMut<CastingState>) {
    if casting.active.is_none() && casting.interrupted.is_none() {
        return;
    }
    casting.tick(time.delta_secs());
    casting.clear_finished();
}

fn sync_casting_bar_state(
    mut ui: ResMut<UiState>,
    mut wrap: Option<ResMut<CastingBarFrameWrap>>,
    mut last_model: Option<ResMut<CastingBarFrameModel>>,
    casting: Res<CastingState>,
) {
    let (Some(mut wrap), Some(mut last_model)) = (wrap.take(), last_model.take()) else {
        return;
    };
    let state = build_state(&casting);
    if last_model.0 == state {
        return;
    }
    last_model.0 = state.clone();
    let res = &mut wrap.0;
    res.shared.insert(state);
    res.screen.sync(&res.shared, &mut ui.registry);
}

fn build_state(casting: &CastingState) -> CastingBarState {
    let Some(cast) = casting.active.as_ref() else {
        if casting.interrupted.is_some() {
            return CastingBarState::interrupted();
        }
        return CastingBarState::default();
    };
    CastingBarState {
        visible: true,
        spell_name: cast.spell_name.clone(),
        timer_text: cast.timer_text(),
        progress: cast.progress(),
        is_channel: cast.cast_type == CastType::Channel,
        is_interruptible: cast.interruptible,
        is_interrupted: false,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use bevy::ecs::system::RunSystemOnce;
    use std::time::Duration;

    #[test]
    fn build_state_hides_bar_without_active_cast() {
        let state = build_state(&CastingState::default());
        assert_eq!(state, CastingBarState::default());
    }

    #[test]
    fn build_state_maps_active_cast_into_ui_state() {
        let state = build_state(&CastingState {
            active: Some(ActiveCast {
                spell_name: "Mining Copper Vein".into(),
                spell_id: 0,
                icon_fdid: 0,
                cast_type: CastType::Cast,
                interruptible: true,
                duration: 1.5,
                elapsed: 0.5,
            }),
            ..default()
        });

        assert!(state.visible);
        assert_eq!(state.spell_name, "Mining Copper Vein");
        assert_eq!(state.timer_text, "1.0");
        assert!((state.progress - (1.0 / 3.0)).abs() < 0.01);
        assert!(!state.is_channel);
        assert!(state.is_interruptible);
    }

    #[test]
    fn tick_casting_state_clears_finished_casts() {
        let mut app = App::new();
        app.world_mut().insert_resource(Time::<()>::default());
        app.world_mut().init_resource::<CastingState>();
        app.world_mut()
            .resource_mut::<CastingState>()
            .start(ActiveCast {
                spell_name: "Mining Copper Vein".into(),
                spell_id: 0,
                icon_fdid: 0,
                cast_type: CastType::Cast,
                interruptible: true,
                duration: 0.1,
                elapsed: 0.0,
            });
        app.world_mut()
            .resource_mut::<Time>()
            .advance_by(Duration::from_secs_f32(0.2));

        app.world_mut()
            .run_system_once(tick_casting_state)
            .expect("tick_casting_state should run");

        assert!(app.world().resource::<CastingState>().active.is_none());
    }

    fn cast_bar_app() -> (App, Entity) {
        let mut app = App::new();
        app.insert_resource(Time::<()>::default());
        app.init_resource::<CastingState>();
        app.add_systems(
            Update,
            (sync_replicated_local_cast, tick_casting_state).chain(),
        );
        let player = app.world_mut().spawn(LocalPlayer).id();
        (app, player)
    }

    fn fireball(elapsed: f32) -> ReplicatedCast {
        let mut cast = ReplicatedCast::normal(133, 0, 2.5, true);
        cast.spell_name = "Fireball".into();
        cast.elapsed = elapsed;
        cast
    }

    fn bar(app: &App) -> CastingBarState {
        build_state(app.world().resource::<CastingState>())
    }

    #[test]
    fn replicated_local_cast_drives_bar_until_removed() {
        let (mut app, player) = cast_bar_app();
        app.world_mut().entity_mut(player).insert(fireball(1.0));
        app.update();
        let state = bar(&app);
        assert!(state.visible);
        assert_eq!(state.spell_name, "Fireball");
        assert_eq!(state.timer_text, "1.5");
        assert!((state.progress - 0.4).abs() < 0.01);
        assert!(!state.is_channel);
        // Between snapshots the bar advances locally.
        app.world_mut()
            .resource_mut::<Time>()
            .advance_by(Duration::from_secs_f32(0.5));
        app.update();
        assert_eq!(bar(&app).timer_text, "1.0");
        // A server resync overrides local progress; finishing locally keeps the bar.
        app.world_mut().entity_mut(player).insert(fireball(2.5));
        app.update();
        assert!(bar(&app).visible);
        app.world_mut()
            .entity_mut(player)
            .remove::<ReplicatedCast>();
        app.update();
        assert_eq!(bar(&app), CastingBarState::default());
    }

    #[test]
    fn remote_unit_cast_does_not_drive_player_bar() {
        let (mut app, _) = cast_bar_app();
        app.world_mut().spawn(fireball(0.0));
        app.update();
        assert!(!bar(&app).visible);
    }

    #[test]
    fn gather_cast_survives_without_server_cast() {
        let (mut app, _) = cast_bar_app();
        app.world_mut()
            .resource_mut::<CastingState>()
            .start(ActiveCast {
                spell_name: "Mining Copper Vein".into(),
                spell_id: 0,
                icon_fdid: 0,
                cast_type: CastType::Cast,
                interruptible: true,
                duration: 1.5,
                elapsed: 0.0,
            });
        app.update();
        assert_eq!(bar(&app).spell_name, "Mining Copper Vein");
    }

    #[test]
    fn interrupt_shows_interrupted_bar_then_hides() {
        let (mut app, player) = cast_bar_app();
        app.world_mut().entity_mut(player).insert(fireball(1.0));
        app.update();
        app.world_mut().resource_mut::<CastingState>().interrupt();
        app.world_mut()
            .entity_mut(player)
            .remove::<ReplicatedCast>();
        app.update();
        assert_eq!(bar(&app), CastingBarState::interrupted());
        app.world_mut()
            .resource_mut::<Time>()
            .advance_by(Duration::from_secs_f32(1.1));
        app.update();
        assert_eq!(bar(&app), CastingBarState::default());
    }
}
