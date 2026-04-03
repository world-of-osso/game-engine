use bevy::prelude::*;

/// The entity currently targeted by the local player.
#[derive(Resource, Default, Debug)]
pub struct CurrentTarget(pub Option<Entity>);
