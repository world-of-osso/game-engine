//! Authored player and creature visual children for replicated units.
use std::{collections::HashMap, f32::consts::FRAC_PI_2, path::PathBuf};

use game_engine_core::creature_display_data::{CreatureDisplay, query_display};
use godot::{
    classes::{MeshInstance3D, Node3D, ShaderMaterial, VisibleOnScreenNotifier3D},
    prelude::*,
};
use rusqlite::{Connection, OpenFlags};
use shared::components::{EquipmentAppearance, Player, SheathState};

use crate::{
    animation::WowAnimationPlayer,
    assets::{
        appearance::NpcAppearances,
        creature::{CreatureGear, load_creature_model},
        equipment::place_equipment,
        player::load_player_model,
    },
    equipment_appearance_data::{
        EquipmentSlot, resolve_equipment_appearance, visual_slot_to_runtime_slots,
    },
    lighting::TerrainLight,
    npc_gear_data::NpcGearData,
    outfit_data::OutfitData,
};

pub(crate) fn bind_visual_light(visual: &Gd<Node3D>, light: Option<&TerrainLight>) {
    let meshes = visual
        .find_children_ex("*")
        .type_("MeshInstance3D")
        .owned(false)
        .done();
    for node in meshes.iter_shared() {
        let mesh = node.cast::<MeshInstance3D>();
        // The native M2 loader creates one surface and one ShaderMaterial per batch.
        let mut material = mesh
            .get_surface_override_material(0)
            .expect("M2 batch has an authored material")
            .cast::<ShaderMaterial>();
        match light {
            Some(light) => light.bind_model(&mut material),
            None => TerrainLight::clear_model(&mut material),
        }
    }
}

/// Union of the model's batch mesh bounds, in model space.
fn mesh_bounds(model: &Gd<Node3D>) -> Aabb {
    model
        .get_children()
        .iter_shared()
        .filter_map(|child| child.try_cast::<MeshInstance3D>().ok())
        .map(|mesh| mesh.get_aabb())
        .reduce(|bounds, next| bounds.merge(next))
        .unwrap_or_default()
}

#[derive(PartialEq)]
pub(crate) enum UnitAppearance {
    /// A creature display and its virtual items (`creature_equip_template`).
    Creature {
        display_id: u32,
        items: EquipmentAppearance,
    },
    Player(Player, EquipmentAppearance),
}

impl UnitAppearance {
    pub fn describe_unit(&self, server_id: u64) -> String {
        match self {
            Self::Creature { display_id, .. } => format!("NPC {server_id} display {display_id}"),
            Self::Player(player, _) => format!("Player {server_id} ({})", player.name),
        }
    }

    pub fn same_player_model(&self, other: &Self) -> bool {
        matches!((self, other), (Self::Player(left, _), Self::Player(right, _))
            if left.race == right.race && left.appearance.sex == right.appearance.sex)
    }
}

fn transfer_player_playback(previous: &Gd<Node3D>, replacement: &Gd<Node3D>) -> Result<(), String> {
    let mut old_animation = previous
        .try_get_node_as::<WowAnimationPlayer>("M2Animation")
        .ok_or("Previous player visual has no bone animation")?;
    let mut new_animation = replacement
        .try_get_node_as::<WowAnimationPlayer>("M2Animation")
        .ok_or("Replacement player visual has no bone animation")?;
    new_animation.set_process(old_animation.is_processing());
    new_animation
        .bind_mut()
        .transfer_playback_from(&mut old_animation.bind_mut())
}

pub(crate) struct WorldModels {
    data_root: PathBuf,
    catalog: Option<Connection>,
    player_displays: Option<Result<HashMap<(u8, u8), u32>, String>>,
    appearances: NpcAppearances,
    /// Creature pose and gear rows, loaded with the first creature; an error stays.
    gear: Option<Result<NpcGearData, String>>,
    outfit: Option<OutfitData>,
}

impl WorldModels {
    pub fn new(data_root: PathBuf) -> Self {
        Self {
            data_root,
            catalog: None,
            player_displays: None,
            appearances: NpcAppearances::default(),
            gear: None,
            outfit: None,
        }
    }

    /// Native player display from the same build-pinned ChrModel rows as unit voices.
    pub fn player_native_display(&mut self, player: &Player) -> Result<u32, String> {
        let dir = self.data_root.join("db2/12.1.0.69933");
        let displays = self
            .player_displays
            .get_or_insert_with(|| game_engine_core::spell_visual::player_displays(&dir))
            .as_ref()
            .map_err(Clone::clone)?;
        displays
            .get(&(player.race, player.appearance.sex))
            .copied()
            .ok_or_else(|| {
                format!(
                    "Missing ChrModel display for race {} sex {}",
                    player.race, player.appearance.sex
                )
            })
    }

    /// The build-pinned DB2 pose and gear rows (`NpcGearData`).
    pub fn gear(&mut self) -> Result<&NpcGearData, String> {
        let data_root = &self.data_root;
        self.gear
            .get_or_insert_with(|| {
                let dir = data_root.join("db2/12.1.0.69933");
                NpcGearData::load(&dir).map_err(|error| format!("NPC pose and gear: {error}"))
            })
            .as_ref()
            .map_err(Clone::clone)
    }

    fn outfit(&mut self) -> &OutfitData {
        let data_root = &self.data_root;
        self.outfit
            .get_or_insert_with(|| OutfitData::load(data_root))
    }

    /// Where each of a creature's virtual item models goes under `sheath`.
    pub fn virtual_item_placements(
        &mut self,
        items: &EquipmentAppearance,
        sheath: SheathState,
    ) -> Result<Vec<(EquipmentSlot, Option<u32>)>, String> {
        let gear = self.gear()?;
        Ok(items
            .entries
            .iter()
            .flat_map(|entry| {
                let attachment = gear.virtual_item_placement(entry, sheath);
                visual_slot_to_runtime_slots(entry.slot)
                    .into_iter()
                    .map(move |slot| (slot, attachment))
            })
            .collect())
    }

    fn query_display(&mut self, display_id: u32) -> Result<CreatureDisplay, String> {
        let path = self.data_root.join("cache/creature_display.sqlite");
        if self.catalog.is_none() {
            let connection = Connection::open_with_flags(&path, OpenFlags::SQLITE_OPEN_READ_ONLY)
                .map_err(|error| format!("Cannot open {}: {error}", path.display()))?;
            self.catalog = Some(connection);
        }
        let connection = self.catalog.as_ref().expect("Catalog opened above");
        query_display(connection, display_id)
            .map_err(|error| {
                format!(
                    "Cannot query display {display_id} in {}: {error}",
                    path.display()
                )
            })?
            .ok_or_else(|| {
                format!(
                    "Creature display {display_id} absent from {}",
                    path.display()
                )
            })
    }

    pub fn load_visual(
        &mut self,
        appearance: &UnitAppearance,
        sheath: SheathState,
        previous_player: Option<&Gd<Node3D>>,
    ) -> Result<Gd<Node3D>, String> {
        match appearance {
            UnitAppearance::Creature { display_id, items } => {
                self.load_creature_visual(*display_id, items, sheath)
            }
            UnitAppearance::Player(player, equipment) => {
                self.load_player_visual(player, equipment, previous_player)
            }
        }
    }

    fn load_player_visual(
        &self,
        player: &Player,
        equipment: &EquipmentAppearance,
        previous_player: Option<&Gd<Node3D>>,
    ) -> Result<Gd<Node3D>, String> {
        let mut model = load_player_model(&self.data_root, player, equipment)?;
        model.set_name("PlayerModel");
        if let Some(previous) = previous_player {
            if let Err(error) = transfer_player_playback(previous, &model) {
                model.free();
                return Err(error);
            }
        }
        Ok(model)
    }

    fn load_creature_visual(
        &mut self,
        display_id: u32,
        items: &EquipmentAppearance,
        sheath: SheathState,
    ) -> Result<Gd<Node3D>, String> {
        let display = self.query_display(display_id)?;
        let armor = self.gear()?.display_armor(display_id)?;
        let outfit = self
            .outfit
            .get_or_insert_with(|| OutfitData::load(&self.data_root));
        let prepared = self
            .appearances
            .prepare(&self.data_root, display_id, |race, sex| {
                resolve_equipment_appearance(&armor, outfit, race, sex)
            })?;
        let (race, sex) = prepared.as_ref().map_or((0, 0), |npc| (npc.race, npc.sex));
        let gear = CreatureGear {
            armor_models: prepared
                .as_ref()
                .map_or_else(Vec::new, |npc| npc.armor.runtime_models.clone()),
            items: self.virtual_item_models(display_id, items, sheath, race, sex)?,
        };
        let (mut model, missing) = load_creature_model(
            &self.data_root,
            &display,
            prepared.as_ref().map(|npc| &npc.appearance),
            &gear,
        )?;
        if !missing.is_empty() {
            godot_warn!("Creature display {display_id} missing texture FDIDs: {missing:?}");
        }
        model.set_name("NpcModel");
        // Last frame's on-screen state for the NPC animation LOD.
        let mut on_screen = VisibleOnScreenNotifier3D::new_alloc();
        on_screen.set_name("OnScreen");
        on_screen.set_aabb(mesh_bounds(&model));
        model.add_child(&on_screen);
        let scale = if display.scale_milli == 0 {
            1.0
        } else {
            display.scale_milli as f32 / 1000.0
        };
        let mut visual = Node3D::new_alloc();
        visual.set_name("NpcVisualRoot");
        visual.set_scale(Vector3::ONE * scale.max(0.01));
        visual.set_rotation(Vector3::new(0.0, -FRAC_PI_2, 0.0));
        visual.add_child(&model);
        Ok(visual)
    }

    /// Each virtual item's models with the attachment `sheath` places it on; an item
    /// that does not resolve is reported and left out.
    fn virtual_item_models(
        &mut self,
        display_id: u32,
        items: &EquipmentAppearance,
        sheath: SheathState,
        race: u8,
        sex: u8,
    ) -> Result<
        Vec<(
            crate::equipment_appearance_data::RuntimeModelAppearance,
            Option<u32>,
        )>,
        String,
    > {
        let placements: Vec<_> = {
            let gear = self.gear()?;
            items
                .entries
                .iter()
                .map(|entry| (entry.clone(), gear.virtual_item_placement(entry, sheath)))
                .collect()
        };
        let outfit = self.outfit();
        let mut models = Vec::new();
        for (entry, attachment) in placements {
            let single = EquipmentAppearance {
                entries: vec![entry.clone()],
            };
            match resolve_equipment_appearance(&single, outfit, race, sex) {
                Ok(resolved) => models.extend(
                    resolved
                        .runtime_models
                        .into_iter()
                        .map(|model| (model, attachment)),
                ),
                Err(error) => godot_error!(
                    "Creature display {display_id} virtual item {:?}: {error}",
                    entry.item_id
                ),
            }
        }
        Ok(models)
    }
}

/// Move a creature visual's virtual item models to their `placements`.
pub(crate) fn place_virtual_items(
    visual: &Gd<Node3D>,
    placements: &[(EquipmentSlot, Option<u32>)],
) -> Result<(), String> {
    let model = visual
        .try_get_node_as::<Node3D>("NpcModel")
        .ok_or("Creature visual has no NpcModel")?;
    for &(slot, attachment) in placements {
        place_equipment(&model, slot, attachment)?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn player_model_display_native_identity_uses_authored_chrmodel_rows() {
        let data_root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../data");
        let mut models = WorldModels::new(data_root.clone());
        let mut player = Player {
            name: "Alice".into(),
            race: 1,
            class: 11,
            appearance: Default::default(),
        };
        assert_eq!(models.player_native_display(&player).unwrap(), 57899);
        player.appearance.sex = 1;
        assert_eq!(models.player_native_display(&player).unwrap(), 56658);
        player.race = 0;
        assert!(
            models
                .player_native_display(&player)
                .unwrap_err()
                .contains("race 0 sex 1")
        );
    }

    /// Stockade Guard display 2989 → CreatureDisplayInfoExtra 1274: its gloves, boots and
    /// tabard switch body geoset groups 4, 5/20 and 12 to their item variants; the shirt,
    /// belt and pants textures are in the display's bake, not item models.
    #[test]
    fn stockade_guard_display_armor_resolves_to_body_geosets() {
        let data_root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../data");
        let mut models = WorldModels::new(data_root.clone());
        let armor = models.gear().unwrap().display_armor(2989).unwrap();
        let resolved = resolve_equipment_appearance(&armor, models.outfit(), 1, 0).unwrap();
        for geoset in [(4, 2), (5, 2), (20, 2), (12, 2)] {
            assert!(
                resolved.outfit.geoset_overrides.contains(&geoset),
                "{geoset:?} in {:?}",
                resolved.outfit.geoset_overrides
            );
        }
        assert!(resolved.hidden_character_geoset_groups.is_empty());
    }

    /// Stockade Guard 46405's sword and shield leave the hands for the hip and back when
    /// sheathed; the rifleman's ranged copy of its rifle is not shown.
    #[test]
    fn virtual_item_placements_follow_the_sheath_state() {
        use shared::components::{EquipmentVisualSlot, EquippedAppearanceEntry};
        let data_root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../data");
        let mut models = WorldModels::new(data_root.clone());
        let item = |slot, item_id, inventory_type| EquippedAppearanceEntry {
            slot,
            item_id: Some(item_id),
            display_info_id: None,
            inventory_type,
            hidden: false,
        };
        let guard = EquipmentAppearance {
            entries: vec![
                item(EquipmentVisualSlot::MainHand, 5305, 13),
                item(EquipmentVisualSlot::OffHand, 1984, 14),
                item(EquipmentVisualSlot::Ranged, 12523, 26),
            ],
        };
        let placements = |models: &mut WorldModels, sheath| {
            models.virtual_item_placements(&guard, sheath).unwrap()
        };
        use EquipmentSlot::{MainHand, OffHand, Ranged};
        assert_eq!(
            placements(&mut models, SheathState::Melee),
            [(MainHand, Some(1)), (OffHand, Some(0)), (Ranged, None)]
        );
        assert_eq!(
            placements(&mut models, SheathState::Unarmed),
            [(MainHand, Some(32)), (OffHand, Some(28)), (Ranged, None)]
        );
        assert_eq!(
            placements(&mut models, SheathState::Ranged),
            [(MainHand, Some(32)), (OffHand, Some(28)), (Ranged, Some(1))]
        );
    }
}
