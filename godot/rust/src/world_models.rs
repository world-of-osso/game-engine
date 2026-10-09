//! Authored player and creature visual children for replicated units.
use std::{
    collections::HashMap,
    f32::consts::FRAC_PI_2,
    path::PathBuf,
    sync::{Arc, Mutex, OnceLock},
};

use game_engine_core::{
    asset_loader::{AssetLoader, Priority},
    creature_display_data::{CreatureDisplay, query_display},
};
use godot::{
    classes::{MeshInstance3D, Node3D, ShaderMaterial},
    prelude::*,
};
use rusqlite::{Connection, OpenFlags};
use shared::components::{EquipmentAppearance, EquipmentVisualSlot, Player, SheathState};

use crate::{
    animation::WowAnimationPlayer,
    assets::{
        appearance::{NpcAppearances, PreparedNpc},
        creature::{
            CreatureGear, CreatureModelParts, build_creature_model, insert_decoded_textures,
            local_resolver, prepare_creature_model,
        },
        equipment::place_equipment,
        player::{PlayerParts, build_player_model, prepare_player_parts},
    },
    equipment_appearance_data::{
        EquipmentSlot, load_baked_equipment_appearance, resolve_equipment_appearance,
        visual_slot_to_runtime_slots,
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
        // M2 batches and WMO groups each have one surface and one ShaderMaterial.
        let mut material = mesh
            .get_active_material(0)
            .expect("M2 or WMO batch has an authored material")
            .cast::<ShaderMaterial>();
        match light {
            Some(light) => light.bind_model(&mut material),
            None => TerrainLight::clear_model(&mut material),
        }
    }
}

/// Union of the model's batch mesh bounds, in model space.
/// Model-space bounds of a model's batch meshes, for the NPC animation LOD frustum test.
pub(crate) fn mesh_bounds(model: &Gd<Node3D>) -> Aabb {
    model
        .get_children()
        .iter_shared()
        .filter_map(|child| child.try_cast::<MeshInstance3D>().ok())
        .map(|mesh| mesh.get_aabb())
        .reduce(|bounds, next| bounds.merge(next))
        .unwrap_or_default()
}

#[derive(PartialEq, Clone)]
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

    /// A player's body model: its race and sex.
    pub fn player_model(&self) -> Option<(u8, u8)> {
        match self {
            Self::Player(player, _) => Some((player.race, player.appearance.sex)),
            Self::Creature { .. } => None,
        }
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

/// Two workers: one unit's cold extraction does not hold up the next.
const WORKERS: usize = 2;

/// What a worker loads for one unit visual.
enum VisualRequest {
    Creature {
        display_id: u32,
        items: EquipmentAppearance,
        sheath: SheathState,
    },
    Player(Player, EquipmentAppearance),
}

/// A unit visual with its file work done: `WorldModels::build_visual` makes its nodes.
pub(crate) enum VisualParts {
    Creature {
        display_id: u32,
        display: CreatureDisplay,
        npc: Option<PreparedNpc>,
        gear: CreatureGear,
        model: CreatureModelParts,
    },
    Player(PlayerParts),
}

impl VisualParts {
    /// The particle emitters of a creature's display model (the model `build_visual`
    /// names `NpcModel`); `None` for players and emitterless models.
    pub fn particles(&self) -> Option<std::rc::Rc<crate::particles::ModelParticles>> {
        match self {
            Self::Creature { display, model, .. } => {
                crate::particles::ModelParticles::from_model(display.model_fdid, &model.model.model)
            }
            Self::Player(_) => None,
        }
    }

    /// The particle emitters of the item models the visual attaches (a creature's virtual
    /// items and armor models, a player's equipment), by the slot their node is named for.
    pub fn item_particles(
        &self,
        data_root: &std::path::Path,
    ) -> Vec<(EquipmentSlot, std::rc::Rc<crate::particles::ModelParticles>)> {
        let models: Vec<_> = match self {
            Self::Creature { gear, .. } => gear
                .items
                .iter()
                .map(|(model, _)| model)
                .chain(&gear.armor_models)
                .collect(),
            Self::Player(parts) => parts.runtime_models().iter().collect(),
        };
        models
            .into_iter()
            .filter_map(|model| {
                let cached = crate::assets::creature::cached_model(data_root, model.fdid)?;
                let particles =
                    crate::particles::ModelParticles::from_model(model.fdid, &cached.model)?;
                Some((model.slot, particles))
            })
            .collect()
    }
}

/// The catalogs unit visuals read, shared by the main thread and the workers; each
/// loads once, on a worker at startup, so no frame waits for it.
struct VisualCatalogs {
    data_root: PathBuf,
    /// `cache/creature_display.sqlite`, opened on first use.
    displays: Mutex<Option<Connection>>,
    appearances: Mutex<NpcAppearances>,
    /// Creature pose and gear rows; an error stays.
    gear: OnceLock<Result<NpcGearData, String>>,
    outfit: OutfitData,
}

impl VisualCatalogs {
    fn gear(&self) -> Result<&NpcGearData, String> {
        self.gear
            .get_or_init(|| {
                let dir = self.data_root.join("db2/12.1.0.69933");
                NpcGearData::load(&dir).map_err(|error| format!("NPC pose and gear: {error}"))
            })
            .as_ref()
            .map_err(Clone::clone)
    }

    fn query_display(&self, display_id: u32) -> Result<CreatureDisplay, String> {
        let path = self.data_root.join("cache/creature_display.sqlite");
        let mut displays = self.displays.lock().expect("creature display catalog");
        if displays.is_none() {
            let connection = Connection::open_with_flags(&path, OpenFlags::SQLITE_OPEN_READ_ONLY)
                .map_err(|error| format!("Cannot open {}: {error}", path.display()))?;
            *displays = Some(connection);
        }
        let connection = displays.as_ref().expect("Catalog opened above");
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

    fn load(&self, request: VisualRequest) -> Result<VisualParts, String> {
        match request {
            VisualRequest::Creature {
                display_id,
                items,
                sheath,
            } => self.load_creature(display_id, &items, sheath),
            VisualRequest::Player(player, equipment) => {
                // The main thread places an arrived visual's weapons from these rows.
                self.gear()?;
                Ok(VisualParts::Player(prepare_player_parts(
                    &self.data_root,
                    &player,
                    &equipment,
                )?))
            }
        }
    }

    fn load_creature(
        &self,
        display_id: u32,
        items: &EquipmentAppearance,
        sheath: SheathState,
    ) -> Result<VisualParts, String> {
        let display = self.query_display(display_id)?;
        let armor = self.gear()?.display_armor(display_id)?;
        let npc = self.appearances.lock().expect("NPC appearances").prepare(
            &self.data_root,
            display_id,
            |appearance| {
                if appearance.baked_texture_fdid.is_some() {
                    load_baked_equipment_appearance(
                        &armor,
                        &self.outfit,
                        appearance.race,
                        appearance.sex,
                    )
                } else {
                    resolve_equipment_appearance(
                        &armor,
                        &self.outfit,
                        appearance.race,
                        appearance.sex,
                    )
                }
            },
        )?;
        let (race, sex) = npc.as_ref().map_or((0, 0), |npc| (npc.race, npc.sex));
        let gear = CreatureGear {
            armor_models: npc
                .as_ref()
                .map_or_else(Vec::new, |npc| npc.armor.runtime_models.clone()),
            items: self.virtual_item_models(display_id, items, sheath, race, sex)?,
        };
        let resolver = local_resolver(&self.data_root);
        let model = prepare_creature_model(&resolver, &self.data_root, &display, &gear)?;
        Ok(VisualParts::Creature {
            display_id,
            display,
            npc,
            gear,
            model,
        })
    }

    /// Each virtual item's models with the attachment `sheath` places it on; an item
    /// that does not resolve is reported and left out.
    fn virtual_item_models(
        &self,
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
        let gear = self.gear()?;
        let mut models = Vec::new();
        for entry in &items.entries {
            let attachment = gear.virtual_item_placement(entry, sheath);
            let single = EquipmentAppearance {
                entries: vec![entry.clone()],
            };
            match resolve_equipment_appearance(&single, &self.outfit, race, sex) {
                Ok(resolved) => models.extend(
                    resolved
                        .runtime_models
                        .into_iter()
                        .map(|model| (model, attachment)),
                ),
                Err(error) => {
                    godot_error!(
                        "Creature display {display_id} virtual item {:?}: {error}",
                        entry.item_id
                    );
                }
            }
        }
        Ok(models)
    }
}

/// Unit visuals: workers do the file work of each requested appearance (extraction,
/// parsing, texture composition and decoding) and the main thread makes its nodes.
pub(crate) struct WorldModels {
    catalogs: Arc<VisualCatalogs>,
    player_displays: Option<Result<HashMap<(u8, u8), u32>, String>>,
    loader: AssetLoader<u64, VisualParts>,
    /// Requests the workers have not taken yet, by request ID.
    requests: Arc<Mutex<HashMap<u64, VisualRequest>>>,
    next_request: u64,
}

impl WorldModels {
    pub fn data_root(&self) -> &std::path::Path {
        &self.catalogs.data_root
    }

    pub fn new(data_root: PathBuf) -> Self {
        let catalogs = Arc::new(VisualCatalogs {
            outfit: OutfitData::load(&data_root),
            data_root,
            displays: Mutex::new(None),
            appearances: Mutex::new(NpcAppearances::default()),
            gear: OnceLock::new(),
        });
        let requests: Arc<Mutex<HashMap<u64, VisualRequest>>> = Arc::default();
        let loader = {
            let catalogs = Arc::clone(&catalogs);
            let requests = Arc::clone(&requests);
            AssetLoader::new("unit-visuals", WORKERS, move |id: &u64| {
                let request = requests
                    .lock()
                    .expect("unit visual requests")
                    .remove(id)
                    .expect("each request is loaded once");
                catalogs.load(request)
            })
        };
        let warm = Arc::clone(&catalogs);
        loader.run(
            move || {
                if let Err(error) = warm.gear() {
                    godot_error!("{error}");
                }
                warm.outfit.resolve_outfit(1, 1, 0);
            },
            Priority::Now,
        );
        Self {
            catalogs,
            player_displays: None,
            loader,
            requests,
            next_request: 0,
        }
    }

    /// Native player display from the same build-pinned ChrModel rows as unit voices.
    pub fn player_native_display(&mut self, player: &Player) -> Result<u32, String> {
        let dir = self.catalogs.data_root.join("db2/12.1.0.69933");
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

    /// The build-pinned DB2 pose and gear rows (`NpcGearData`) once the startup worker
    /// has loaded them; `None` until then, so no frame waits for the load.
    pub fn loaded_gear(&self) -> Result<Option<&NpcGearData>, String> {
        let loaded = self.catalogs.gear.get();
        loaded
            .map(|gear| gear.as_ref().map_err(Clone::clone))
            .transpose()
    }

    /// The gear rows for a unit whose visual has arrived: every visual load reads them.
    pub fn visual_gear(&self) -> Result<&NpcGearData, String> {
        self.loaded_gear()?
            .ok_or_else(|| "NPC pose and gear rows not loaded before a unit visual".into())
    }

    #[cfg(test)]
    fn outfit(&self) -> &OutfitData {
        &self.catalogs.outfit
    }

    /// Where each of a creature's virtual item models goes under `sheath`.
    pub fn virtual_item_placements(
        &mut self,
        items: &EquipmentAppearance,
        sheath: SheathState,
    ) -> Result<Vec<(EquipmentSlot, Option<u32>)>, String> {
        Ok(virtual_item_placements(self.visual_gear()?, items, sheath))
    }

    /// Where a player's weapons sit under `sheath` (see [`player_weapon_placements`]).
    pub fn player_weapon_placements(
        &mut self,
        equipment: &EquipmentAppearance,
        sheath: SheathState,
    ) -> Result<Vec<(EquipmentSlot, Option<u32>)>, String> {
        Ok(player_weapon_placements(
            self.visual_gear()?,
            equipment,
            sheath,
        ))
    }

    /// Start loading the visual of `appearance` (its virtual items placed for `sheath`);
    /// `poll` hands it out under the returned request ID. Players load ahead of
    /// creatures: the loading screen waits for the local player's model.
    pub fn request(&mut self, appearance: &UnitAppearance, sheath: SheathState) -> u64 {
        let (request, priority) = match appearance {
            UnitAppearance::Creature { display_id, items } => (
                VisualRequest::Creature {
                    display_id: *display_id,
                    items: items.clone(),
                    sheath,
                },
                Priority::Later,
            ),
            UnitAppearance::Player(player, equipment) => (
                VisualRequest::Player(player.clone(), equipment.clone()),
                Priority::Now,
            ),
        };
        let id = self.next_request;
        self.next_request += 1;
        self.requests
            .lock()
            .expect("unit visual requests")
            .insert(id, request);
        self.loader.request(id, priority);
        id
    }

    /// Requested visuals whose file work finished since the last call.
    pub fn poll(&mut self) -> Vec<(u64, Result<VisualParts, String>)> {
        self.loader.poll()
    }

    /// Main thread: the nodes of `parts`; a player visual replacing `previous_player`
    /// continues its playback.
    pub fn build_visual(
        &self,
        parts: VisualParts,
        previous_player: Option<&Gd<Node3D>>,
    ) -> Result<Gd<Node3D>, String> {
        match parts {
            VisualParts::Creature {
                display_id,
                display,
                npc,
                gear,
                model,
            } => self.build_creature_visual(display_id, &display, npc, &gear, model),
            VisualParts::Player(parts) => self.build_player_visual(parts, previous_player),
        }
    }

    /// Main thread: keep the textures a discarded visual's worker decoded, as no other
    /// worker decodes them again.
    pub fn discard(&self, parts: VisualParts) {
        let textures = match parts {
            VisualParts::Creature { model, .. } => model.textures,
            VisualParts::Player(parts) => parts.into_textures(),
        };
        if let Err(error) = insert_decoded_textures(textures) {
            godot_error!("{error}");
        }
    }

    fn build_player_visual(
        &self,
        parts: PlayerParts,
        previous_player: Option<&Gd<Node3D>>,
    ) -> Result<Gd<Node3D>, String> {
        let catalogs = &self.catalogs;
        let mut model = build_player_model(&catalogs.data_root, parts)?;
        model.set_name("PlayerModel");
        // Its weapons are placed for the sheath state by the sync that follows the attach.
        if let Some(previous) = previous_player
            && let Err(error) = transfer_player_playback(previous, &model)
        {
            model.free();
            return Err(error);
        }
        Ok(model)
    }

    fn build_creature_visual(
        &self,
        display_id: u32,
        display: &CreatureDisplay,
        npc: Option<PreparedNpc>,
        gear: &CreatureGear,
        parts: CreatureModelParts,
    ) -> Result<Gd<Node3D>, String> {
        let catalogs = &self.catalogs;
        insert_decoded_textures(parts.textures)?;
        let appearance = npc.map(|npc| npc.appearance.into_prepared()).transpose()?;
        let (mut model, missing) = build_creature_model(
            &catalogs.data_root,
            display,
            &parts.model,
            appearance.as_ref(),
            gear,
        )?;
        if !missing.is_empty() {
            godot_warn!("Creature display {display_id} missing texture FDIDs: {missing:?}");
        }
        model.set_name("NpcModel");
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
}

/// Where each of a creature's virtual item models goes under `sheath`.
pub(crate) fn virtual_item_placements(
    gear: &NpcGearData,
    items: &EquipmentAppearance,
    sheath: SheathState,
) -> Vec<(EquipmentSlot, Option<u32>)> {
    items
        .entries
        .iter()
        .flat_map(|entry| {
            let attachment = gear.virtual_item_placement(entry, sheath);
            visual_slot_to_runtime_slots(entry.slot)
                .into_iter()
                .map(move |slot| (slot, attachment))
        })
        .collect()
}

/// Where a player's weapons sit under `sheath`: drawn in the hands or at their
/// `Item.SheatheType` place, as a creature's virtual items. Armor stays where it is.
pub(crate) fn player_weapon_placements(
    gear: &NpcGearData,
    equipment: &EquipmentAppearance,
    sheath: SheathState,
) -> Vec<(EquipmentSlot, Option<u32>)> {
    let weapons = EquipmentAppearance {
        entries: equipment
            .entries
            .iter()
            .filter(|entry| {
                matches!(
                    entry.slot,
                    EquipmentVisualSlot::MainHand
                        | EquipmentVisualSlot::OffHand
                        | EquipmentVisualSlot::Ranged
                )
            })
            .cloned()
            .collect(),
    };
    virtual_item_placements(gear, &weapons, sheath)
}

/// Move a creature visual's virtual item models to their `placements`.
pub(crate) fn place_virtual_items(
    visual: &Gd<Node3D>,
    placements: &[(EquipmentSlot, Option<u32>)],
) -> Result<(), String> {
    let model = visual
        .try_get_node_as::<Node3D>("NpcModel")
        .ok_or("Creature visual has no NpcModel")?;
    place_items(&model, placements)
}

/// Move a model's item models to their `placements`.
pub(crate) fn place_items(
    model: &Gd<Node3D>,
    placements: &[(EquipmentSlot, Option<u32>)],
) -> Result<(), String> {
    for &(slot, attachment) in placements {
        place_equipment(model, slot, attachment)?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The main thread reads the gear rows without waiting for their startup load; the
    /// first replicated unit at world entry used to wait for the whole 28 MB parse.
    #[test]
    fn gear_rows_are_read_without_waiting_for_their_load() {
        let data_root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../data");
        let models = WorldModels::new(data_root);
        let started = std::time::Instant::now();
        let first = models.loaded_gear().map(|gear| gear.is_some());
        let waited = started.elapsed();
        assert!(
            waited < std::time::Duration::from_millis(50),
            "waited {waited:?}"
        );
        assert_eq!(first, Ok(false), "the startup load is still parsing");
        models.catalogs.gear().expect("gear rows");
        assert!(models.loaded_gear().expect("gear rows").is_some());
    }

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
        let models = WorldModels::new(data_root.clone());
        let armor = models.catalogs.gear().unwrap().display_armor(2989).unwrap();
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

    /// The mage starter set (showcase gear.json): Bent Staff 35 (InventoryType 17,
    /// SheatheType 2) sits on the back (30) while sheathed and in the right hand (1)
    /// when drawn; the robe and the rest of the armor are not moved.
    #[test]
    fn a_player_staff_is_sheathed_on_the_back() {
        use shared::components::{EquipmentVisualSlot, EquippedAppearanceEntry};
        let data_root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../data");
        let mut models = WorldModels::new(data_root.clone());
        // Placements read the gear rows; wait for the startup load, as a visual load does.
        models.catalogs.gear().expect("gear rows");
        let item = |slot, item_id, inventory_type| EquippedAppearanceEntry {
            slot,
            item_id: Some(item_id),
            display_info_id: None,
            inventory_type,
            hidden: false,
            definition_source: Some(shared::item_data::ItemDefinitionSource::Retail),
        };
        let mage = EquipmentAppearance {
            entries: vec![
                item(EquipmentVisualSlot::MainHand, 35, 17),
                item(EquipmentVisualSlot::Feet, 55, 8),
                item(EquipmentVisualSlot::Chest, 56, 20),
                item(EquipmentVisualSlot::Legs, 1395, 7),
                item(EquipmentVisualSlot::Shirt, 6096, 4),
            ],
        };
        let mut placements = |sheath| models.player_weapon_placements(&mage, sheath).unwrap();
        assert_eq!(
            placements(SheathState::Unarmed),
            [(EquipmentSlot::MainHand, Some(30))]
        );
        assert_eq!(
            placements(SheathState::Melee),
            [(EquipmentSlot::MainHand, Some(1))]
        );
    }

    /// Stockade Guard 46405's sword and shield leave the hands for the hip and back when
    /// sheathed; the rifleman's ranged copy of its rifle is not shown.
    #[test]
    fn virtual_item_placements_follow_the_sheath_state() {
        use shared::components::{EquipmentVisualSlot, EquippedAppearanceEntry};
        let data_root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../data");
        let mut models = WorldModels::new(data_root.clone());
        // Placements read the gear rows; wait for the startup load, as a visual load does.
        models.catalogs.gear().expect("gear rows");
        let item = |slot, item_id, inventory_type| EquippedAppearanceEntry {
            slot,
            item_id: Some(item_id),
            display_info_id: None,
            inventory_type,
            hidden: false,
            definition_source: Some(shared::item_data::ItemDefinitionSource::Retail),
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
