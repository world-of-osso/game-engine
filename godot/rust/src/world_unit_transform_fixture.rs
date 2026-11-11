//! Offline scene-graph regression; synthetic NPC units isolate per-frame transform writes
//! from replication and asset loading.
use super::{WorldUnits, spawn_unit};
use godot::{classes::ProjectSettings, prelude::*};

#[derive(GodotClass)]
#[class(base=Node3D)]
struct UnitTransformFixture {
    base: Base<Node3D>,
    world: WorldUnits,
}

#[godot_api]
impl INode3D for UnitTransformFixture {
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
impl UnitTransformFixture {
    /// One remote NPC per yaw, named `Unit<index>`, facing its own target.
    #[func]
    fn initialize(&mut self, yaws: PackedFloat32Array) {
        self.world.reset();
        let mut parent = self.base().clone();
        for (index, yaw) in yaws.as_slice().iter().enumerate() {
            let position = Vector3::new(index as f32 * 3.0, 0.0, -5.0);
            let unit = spawn_unit(
                &mut self.world.root,
                &mut parent,
                &format!("Unit{index}"),
                false,
                position,
                *yaw,
            );
            self.world.units.insert(index as u64 + 1, unit);
        }
    }

    #[func]
    fn advance(&mut self, delta: f32) {
        self.world.advance(delta);
    }

    #[func]
    fn set_target_yaw(&mut self, index: i64, yaw: f32) {
        let unit = self
            .world
            .units
            .get_mut(&(index as u64 + 1))
            .expect("Fixture unit");
        unit.motion.target.yaw = Some(yaw);
    }
}
