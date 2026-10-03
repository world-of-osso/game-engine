use bevy::prelude::Resource;

const MAX_DURATION_SECS: f32 = 60.0;

#[derive(Resource, Default)]
pub struct ScriptedMovement {
    active: Option<ActiveScriptedMovement>,
}

#[derive(Clone, Copy, Debug, PartialEq)]
struct ActiveScriptedMovement {
    remaining_secs: f32,
    facing_yaw: Option<f32>,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ScriptedMovementStep {
    pub duration_secs: f32,
    pub facing_yaw: Option<f32>,
}

impl ScriptedMovement {
    pub fn start(
        &mut self,
        duration_secs: f32,
        heading_degrees: Option<f32>,
    ) -> Result<(), String> {
        if !duration_secs.is_finite()
            || !(0.0..=MAX_DURATION_SECS).contains(&duration_secs)
            || duration_secs == 0.0
        {
            return Err(format!(
                "duration must be finite, positive, and no more than {MAX_DURATION_SECS} seconds"
            ));
        }
        let facing_yaw = heading_degrees.map(degrees_to_radians).transpose()?;
        self.active = Some(ActiveScriptedMovement {
            remaining_secs: duration_secs,
            facing_yaw,
        });
        Ok(())
    }

    pub fn stop(&mut self) {
        self.active = None;
    }

    pub fn next_step(&mut self, frame_secs: f32) -> Option<ScriptedMovementStep> {
        if !frame_secs.is_finite() || frame_secs <= 0.0 {
            return None;
        }
        let active = self.active.as_mut()?;
        let duration_secs = frame_secs.min(active.remaining_secs);
        active.remaining_secs -= duration_secs;
        let step = ScriptedMovementStep {
            duration_secs,
            facing_yaw: active.facing_yaw,
        };
        if active.remaining_secs <= f32::EPSILON {
            self.stop();
        }
        Some(step)
    }
}

fn degrees_to_radians(degrees: f32) -> Result<f32, String> {
    if !degrees.is_finite() {
        return Err("heading must be finite".into());
    }
    Ok(degrees.rem_euclid(360.0).to_radians())
}

#[cfg(test)]
#[path = "../tests/unit/movement_control_tests.rs"]
mod tests;
