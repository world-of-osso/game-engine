//! Ownership used by screen-space nameplate hit testing.
use bevy::prelude::*;

#[derive(Component, Clone, Copy)]
pub(crate) struct NameplateHitTarget(pub Entity);
