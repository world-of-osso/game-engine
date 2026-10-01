//! Real replicated mailbox M2s, resolved through build-pinned GameObjectDisplayInfo.
use crate::{
    assets::{
        build_model,
        creature::{cache_model_files, cache_model_textures, local_resolver},
        read_model,
    },
    lighting::TerrainLight,
    world_models::bind_visual_light,
};
use game_engine_core::csv_util::parse_csv_line;
use game_engine_network::GameObjectSnapshot;
use godot::{classes::Node3D, prelude::*};
use shared::protocol::{GAMEOBJECT_TYPE_MAILBOX, GameObjectInfo};
use std::{collections::HashMap, path::PathBuf};

struct ObjectNode {
    snapshot: GameObjectSnapshot,
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

impl GameObjects {
    pub fn new(data_root: PathBuf) -> Self {
        Self {
            data_root,
            displays: None,
            objects: HashMap::new(),
            light: None,
        }
    }
    pub fn info(&self, id: u64) -> Option<&GameObjectInfo> {
        Some(&self.objects.get(&id)?.snapshot.info)
    }
    pub fn position(&self, id: u64) -> Option<Vector3> {
        Some(self.objects.get(&id)?.node.get_global_position())
    }
    pub fn upsert(
        &mut self,
        parent: &mut Gd<Node3D>,
        snapshot: GameObjectSnapshot,
    ) -> Result<(), String> {
        let id = snapshot.server_id;
        if snapshot.info.go_type != GAMEOBJECT_TYPE_MAILBOX {
            self.remove(id);
            return Ok(());
        }
        let Some(position) = snapshot.position else {
            return Ok(());
        };
        if !snapshot.info.scale.is_finite() || snapshot.info.scale <= 0.0 {
            return Err(format!(
                "Mailbox {id} has invalid replicated scale {}",
                snapshot.info.scale
            ));
        }
        let changed = self
            .objects
            .get(&id)
            .is_none_or(|old| old.snapshot.info.display_id != snapshot.info.display_id);
        if changed {
            let visual = self.load_visual(&snapshot.info)?;
            if let Err(error) = crate::targeting::attach_pick_area(&visual, id) {
                visual.free();
                return Err(error);
            }
            self.remove(id);
            let mut node = Node3D::new_alloc();
            node.set_name(&format!("Mailbox_{id}"));
            node.set_meta("game_object_server_id", &(id as i64).to_variant());
            node.set_meta("game_object_name", &snapshot.info.name.to_variant());
            node.set_meta(
                "game_object_entry",
                &(snapshot.info.entry as i64).to_variant(),
            );
            node.set_meta(
                "game_object_display_id",
                &(snapshot.info.display_id as i64).to_variant(),
            );
            node.add_child(&visual);
            parent.add_child(&node);
            self.objects.insert(
                id,
                ObjectNode {
                    snapshot: snapshot.clone(),
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
            snapshot.rotation.map_or(0.0, |r| r.y),
            0.0,
        ));
        object.node.set_scale(Vector3::ONE * snapshot.info.scale);
        object.snapshot = snapshot;
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
                    "Mailbox entry {} has no GameObjectDisplayInfo {} model FileDataID",
                    info.entry, info.display_id
                )
            })?;
        let resolver = local_resolver(&self.data_root);
        let path = cache_model_files(&resolver, &self.data_root, fdid).map_err(|e| {
            format!(
                "Mailbox entry {} display {} model {fdid}: {e}",
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
                "Mailbox entry {} model {fdid} missing texture FDIDs: {missing:?}",
                info.entry
            ));
        }
        model.set_name("GameObjectModel");
        model.set_meta("model_file_data_id", &(fdid as i64).to_variant());
        model.set_rotation(Vector3::new(0.0, -std::f32::consts::FRAC_PI_2, 0.0));
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
