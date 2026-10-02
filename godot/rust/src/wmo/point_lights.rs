//! A WMO placement's MOLP group point lights as `OmniLight3D` children of its node, encoded
//! as the M2 point lights are (`assets/m2_lights.rs`): range = attenuation end, start /
//! end in the specular parameter, colour x intensity; the opaque shaders apply retail's
//! squared ramp (`shaders/m2_point_light.gdshaderinc`).

use game_engine_core::wmo;
use godot::{
    classes::{Node3D, OmniLight3D, light_3d},
    prelude::*,
};

use super::assets::NativeWmoAsset;

/// Adds the lights of every group for the active `doodad_sets` under `root`.
pub(crate) fn add_point_lights(root: &mut Gd<Node3D>, asset: &NativeWmoAsset, doodad_sets: &[u16]) {
    for group in &asset.groups {
        let lights = wmo::active_point_lights(&group.group, doodad_sets);
        for (index, (position, light)) in lights.into_iter().enumerate() {
            let [r, g, b] = light.color;
            let mut node = OmniLight3D::new_alloc();
            node.set_name(&format!("Group{}_Light{index}", group.index));
            node.set_position(Vector3::from_array(position));
            node.set_color(Color::from_rgb(r, g, b));
            node.set_param(light_3d::Param::ATTENUATION, 0.0);
            node.set_param(light_3d::Param::RANGE, light.attenuation_end);
            node.set_param(light_3d::Param::SPECULAR, light.start_fraction());
            node.set_shadow(false);
            root.add_child(&node);
        }
    }
}
