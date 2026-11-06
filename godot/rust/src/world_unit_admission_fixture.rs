//! Offline scene-graph regression; synthetic units with box meshes isolate portal
//! admission from replication and asset loading.
use super::{WorldUnits, spawn_unit};
use crate::wmo::portals::HalfSpace;
use glam::Vec3;
use godot::{
    classes::{
        BoxMesh, MeshInstance3D, ProjectSettings, geometry_instance_3d::ShadowCastingSetting,
    },
    prelude::*,
};

#[derive(GodotClass)]
#[class(base=Node3D)]
struct UnitAdmissionFixture {
    base: Base<Node3D>,
    world: WorldUnits,
}

#[godot_api]
impl INode3D for UnitAdmissionFixture {
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
impl UnitAdmissionFixture {
    /// One remote NPC per position, named `Unit<index>`, each showing a visual.
    #[func]
    fn initialize(&mut self, positions: PackedVector3Array) {
        self.world.reset();
        let mut parent = self.base().clone();
        for (index, position) in positions.as_slice().iter().enumerate() {
            let unit = spawn_unit(
                &mut self.world.root,
                &mut parent,
                &format!("Unit{index}"),
                false,
                *position,
                0.0,
            );
            self.world.units.insert(index as u64 + 1, unit);
            self.replace_visual(index as i64);
        }
    }

    #[func]
    fn set_local_player(&mut self, index: i64) {
        self.world.local_player_id = Some(index as u64 + 1);
    }

    /// A new `NpcVisualRoot/NpcModel` whose 1×2×1 box batches `On`, `DoubleSided` and
    /// `Off` cast shadows as named.
    #[func]
    fn replace_visual(&mut self, index: i64) {
        let unit = self
            .world
            .units
            .get_mut(&(index as u64 + 1))
            .expect("Fixture unit");
        // Free the previous visual first so the new one keeps its node name.
        unit.replace_visual(None);
        let mut visual = Node3D::new_alloc();
        visual.set_name("NpcVisualRoot");
        let mut model = Node3D::new_alloc();
        model.set_name("NpcModel");
        for (name, shadows) in [
            ("On", ShadowCastingSetting::ON),
            ("DoubleSided", ShadowCastingSetting::DOUBLE_SIDED),
            ("Off", ShadowCastingSetting::OFF),
        ] {
            let mut mesh = BoxMesh::new_gd();
            mesh.set_size(Vector3::new(1.0, 2.0, 1.0));
            let mut batch = MeshInstance3D::new_alloc();
            batch.set_name(name);
            batch.set_mesh(&mesh);
            batch.set_cast_shadows_setting(shadows);
            model.add_child(&batch);
        }
        visual.add_child(&model);
        unit.node.add_child(&visual);
        unit.replace_visual(Some(visual));
    }

    /// Moves a unit, as a replicated position does.
    #[func]
    fn place_unit(&mut self, index: i64, position: Vector3) {
        let unit = self
            .world
            .units
            .get_mut(&(index as u64 + 1))
            .expect("Fixture unit");
        unit.node.set_position(position);
    }

    /// Reapplies attachment participation, as attachment gates do when it changes.
    #[func]
    fn refresh_shadows(&mut self, index: i64) {
        let unit = &self.world.units[&(index as u64 + 1)];
        crate::animation::refresh_unit_shadows(&unit.node.clone().upcast());
    }

    /// Culls from an indoor camera whose interior group spans `interior_min..interior_max`
    /// and whose visible exterior windows are each one half space `(normal, d)`;
    /// `outdoor` culls from an outdoor camera instead.
    #[func]
    fn cull(
        &mut self,
        outdoor: bool,
        interior_min: Vector3,
        interior_max: Vector3,
        windows: PackedVector4Array,
    ) {
        let to_glam = |v: Vector3| Vec3::new(v.x, v.y, v.z);
        let windows: Vec<Vec<HalfSpace>> = windows
            .as_slice()
            .iter()
            .map(|plane| {
                vec![HalfSpace {
                    normal: Vec3::new(plane.x, plane.y, plane.z),
                    d: plane.w,
                }]
            })
            .collect();
        let interiors = [(to_glam(interior_min), to_glam(interior_max))];
        self.world
            .cull_outdoor((!outdoor).then_some(windows.as_slice()), &interiors);
    }
}
