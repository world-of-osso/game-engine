use bevy::prelude::*;

/// Game state machine controlling which systems are active.
#[derive(States, Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum GameState {
    #[default]
    Login,
    Connecting,
    CharSelect,
    CharCreate,
    Loading,
    InWorld,
}
