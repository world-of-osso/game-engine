//! Missile/projectile tracking data model.
//!
//! Handles projectile visuals that fly from a caster's attachment point to a
//! target position. Each missile has a speed, arc height, and model FDID.
//! The trajectory is a parabolic arc interpolated over time.

use bevy::prelude::*;

/// A missile definition describing the visual and flight parameters.
#[derive(Clone, Debug, PartialEq)]
pub struct MissileDef {
    /// M2 model FDID for the projectile visual.
    pub model_fdid: u32,
    /// Flight speed in world units per second.
    pub speed: f32,
    /// Arc height as a fraction of the total distance (0 = straight line).
    pub arc_height: f32,
    /// Scale of the projectile model.
    pub scale: f32,
    /// Whether the projectile faces its direction of travel.
    pub face_direction: bool,
}

impl Default for MissileDef {
    fn default() -> Self {
        Self {
            model_fdid: 0,
            speed: 20.0,
            arc_height: 0.0,
            scale: 1.0,
            face_direction: true,
        }
    }
}

/// An active missile in flight.
#[derive(Component, Clone, Debug)]
pub struct ActiveMissile {
    pub origin: Vec3,
    pub target: Vec3,
    pub speed: f32,
    pub arc_height: f32,
    pub elapsed: f32,
    pub face_direction: bool,
}

impl ActiveMissile {
    /// Total flight distance.
    pub fn distance(&self) -> f32 {
        self.origin.distance(self.target)
    }

    /// Total flight duration.
    pub fn duration(&self) -> f32 {
        if self.speed <= 0.0 {
            return 0.0;
        }
        self.distance() / self.speed
    }

    /// Flight progress 0.0–1.0.
    pub fn progress(&self) -> f32 {
        let dur = self.duration();
        if dur <= 0.0 {
            return 1.0;
        }
        (self.elapsed / dur).clamp(0.0, 1.0)
    }

    /// Whether the missile has reached its target.
    pub fn is_finished(&self) -> bool {
        self.elapsed >= self.duration()
    }

    /// Current world position along the parabolic arc.
    pub fn position(&self) -> Vec3 {
        let t = self.progress();
        let base = self.origin.lerp(self.target, t);
        let arc_offset = self.arc_height * self.distance() * parabolic_arc(t);
        Vec3::new(base.x, base.y + arc_offset, base.z)
    }

    /// Advance the missile by `dt` seconds.
    pub fn tick(&mut self, dt: f32) {
        self.elapsed = (self.elapsed + dt).min(self.duration());
    }
}

/// Parabolic arc: peaks at t=0.5, zero at t=0 and t=1.
fn parabolic_arc(t: f32) -> f32 {
    4.0 * t * (1.0 - t)
}

/// Queue for spawning new missiles.
#[derive(Resource, Default)]
pub struct MissileSpawnQueue {
    pub pending: Vec<MissileSpawnRequest>,
}

pub struct MissileSpawnRequest {
    pub def: MissileDef,
    pub origin: Vec3,
    pub target: Vec3,
}

impl MissileSpawnQueue {
    pub fn enqueue(&mut self, request: MissileSpawnRequest) {
        self.pending.push(request);
    }

    pub fn drain(&mut self) -> Vec<MissileSpawnRequest> {
        std::mem::take(&mut self.pending)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn straight_missile(origin: Vec3, target: Vec3) -> ActiveMissile {
        ActiveMissile {
            origin,
            target,
            speed: 10.0,
            arc_height: 0.0,
            elapsed: 0.0,
            face_direction: true,
        }
    }

    fn arced_missile(origin: Vec3, target: Vec3) -> ActiveMissile {
        ActiveMissile {
            origin,
            target,
            speed: 10.0,
            arc_height: 0.3,
            elapsed: 0.0,
            face_direction: true,
        }
    }

    // --- Trajectory ---

    #[test]
    fn position_at_start_is_origin() {
        let m = straight_missile(Vec3::ZERO, Vec3::new(10.0, 0.0, 0.0));
        let pos = m.position();
        assert!((pos.x - 0.0).abs() < 0.01);
        assert!((pos.y - 0.0).abs() < 0.01);
    }

    #[test]
    fn position_at_end_is_target() {
        let mut m = straight_missile(Vec3::ZERO, Vec3::new(10.0, 0.0, 0.0));
        m.elapsed = m.duration();
        let pos = m.position();
        assert!((pos.x - 10.0).abs() < 0.01);
    }

    #[test]
    fn position_midway_straight() {
        let mut m = straight_missile(Vec3::ZERO, Vec3::new(10.0, 0.0, 0.0));
        m.elapsed = m.duration() / 2.0;
        let pos = m.position();
        assert!((pos.x - 5.0).abs() < 0.01);
        assert!((pos.y - 0.0).abs() < 0.01); // no arc
    }

    #[test]
    fn arced_missile_peaks_at_midpoint() {
        let mut m = arced_missile(Vec3::ZERO, Vec3::new(10.0, 0.0, 0.0));
        m.elapsed = m.duration() / 2.0;
        let pos = m.position();
        // Arc height = 0.3 * distance * parabolic(0.5) = 0.3 * 10 * 1.0 = 3.0
        assert!((pos.y - 3.0).abs() < 0.01);
        assert!((pos.x - 5.0).abs() < 0.01);
    }

    #[test]
    fn arced_missile_zero_at_endpoints() {
        let m = arced_missile(Vec3::ZERO, Vec3::new(10.0, 0.0, 0.0));
        assert!((m.position().y - 0.0).abs() < 0.01); // start

        let mut m2 = arced_missile(Vec3::ZERO, Vec3::new(10.0, 0.0, 0.0));
        m2.elapsed = m2.duration();
        assert!((m2.position().y - 0.0).abs() < 0.01); // end
    }

    // --- Duration and progress ---

    #[test]
    fn duration_from_distance_and_speed() {
        let m = straight_missile(Vec3::ZERO, Vec3::new(20.0, 0.0, 0.0));
        assert!((m.duration() - 2.0).abs() < 0.01); // 20 / 10 = 2s
    }

    #[test]
    fn duration_zero_speed() {
        let m = ActiveMissile {
            speed: 0.0,
            ..straight_missile(Vec3::ZERO, Vec3::X)
        };
        assert_eq!(m.duration(), 0.0);
        assert_eq!(m.progress(), 1.0);
    }

    #[test]
    fn progress_clamped() {
        let mut m = straight_missile(Vec3::ZERO, Vec3::new(10.0, 0.0, 0.0));
        m.elapsed = 999.0;
        assert_eq!(m.progress(), 1.0);
    }

    #[test]
    fn tick_advances_elapsed() {
        let mut m = straight_missile(Vec3::ZERO, Vec3::new(10.0, 0.0, 0.0));
        m.tick(0.5);
        assert!((m.elapsed - 0.5).abs() < 0.01);
        assert!(!m.is_finished());
    }

    #[test]
    fn tick_clamps_at_duration() {
        let mut m = straight_missile(Vec3::ZERO, Vec3::new(10.0, 0.0, 0.0));
        m.tick(999.0);
        assert!(m.is_finished());
        assert!((m.elapsed - m.duration()).abs() < 0.01);
    }

    #[test]
    fn is_finished_at_target() {
        let mut m = straight_missile(Vec3::ZERO, Vec3::new(10.0, 0.0, 0.0));
        assert!(!m.is_finished());
        m.elapsed = m.duration();
        assert!(m.is_finished());
    }

    // --- Parabolic arc ---

    #[test]
    fn parabolic_arc_endpoints_zero() {
        assert!((parabolic_arc(0.0) - 0.0).abs() < 0.001);
        assert!((parabolic_arc(1.0) - 0.0).abs() < 0.001);
    }

    #[test]
    fn parabolic_arc_peak_at_half() {
        assert!((parabolic_arc(0.5) - 1.0).abs() < 0.001);
    }

    #[test]
    fn parabolic_arc_symmetric() {
        assert!((parabolic_arc(0.25) - parabolic_arc(0.75)).abs() < 0.001);
    }

    // --- MissileDef ---

    #[test]
    fn missile_def_defaults() {
        let def = MissileDef::default();
        assert_eq!(def.speed, 20.0);
        assert_eq!(def.arc_height, 0.0);
        assert!(def.face_direction);
    }

    // --- Queue ---

    #[test]
    fn spawn_queue_enqueue_and_drain() {
        let mut queue = MissileSpawnQueue::default();
        queue.enqueue(MissileSpawnRequest {
            def: MissileDef::default(),
            origin: Vec3::ZERO,
            target: Vec3::X,
        });
        assert_eq!(queue.pending.len(), 1);
        let drained = queue.drain();
        assert_eq!(drained.len(), 1);
        assert!(queue.pending.is_empty());
    }
}
