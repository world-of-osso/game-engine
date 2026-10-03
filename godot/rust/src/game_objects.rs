//! Real replicated mailbox, Guild Vault and chair M2s, resolved through build-pinned
//! GameObjectDisplayInfo.
use crate::{
    assets::{
        M2_BOUNDS_META, build_model,
        creature::{cache_model_files, cache_model_textures, local_resolver},
        read_model,
    },
    lighting::TerrainLight,
    world_models::bind_visual_light,
};
use game_engine_core::csv_util::parse_csv_line;
use game_engine_network::replica::Unit;
use game_engine_ui_model::wow_cursor_data::ActiveWowCursor;
use godot::{classes::Node3D, prelude::*};
use shared::components::{Position, Rotation};
use shared::protocol::{
    GAMEOBJECT_TYPE_CHAIR, GAMEOBJECT_TYPE_GUILD_BANK, GAMEOBJECT_TYPE_MAILBOX, GameObjectInfo,
};
use std::{collections::HashMap, path::PathBuf};

struct ObjectNode {
    /// The display its loaded visual shows.
    display_id: u32,
    node: Gd<Node3D>,
}
pub(crate) struct GameObjects {
    data_root: PathBuf,
    displays: Option<Result<HashMap<u32, u32>, String>>,
    objects: HashMap<u64, ObjectNode>,
    light: Option<TerrainLight>,
}

fn parse_displays(text: &str) -> Result<HashMap<u32, u32>, String> {
    let mut lines = text.lines();
    let headers = parse_csv_line(lines.next().ok_or("Empty GameObjectDisplayInfo.csv")?);
    let id = headers
        .iter()
        .position(|h| h == "ID")
        .ok_or("GameObjectDisplayInfo lacks ID")?;
    let fdid = headers
        .iter()
        .position(|h| h == "FileDataID")
        .ok_or("GameObjectDisplayInfo lacks FileDataID")?;
    lines
        .filter(|line| !line.trim().is_empty())
        .map(|line| {
            let row = parse_csv_line(line);
            let number = |index: usize| {
                row.get(index)
                    .ok_or_else(|| format!("Incomplete GameObjectDisplayInfo row: {line}"))?
                    .parse::<u32>()
                    .map_err(|e| format!("GameObjectDisplayInfo row {line}: {e}"))
            };
            Ok((number(id)?, number(fdid)?))
        })
        .collect()
}

/// The model's yaw inside its object node.
const MODEL_YAW: f32 = -std::f32::consts::FRAC_PI_2;

/// The cursor a shown game object takes under the pointer and as its soft target icon.
/// Retail sends the template's `IconName` (TrinityCore `GameObjectTemplate::IconName`,
/// GameObjectData.h:67); world.db leaves it empty for the Goldshire Mailbox 142075 and
/// its Wooden Chairs, so the client's type default applies: `interface/cursor/mail.blp`
/// over a mailbox and the gears of `interface/cursor/interact.blp` over any other usable
/// object, as the Bevy client maps them (`cursor_for_interaction`, wow_cursor.rs:141-178).
pub(crate) fn game_object_cursor(go_type: u8) -> Option<ActiveWowCursor> {
    match go_type {
        GAMEOBJECT_TYPE_MAILBOX => Some(ActiveWowCursor::Mail),
        GAMEOBJECT_TYPE_GUILD_BANK | GAMEOBJECT_TYPE_CHAIR => Some(ActiveWowCursor::Interact),
        _ => None,
    }
}

/// The top centre of an M2's bounding box placed by `model` (the model node's global
/// transform).
fn bounds_top(model: Transform3D, bounds: Aabb) -> Vector3 {
    let top = bounds.center() + Vector3::new(0.0, bounds.size.y / 2.0, 0.0);
    model * top
}

impl GameObjects {
    pub fn new(data_root: PathBuf) -> Self {
        Self {
            data_root,
            displays: None,
            objects: HashMap::new(),
            light: None,
        }
    }
    /// Whether `id` is a shown mailbox, Guild Vault or chair.
    pub fn contains(&self, id: u64) -> bool {
        self.objects.contains_key(&id)
    }
    pub fn position(&self, id: u64) -> Option<Vector3> {
        Some(self.objects.get(&id)?.node.get_global_position())
    }
    /// The top centre of the object's model bounds, where its soft target icon sits.
    pub fn icon_anchor(&self, id: u64) -> Option<Vector3> {
        let model = self
            .objects
            .get(&id)?
            .node
            .get_node_or_null("GameObjectModel")?
            .try_cast::<Node3D>()
            .ok()?;
        let bounds = model.get_meta(M2_BOUNDS_META).try_to::<Aabb>().ok()?;
        Some(bounds_top(model.get_global_transform(), bounds))
    }
    pub fn upsert(
        &mut self,
        parent: &mut Gd<Node3D>,
        unit: Unit,
        info: &GameObjectInfo,
    ) -> Result<(), String> {
        let id = unit.server_id;
        if game_object_cursor(info.go_type).is_none() {
            self.remove(id);
            return Ok(());
        }
        let Some(position) = unit.get::<Position>() else {
            return Ok(());
        };
        if !info.scale.is_finite() || info.scale <= 0.0 {
            return Err(format!(
                "Game object {id} has invalid replicated scale {}",
                info.scale
            ));
        }
        let changed = self
            .objects
            .get(&id)
            .is_none_or(|old| old.display_id != info.display_id);
        if changed {
            let visual = self.load_visual(info)?;
            if let Err(error) = crate::targeting::attach_pick_area(&visual, id) {
                visual.free();
                return Err(error);
            }
            self.remove(id);
            let mut node = Node3D::new_alloc();
            node.set_name(&format!("Mailbox_{id}"));
            node.set_meta("game_object_server_id", &(id as i64).to_variant());
            node.set_meta("game_object_name", &info.name.to_variant());
            node.set_meta("game_object_entry", &(info.entry as i64).to_variant());
            node.set_meta(
                "game_object_display_id",
                &(info.display_id as i64).to_variant(),
            );
            node.add_child(&visual);
            parent.add_child(&node);
            self.objects.insert(
                id,
                ObjectNode {
                    display_id: info.display_id,
                    node,
                },
            );
        }
        let object = self
            .objects
            .get_mut(&id)
            .ok_or("Mailbox visual missing after load")?;
        object
            .node
            .set_position(Vector3::new(position.x, position.y, position.z));
        object.node.set_rotation(Vector3::new(
            0.0,
            unit.get::<Rotation>().map_or(0.0, |r| r.y),
            0.0,
        ));
        object.node.set_scale(Vector3::ONE * info.scale);
        Ok(())
    }
    fn load_visual(&mut self, info: &GameObjectInfo) -> Result<Gd<Node3D>, String> {
        let path = self
            .data_root
            .join("db2/12.1.0.69933/GameObjectDisplayInfo.csv");
        let displays = self
            .displays
            .get_or_insert_with(|| {
                std::fs::read_to_string(&path)
                    .map_err(|e| format!("Read {}: {e}", path.display()))
                    .and_then(|text| parse_displays(&text))
            })
            .as_ref()
            .map_err(Clone::clone)?;
        let fdid = displays
            .get(&info.display_id)
            .copied()
            .filter(|id| *id != 0)
            .ok_or_else(|| {
                format!(
                    "Game object entry {} has no GameObjectDisplayInfo {} model FileDataID",
                    info.entry, info.display_id
                )
            })?;
        let resolver = local_resolver(&self.data_root);
        let path = cache_model_files(&resolver, &self.data_root, fdid).map_err(|e| {
            format!(
                "Game object entry {} display {} model {fdid}: {e}",
                info.entry, info.display_id
            )
        })?;
        let path = GString::from(path.to_string_lossy().as_ref());
        let parsed = read_model(&path)?;
        cache_model_textures(&resolver, &self.data_root, &[0; 3], &parsed)?;
        let (mut model, missing) = build_model(&parsed, &path, &[0; 3], None)?;
        if !missing.is_empty() {
            model.free();
            return Err(format!(
                "Game object entry {} model {fdid} missing texture FDIDs: {missing:?}",
                info.entry
            ));
        }
        model.set_name("GameObjectModel");
        model.set_meta("model_file_data_id", &(fdid as i64).to_variant());
        model.set_rotation(Vector3::new(0.0, MODEL_YAW, 0.0));
        bind_visual_light(&model, self.light.as_ref());
        Ok(model)
    }
    pub fn update_lighting(&mut self, light: Option<TerrainLight>) {
        for object in self.objects.values() {
            bind_visual_light(&object.node, light.as_ref());
        }
        self.light = light;
    }
    pub fn remove(&mut self, id: u64) {
        if let Some(object) = self.objects.remove(&id) {
            object.node.free();
        }
    }
    pub fn reset(&mut self) {
        for (_, object) in self.objects.drain() {
            object.node.free();
        }
        self.light = None;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use godot::builtin::EulerOrder;

    /// The model node's global transform as `upsert` places it: the object node at the
    /// replicated position, yaw (orientation + pi/2, game-server `game_object_bundle`) and
    /// scale, and the model rotated by -pi/2 inside it.
    fn placed_model(position: Vector3, orientation: f32, scale: f32) -> Transform3D {
        let yaw = orientation + std::f32::consts::FRAC_PI_2;
        let node = Transform3D::new(
            Basis::from_euler(EulerOrder::YXZ, Vector3::new(0.0, yaw, 0.0))
                .scaled(Vector3::ONE * scale),
            position,
        );
        node * Transform3D::new(
            Basis::from_euler(EulerOrder::YXZ, Vector3::new(0.0, MODEL_YAW, 0.0)),
            Vector3::ZERO,
        )
    }

    /// An M2 header bounding box (WoW model space, Z up) as `m2_bounds` stores it.
    fn m2_box(min: [f32; 3], max: [f32; 3]) -> Aabb {
        let low = Vector3::new(min[0], min[2], -max[1]);
        let high = Vector3::new(max[0], max[2], -min[1]);
        Aabb::new(low, high - low)
    }

    /// WoW `(x, y, z)` in engine coordinates (game-server `wow_to_bevy`).
    fn engine(x: f32, y: f32, z: f32) -> Vector3 {
        Vector3::new(x, z, -y)
    }

    #[test]
    fn mailbox_and_chair_cursors() {
        assert_eq!(
            game_object_cursor(GAMEOBJECT_TYPE_MAILBOX),
            Some(ActiveWowCursor::Mail)
        );
        assert_eq!(
            game_object_cursor(GAMEOBJECT_TYPE_CHAIR),
            Some(ActiveWowCursor::Interact)
        );
        assert_eq!(
            game_object_cursor(GAMEOBJECT_TYPE_GUILD_BANK),
            Some(ActiveWowCursor::Interact)
        );
        // GAMEOBJECT_TYPE_GENERIC decoration is not shown as usable.
        assert_eq!(game_object_cursor(5), None);
    }

    #[test]
    fn icon_anchor_is_the_top_of_the_model_box() {
        // Goldshire Mailbox (gameobject guid 26784, display 1907, model 199999.m2 box
        // -0.516,-0.702,-0.005 .. 0.517,0.721,3.080).
        let (x, y, z, o) = (-9455.99, 45.8229, 56.4395, 1.40499_f32);
        let top = bounds_top(
            placed_model(engine(x, y, z), o, 1.0),
            m2_box([-0.516, -0.702, -0.005], [0.517, 0.721, 3.080]),
        );
        // Box centre (0.0005, 0.0095) turned by the orientation.
        let (cx, cy) = (
            0.0005 * o.cos() - 0.0095 * o.sin(),
            0.0005 * o.sin() + 0.0095 * o.cos(),
        );
        let expected = engine(x + cx, y + cy, z + 3.080);
        assert!(top.distance_to(expected) < 0.001, "{top:?} vs {expected:?}");

        // Goldshire Wooden Chair (guid 26246, orientation 3.77864, display 39, model
        // 198115.m2 box -0.999,-0.553,-0.007 .. 0.179,0.553,1.328): the box centre sits
        // 0.41 yd behind the origin along the chair's facing.
        let (x, y, z, o) = (-9458.4, 16.0574, 56.9748, 3.77864_f32);
        let top = bounds_top(
            placed_model(engine(x, y, z), o, 1.0),
            m2_box([-0.999, -0.553, -0.007], [0.179, 0.553, 1.328]),
        );
        let back = -0.41;
        let expected = engine(x + back * o.cos(), y + back * o.sin(), z + 1.328);
        assert!(top.distance_to(expected) < 0.001, "{top:?} vs {expected:?}");
    }
    #[test]
    fn native_mailbox_display_metadata_uses_file_data_id_not_entry_or_display_id() {
        let rows =
            parse_displays("FileDataID,ID,GeoBox_0\n198004,1727,-0.8\n199999,1907,-0.5\n").unwrap();
        assert_eq!(rows.get(&1727), Some(&198004));
        assert_eq!(rows.get(&1907), Some(&199999));
        assert_eq!(rows.get(&140907), None);
        assert!(parse_displays("ID,Model\n1727,198004\n").is_err());
        assert!(parse_displays("ID,FileDataID\n1727,bad\n").is_err());
    }
}
