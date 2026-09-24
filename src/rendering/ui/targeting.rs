use bevy::prelude::*;

/// The entity currently targeted by the local player.
#[derive(Resource, Default, Debug)]
pub struct CurrentTarget(pub Option<Entity>);

/// The local player's focus unit, shown by `FocusFrame`.
#[derive(Resource, Default, Debug)]
pub struct FocusTarget(pub Option<Entity>);

/// Request to change the focus unit; `/focus` and `/clearfocus` send this too.
#[derive(Message, Clone, Copy, Debug, PartialEq, Eq)]
pub enum SetFocus {
    /// Focus the current target (`/focus` without a unit).
    CurrentTarget,
    Unit(Entity),
    Clear,
}

/// Applies queued `SetFocus` requests to `FocusTarget`.
pub fn apply_set_focus(
    mut requests: MessageReader<SetFocus>,
    current_target: Res<CurrentTarget>,
    mut focus: ResMut<FocusTarget>,
) {
    for request in requests.read() {
        focus.0 = match *request {
            SetFocus::CurrentTarget => current_target.0,
            SetFocus::Unit(entity) => Some(entity),
            SetFocus::Clear => None,
        };
    }
}
