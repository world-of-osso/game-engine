//! Client source-route clock and local passenger carrying, independent of Godot nodes.
use shared::{
    components::TransportState,
    transport_route::{TransportPose, TransportRoute},
};

pub(crate) struct TransportView {
    source: TransportState,
    route: TransportRoute,
    elapsed_ms: f64,
}

impl TransportView {
    pub fn new(source: &TransportState) -> Result<Self, String> {
        let route = TransportRoute::new(source.knots.clone(), source.speed)
            .map_err(|error| format!("transport {} route: {error:?}", source.spawn_id))?;
        Ok(Self {
            source: source.clone(),
            route,
            elapsed_ms: source.elapsed_ms as f64,
        })
    }
    pub fn observe(&mut self, source: &TransportState) -> Result<(), String> {
        if source.path_id != self.source.path_id
            || source.speed != self.source.speed
            || source.knots != self.source.knots
        {
            *self = Self::new(source)?;
            return Ok(());
        }
        self.elapsed_ms = self.elapsed_ms.max(source.elapsed_ms as f64);
        self.source = source.clone();
        Ok(())
    }
    pub fn advance(&mut self, delta: f32) {
        self.elapsed_ms += f64::from(delta) * 1000.0;
    }
    pub fn pose(&self) -> TransportPose {
        self.route.pose_at(self.elapsed_ms as u64)
    }
}

pub(crate) struct LocalTransport {
    pub ship: u64,
    pub epoch: u32,
    pub attached: bool,
    pub pending_disembark: bool,
    previous: TransportPose,
}

impl LocalTransport {
    pub fn new(ship: u64, epoch: u32, pose: TransportPose) -> Self {
        Self {
            ship,
            epoch,
            attached: true,
            pending_disembark: false,
            previous: pose,
        }
    }
    pub fn carry(&mut self, world: TransportPose, ship: TransportPose) -> TransportPose {
        let local = self.previous.world_to_local(world);
        self.previous = ship;
        ship.local_to_world(local)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn state(elapsed_ms: u64) -> TransportState {
        TransportState {
            spawn_id: 35,
            path_id: 6519,
            interior_map: 1856,
            knots: vec![
                [0.0, 0.0, 297.0],
                [10.0, 0.0, 297.0],
                [20.0, 0.0, 297.0],
                [30.0, 0.0, 297.0],
            ],
            speed: 4.0,
            elapsed_ms,
        }
    }
    #[test]
    fn source_route_clock_advances_between_updates_and_does_not_rewind_on_late_receipts() {
        let mut view = TransportView::new(&state(0)).unwrap();
        view.advance(0.25);
        assert!((view.pose().position[0] - 11.0).abs() < 0.0001);
        view.observe(&state(100)).unwrap();
        assert!((view.pose().position[0] - 11.0).abs() < 0.0001);
        view.observe(&state(500)).unwrap();
        assert!((view.pose().position[0] - 12.0).abs() < 0.0001);
    }
    #[test]
    fn idle_and_predicted_walking_feet_follow_ship_translation_and_rotation() {
        let first = TransportPose {
            position: [100.0, 200.0, 297.0],
            orientation: 0.0,
        };
        let later = TransportPose {
            position: [110.0, 200.0, 297.0],
            orientation: std::f32::consts::FRAC_PI_2,
        };
        let local = TransportPose {
            position: [-29.8623, 0.520232, 20.5534],
            orientation: 0.25,
        };
        let mut carrier = LocalTransport::new(200, 3, first);
        let carried = carrier.carry(first.local_to_world(local), later);
        let expected = later.local_to_world(local);
        assert_eq!(carried, expected);
        let mut walked = carried;
        walked.position[1] += 1.0;
        let last = TransportPose {
            position: [115.0, 200.0, 297.0],
            ..later
        };
        let carried = carrier.carry(walked, last);
        assert!((carried.position[1] - walked.position[1]).abs() < 0.0001);
        assert!((carried.position[0] - walked.position[0] - 5.0).abs() < 0.0001);
    }
}
