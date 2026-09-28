use game_engine_core::loading_readiness::{
    GlobalWmoState, LoadingInput, LoadingReadiness, TileState, evaluate_world_loading,
};

fn input() -> LoadingInput {
    LoadingInput {
        local_player_ready: true,
        map_ready: true,
        global_wmo: GlobalWmoState::None,
        center_tile: TileState::NotRequested,
    }
}

fn assert_readiness(input: LoadingInput, complete: bool, percent: u8, status: &'static str) {
    assert_eq!(
        evaluate_world_loading(input),
        LoadingReadiness {
            complete,
            progress_percent: percent,
            status_text: status,
        }
    );
}

#[test]
fn local_player_and_map_gate_all_terrain_states() {
    let mut state = input();
    state.local_player_ready = false;
    state.map_ready = false;
    state.global_wmo = GlobalWmoState::Spawned;
    state.center_tile = TileState::Loaded;
    assert_readiness(state, false, 35, "Initializing character...");

    state.local_player_ready = true;
    assert_readiness(state, false, 62, "Waiting for terrain...");
}

#[test]
fn global_wmo_state_precedes_destination_tile() {
    let mut state = input();
    state.center_tile = TileState::Loaded;
    state.global_wmo = GlobalWmoState::Pending;
    assert_readiness(state, false, 86, "Loading terrain...");
    state.global_wmo = GlobalWmoState::Failed;
    assert_readiness(state, false, 86, "Terrain failed to load");
    state.global_wmo = GlobalWmoState::Spawned;
    state.center_tile = TileState::Failed;
    assert_readiness(state, true, 100, "Entering world...");
}

#[test]
fn destination_center_tile_controls_readiness_without_global_wmo() {
    let mut state = input();
    assert_readiness(state, false, 74, "Preparing terrain...");
    state.center_tile = TileState::Pending;
    assert_readiness(state, false, 86, "Loading terrain...");
    state.center_tile = TileState::Failed;
    assert_readiness(state, false, 86, "Terrain failed to load");
    state.center_tile = TileState::Loaded;
    assert_readiness(state, true, 100, "Entering world...");
}
