//! Bevy-free world-loading outcome rules shared by both renderers.

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct LoadingReadiness {
    pub complete: bool,
    pub progress_percent: u8,
    pub status_text: &'static str,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GlobalWmoState {
    None,
    Spawned,
    Pending,
    Failed,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TileState {
    NotRequested,
    Loaded,
    Failed,
    Pending,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct LoadingInput {
    pub local_player_ready: bool,
    pub map_ready: bool,
    pub global_wmo: GlobalWmoState,
    pub center_tile: TileState,
}

pub fn evaluate_world_loading(input: LoadingInput) -> LoadingReadiness {
    if !input.local_player_ready {
        return LoadingReadiness {
            complete: false,
            progress_percent: 35,
            status_text: "Initializing character...",
        };
    }

    if !input.map_ready {
        return LoadingReadiness {
            complete: false,
            progress_percent: 62,
            status_text: "Waiting for terrain...",
        };
    }

    match input.global_wmo {
        GlobalWmoState::None => {}
        GlobalWmoState::Spawned => return ready(),
        GlobalWmoState::Pending => return loading(),
        GlobalWmoState::Failed => return failed(),
    }

    match input.center_tile {
        TileState::Loaded => ready(),
        TileState::Failed => failed(),
        TileState::Pending => loading(),
        TileState::NotRequested => LoadingReadiness {
            complete: false,
            progress_percent: 74,
            status_text: "Preparing terrain...",
        },
    }
}

fn ready() -> LoadingReadiness {
    LoadingReadiness {
        complete: true,
        progress_percent: 100,
        status_text: "Entering world...",
    }
}

fn loading() -> LoadingReadiness {
    LoadingReadiness {
        complete: false,
        progress_percent: 86,
        status_text: "Loading terrain...",
    }
}

fn failed() -> LoadingReadiness {
    LoadingReadiness {
        complete: false,
        progress_percent: 86,
        status_text: "Terrain failed to load",
    }
}
