//! Offline scene-graph regression; synthetic visuals isolate host selection from asset loading.
use super::{WorldUnits, mount, spawn_unit};
use godot::{classes::ProjectSettings, prelude::*};
use shared::components::VehiclePassenger;

const HOST: u64 = 100;
const RIDER: u64 = 200;

#[derive(GodotClass)]
#[class(base=Node3D)]
struct VehicleAttachmentFixture {
    base: Base<Node3D>,
    world: WorldUnits,
}

#[godot_api]
impl INode3D for VehicleAttachmentFixture {
    fn init(base: Base<Node3D>) -> Self {
        let data = ProjectSettings::singleton().globalize_path("res://../data");
        Self {
            base,
            world: WorldUnits::new(data.to_string().into()),
        }
    }

    fn exit_tree(&mut self) {
        self.world.reset();
    }
}

#[godot_api]
impl VehicleAttachmentFixture {
    #[func]
    fn initialize(&mut self, npc: bool) -> GString {
        self.create_units(npc)
            .err()
            .map_or_else(GString::new, |error| GString::from(error.as_str()))
    }

    #[func]
    fn advance_host(&mut self, position: Vector3, yaw: f32) {
        let host = self.world.units.get_mut(&HOST).expect("Fixture host");
        host.motion.target.position = glam::Vec3::new(position.x, position.y, position.z);
        host.motion.target.yaw = Some(yaw);
        self.world.advance(1.0);
    }

    #[func]
    fn claim_driver(&mut self) -> Dictionary {
        self.world.local_player_id = Some(RIDER);
        let rider = self.world.units.get_mut(&RIDER).expect("Fixture rider");
        rider.motion.control = Some(shared::components::MovementControl {
            controlled: true,
            epoch: 7,
        });
        rider.driver_movement = Some(shared::components::VehicleDriverMovement {
            flags: 1594726007,
            turn_speed: 1.04719758,
            walk_speed: 3.0,
            run_speed: 0.98571 * 7.0,
        });
        self.mover_state()
    }

    #[func]
    fn mover_state(&self) -> Dictionary {
        let mut state = Dictionary::new();
        state.set("vehicle", self.world.local_vehicle_movement().is_some());
        state.set("blocked", self.world.local_input_blocked());
        state.set("mover", self.world.local_mover_node().unwrap().get_name());
        state
    }

    #[func]
    fn release_rider(&mut self) -> GString {
        let rider = self.world.units.get_mut(&RIDER).expect("Fixture rider");
        mount::sync_passenger(rider, None, &self.world.data_root)
            .err()
            .map_or_else(GString::new, |error| GString::from(error.as_str()))
    }
}

impl VehicleAttachmentFixture {
    fn create_units(&mut self, npc: bool) -> Result<(), String> {
        self.world.reset();
        let mut parent = self.base().clone();
        let mut host = spawn_unit(
            &mut self.world.root,
            &mut parent,
            "Host",
            !npc,
            Vector3::ZERO,
            0.0,
        );
        let mut visual = child(&mut host.node, "NpcVisualRoot");
        let mut model = child(&mut visual, "NpcModel");
        let mut skeleton = child(&mut model, "Skeleton3D");
        let seat_id = if npc { 5766 } else { 2764 };
        let seat =
            game_engine_core::vehicle_seat::read_vehicle_seat(&self.world.data_root, seat_id)?;
        let mut bone = child(&mut skeleton, &format!("AttachmentBone{}", seat.attachment));
        let mut point = child(&mut bone, &format!("Attachment{}", seat.attachment));
        // Nonzero, rotated bone separates host selection from the unit's logical position.
        point.set_position(Vector3::new(1.25, 4.5, -2.75));
        point.set_rotation(Vector3::new(0.0, 0.4, 0.0));
        if npc {
            host.visual = Some(visual);
        } else {
            host.mount = Some(mount::UnitMount {
                // Installed Mount280 / MountXDisplay422 (Traveler's Tundra Mammoth).
                mount_id: 280,
                display_id: 27237,
                request: None,
                node: Some(visual),
            });
        }
        let mut rider = spawn_unit(
            &mut self.world.root,
            &mut parent,
            "Rider",
            true,
            Vector3::ZERO,
            0.0,
        );
        rider.visual = Some(child(&mut rider.node, "PlayerModel"));
        mount::sync_passenger(
            &mut rider,
            Some(&VehiclePassenger {
                driver: HOST,
                seat_index: if npc { 0 } else { 1 },
                seat_id,
            }),
            &self.world.data_root,
        )?;
        self.world.units.insert(HOST, host);
        self.world.units.insert(RIDER, rider);
        Ok(())
    }
}

fn child(parent: &mut Gd<Node3D>, name: &str) -> Gd<Node3D> {
    let mut node = Node3D::new_alloc();
    node.set_name(name);
    parent.add_child(&node);
    node
}
