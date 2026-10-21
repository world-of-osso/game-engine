//! Native ship-frame carrying and movement reports, using rendered authored decks.
use crate::{GameClient, transport_motion::LocalTransport};
use godot::prelude::*;
use shared::{components::TransportPassenger, transport_route::TransportPose};

impl GameClient {
    fn replicated_transport_passenger(&self) -> Option<TransportPassenger> {
        self.world
            .local_mover_id()
            .and_then(|id| self.replica.unit(id))
            .and_then(|unit| unit.get::<TransportPassenger>())
            .copied()
    }

    pub(super) fn transport_ready(&self) -> bool {
        self.replicated_transport_passenger()
            .is_none_or(|frame| self.game_objects.transport_pose(frame.transport).is_some())
    }

    pub(super) fn update_transport_motion(&mut self, delta: f32) -> Result<(), String> {
        self.game_objects.advance_transports(delta);
        let Some(frame) = self.replicated_transport_passenger() else {
            self.local_transport = None;
            return Ok(());
        };
        let (Some(ship), Some(epoch), Some(mut player), Some(yaw)) = (
            self.game_objects.transport_pose(frame.transport),
            self.world.local_player_epoch(),
            self.world.local_player_node(),
            self.world.local_player_facing(),
        ) else {
            return Ok(());
        };
        let pose = match &mut self.local_transport {
            Some(carrier) if carrier.ship == frame.transport && carrier.epoch == epoch => {
                if !carrier.attached {
                    return Ok(());
                }
                let at = player.get_position();
                carrier.carry(
                    TransportPose {
                        position: [at.x, -at.z, at.y],
                        orientation: yaw - std::f32::consts::FRAC_PI_2,
                    },
                    ship,
                )
            }
            _ => {
                self.local_transport = Some(LocalTransport::new(frame.transport, epoch, ship));
                ship.local_to_world(TransportPose {
                    position: frame.position,
                    orientation: frame.orientation,
                })
            }
        };
        player.set_position(Vector3::new(
            pose.position[0],
            pose.position[2],
            -pose.position[1],
        ));
        player.set_rotation(Vector3::new(0.0, pose.orientation, 0.0));
        self.world
            .set_local_player_facing(pose.orientation + std::f32::consts::FRAC_PI_2);
        Ok(())
    }

    pub(super) fn transport_input_frame(&mut self) -> Result<Option<TransportPassenger>, String> {
        let Some(carrier) = &mut self.local_transport else {
            return Ok(None);
        };
        if !carrier.attached {
            return Ok(None);
        }
        let player = self
            .world
            .local_player_node()
            .ok_or("transport passenger node unavailable")?;
        let ship = self
            .game_objects
            .transport_pose(carrier.ship)
            .ok_or("rendered ship frame unavailable")?;
        let space = player
            .get_world_3d()
            .and_then(|world| world.get_direct_space_state())
            .ok_or("transport passenger physics space unavailable")?;
        let at = player.get_position();
        let feet = glam::Vec3::new(at.x, at.y, at.z);
        let floor = crate::game_objects::floor_object(&space, feet);
        if floor != Some(carrier.ship) {
            carrier.attached = false;
            carrier.pending_disembark = true;
            return Ok(None);
        }
        let yaw = self
            .world
            .local_player_facing()
            .ok_or("transport passenger facing unavailable")?;
        let local = ship.world_to_local(TransportPose {
            position: [at.x, -at.z, at.y],
            orientation: yaw - std::f32::consts::FRAC_PI_2,
        });
        Ok(Some(TransportPassenger {
            transport: carrier.ship,
            position: local.position,
            orientation: local.orientation,
        }))
    }
}
