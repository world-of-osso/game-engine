//! One isolated native model scene per visible Wardrobe card; no replicated gear edits.
//! Item and illusion cards show weapons standalone; set cards dress an undressed player
//! (TransmogSetBaseModelMixin SetAutoDress(false) + TryOn).
use super::*;
use game_engine_ui_collections::transmog::ItemEntry;
use game_engine_ui_collections::transmog_frame::{
    custom_set_model_scene, illusion_model_scene, set_model_scene, source_model_scene,
};
use godot::classes::Node3D;
use std::collections::BTreeMap;

pub(super) struct CollectionPreview {
    scene: Scene,
    appearance: UnitAppearance,
    /// Weapon slot shown as a standalone item model; None dresses the player body.
    weapon: Option<EquipmentSlot>,
    pending: Option<u64>,
}

type Desired = BTreeMap<String, (UnitAppearance, Option<EquipmentSlot>)>;

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
        for (scene, (appearance, weapon)) in desired {
            if !self.transmog.collection.contains_key(&scene) {
                self.request_transmog_collection_model(scene, appearance, weapon)?;
            }
        }
        self.attach_transmog_collection_models()
    }

    fn transmog_collection_appearances(&self) -> Result<Desired, String> {
        let Some(UnitAppearance::Player(player, _, tabard)) = self.world.local_player_appearance()
        else {
            return Ok(BTreeMap::new());
        };
        let book = &self.transmog.book;
        let dress = |equipment| UnitAppearance::Player(player.clone(), equipment, *tabard);
        let mut desired = Desired::new();
        match book.tab {
            WardrobeTab::Items => {
                let Some(slot) = book.selected else {
                    return Ok(desired);
                };
                let weapon = is_weapon(slot).then_some(slot);
                for entry in book.page_items() {
                    let (scene, equipment) = match entry {
                        ItemEntry::Hidden => continue,
                        ItemEntry::Source(source) => (
                            source_model_scene(source.source_id),
                            book.collection_equipment(source.source_id),
                        ),
                        ItemEntry::Illusion(illusion) => (
                            illusion_model_scene(illusion.id),
                            book.weapon_equipment(slot),
                        ),
                    };
                    let equipment = equipment
                        .ok_or_else(|| format!("Wardrobe card {scene} has no visual slot"))?;
                    desired.insert(scene, (dress(equipment), weapon));
                }
            }
            WardrobeTab::Sets => {
                for (set, _) in book.page_sets() {
                    if let Some(equipment) = book.set_equipment(set.set_id) {
                        desired.insert(set_model_scene(set.set_id), (dress(equipment), None));
                    }
                }
            }
            WardrobeTab::CustomSets => {
                for set in book.page_custom_sets() {
                    if let Some(equipment) = book.custom_set_equipment(set.id) {
                        desired.insert(custom_set_model_scene(set.id), (dress(equipment), None));
                    }
                }
            }
            WardrobeTab::Situations => {}
        }
        Ok(desired)
    }

    fn remove_old_transmog_collection(&mut self, desired: &Desired) {
        let removed: Vec<_> = self
            .transmog
            .collection
            .iter()
            .filter(|(scene, preview)| {
                desired.get(*scene) != Some(&(preview.appearance.clone(), preview.weapon))
            })
            .map(|(scene, _)| scene.clone())
            .collect();
        for scene in removed {
            let preview = self
                .transmog
                .collection
                .remove(&scene)
                .expect("visible collection card");
            if let Some(request) = preview.pending {
                self.world.cancel_detached_visual(request);
            }
            preview.scene.free();
        }
    }

    fn request_transmog_collection_model(
        &mut self,
        scene_name: String,
        appearance: UnitAppearance,
        weapon: Option<EquipmentSlot>,
    ) -> Result<(), String> {
        let host = self
            .transmog
            .ui
            .as_ref()
            .ok_or("Wardrobe UI missing")?
            .bind()
            .frame_control(&scene_name)
            .ok_or_else(|| format!("Wardrobe {scene_name} control missing"))?;
        let scene = Scene::new(host);
        let request = self.world.request_detached_visual(&appearance);
        self.transmog.collection.insert(
            scene_name,
            CollectionPreview {
                scene,
                appearance,
                weapon,
                pending: Some(request),
            },
        );
        Ok(())
    }

    fn attach_transmog_collection_models(&mut self) -> Result<(), String> {
        for (scene_name, preview) in &mut self.transmog.collection {
            preview.scene.fit_viewport();
            let Some(request) = preview.pending else {
                continue;
            };
            let Some(loaded) = self.world.take_detached_visual(request) else {
                continue;
            };
            preview.pending = None;
            let player = loaded.map_err(|error| format!("Wardrobe {scene_name}: {error}"))?;
            match preview.weapon {
                Some(slot) => {
                    let model = weapon_model(player, slot)?;
                    bind_sheet_light(&model);
                    preview.scene.replace_item_model(model);
                }
                None => {
                    // TryOn of a set's weapon pieces shows them in hand, as in the preview.
                    if let Err(error) = self.world.place_player_weapons(
                        &player,
                        &preview.appearance,
                        transmog_sheath(false),
                    ) {
                        player.free();
                        return Err(format!("Wardrobe {scene_name}: {error}"));
                    }
                    bind_sheet_light(&player);
                    preview.scene.replace_model(player);
                }
            }
        }
        Ok(())
    }
}

fn is_weapon(slot: EquipmentSlot) -> bool {
    matches!(
        slot,
        EquipmentSlot::MainHand | EquipmentSlot::OffHand | EquipmentSlot::Ranged
    )
}

/// Retail SetItemAppearance shows a standalone weapon.
fn weapon_model(player: Gd<Node3D>, slot: EquipmentSlot) -> Result<Gd<Node3D>, String> {
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
