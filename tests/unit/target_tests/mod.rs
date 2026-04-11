use super::*;
use crate::networking::ResolvedModelAssetInfo;
use bevy::camera::primitives::Aabb;

#[derive(Resource, Default)]
struct TargetResolutionResult(Option<Entity>);

#[derive(Resource, Default)]
struct TargetCircleSizeResult(f32);

mod ancestor_resolution;
mod click_raycast;
mod interactions;
mod selection;
mod visuals;
