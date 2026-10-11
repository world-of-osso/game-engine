//! One isolated native model scene per visible Wardrobe cell; no replicated gear edits.
use super::*;
use game_engine_ui_model::transmog_frame::source_model_scene;
use godot::classes::Node3D;
use std::collections::BTreeMap;

pub(super) struct CollectionPreview {
    scene: Scene,
    appearance: UnitAppearance,
    slot: EquipmentSlot,
    pending: Option<u64>,
}

impl GameClient {
    pub(super) fn clear_transmog_collection(&mut self) {
        for (_, preview) in std::mem::take(&mut self.transmog.collection) {
            if let Some(request) = preview.pending {
                self.world.cancel_detached_visual(request);
            }
            preview.scene.free();
        }
    }

    pub(super) fn sync_transmog_collection(&mut self) -> Result<(), String> {
        let desired = self.transmog_collection_appearances()?;
        self.remove_old_transmog_collection(&desired);
        for (source_id, appearance) in desired {
            if !self.transmog.collection.contains_key(&source_id) {
                self.request_transmog_collection_model(source_id, appearance)?;
            }
        }
        self.attach_transmog_collection_models()
    }

    fn transmog_collection_appearances(&self) -> Result<BTreeMap<u32, UnitAppearance>, String> {
        let Some(UnitAppearance::Player(player, _)) = self.world.local_player_appearance() else {
            return Ok(BTreeMap::new());
        };
        let book = &self.transmog.book;
        book.page_sources()
            .iter()
            .map(|source| {
                let equipment = book.collection_equipment(source.source_id).ok_or_else(|| {
                    format!(
                        "Wardrobe source{} has no selected visual slot",
                        source.source_id
                    )
                })?;
                Ok((
                    source.source_id,
                    UnitAppearance::Player(player.clone(), equipment),
                ))
            })
            .collect()
    }

    fn remove_old_transmog_collection(&mut self, desired: &BTreeMap<u32, UnitAppearance>) {
        let removed: Vec<_> = self
            .transmog
            .collection
            .iter()
            .filter(|(id, preview)| desired.get(id) != Some(&preview.appearance))
            .map(|(id, _)| *id)
            .collect();
        for id in removed {
            let preview = self
                .transmog
                .collection
                .remove(&id)
                .expect("visible collection cell");
            if let Some(request) = preview.pending {
                self.world.cancel_detached_visual(request);
            }
            preview.scene.free();
        }
    }

    fn request_transmog_collection_model(
        &mut self,
        source_id: u32,
        appearance: UnitAppearance,
    ) -> Result<(), String> {
        let slot = self
            .transmog
            .book
            .selected
            .ok_or("Wardrobe collection has no selected slot")?;
        let host = self
            .transmog
            .ui
            .as_ref()
            .ok_or("Wardrobe UI missing")?
            .bind()
            .frame_control(&source_model_scene(source_id))
            .ok_or_else(|| format!("Wardrobe source{source_id} model control missing"))?;
        let scene = Scene::new(host);
        let request = self.world.request_detached_visual(&appearance);
        self.transmog.collection.insert(
            source_id,
            CollectionPreview {
                scene,
                appearance,
                slot,
                pending: Some(request),
            },
        );
        Ok(())
    }

    fn attach_transmog_collection_models(&mut self) -> Result<(), String> {
        for (source_id, preview) in &mut self.transmog.collection {
            preview.scene.fit_viewport();
            let Some(request) = preview.pending else {
                continue;
            };
            let Some(loaded) = self.world.take_detached_visual(request) else {
                continue;
            };
            preview.pending = None;
            let player = loaded.map_err(|error| format!("Wardrobe source{source_id}: {error}"))?;
            let model = collection_model(player, preview.slot)?;
            bind_sheet_light(&model);
            preview.scene.replace_model(model);
        }
        Ok(())
    }
}

/// Retail SetItemAppearance shows a standalone weapon; armor dresses the player body.
fn collection_model(player: Gd<Node3D>, slot: EquipmentSlot) -> Result<Gd<Node3D>, String> {
    if !matches!(
        slot,
        EquipmentSlot::MainHand | EquipmentSlot::OffHand | EquipmentSlot::Ranged
    ) {
        return Ok(player);
    }
    let items = player
        .find_children_ex(&format!("Equipment{slot:?}*"))
        .type_("Node3D")
        .owned(false)
        .done();
    if items.is_empty() {
        player.free();
        return Err(format!("Wardrobe {slot:?} appearance has no weapon model"));
    }
    let mut group = Node3D::new_alloc();
    group.set_name("WardrobeItemModel");
    for node in items.iter_shared() {
        let item = node.cast::<Node3D>();
        let mut parent = item.get_parent().expect("equipment attached to player");
        parent.remove_child(&item);
        group.add_child(&item);
    }
    player.free();
    Ok(group)
}
