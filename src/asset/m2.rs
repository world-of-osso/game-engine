// TODO: Parse M2 files using wow-m2 crate and convert to Bevy Mesh + SkinnedMesh

use bevy::prelude::*;

pub struct WowModel {
    pub name: String,
}

pub struct WowModelPlugin;

impl Plugin for WowModelPlugin {
    fn build(&self, _app: &mut App) {}
}
