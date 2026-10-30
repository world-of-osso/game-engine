//! Offline diagnostic host for the spell-visual census.
use std::collections::HashMap;
use std::path::PathBuf;

use game_engine_core::spell_visual::{CasterModels, VisualKit, read_animation_fallbacks};
use game_engine_network::replica::Replica;
use godot::classes::{Node3D, ProjectSettings};
use godot::prelude::*;
use shared::components::{CharacterAppearance, Player};

use crate::animation::{ActionPriority, WowAnimationPlayer};
use crate::spell_effects::SpellEffects;
use crate::world::WorldUnits;
use crate::world_models::UnitAppearance;

mod outfit;
use outfit::{OutfitCatalog, StartingOutfit};

#[derive(GodotClass)]
#[class(base = Node3D)]
struct SpellCensusFixture {
    base: Base<Node3D>,
    data: PathBuf,
    legal: Option<CasterModels>,
    outfits: Option<OutfitCatalog>,
    outfit: Option<StartingOutfit>,
    appearance: Option<UnitAppearance>,
    world: WorldUnits,
    effects: SpellEffects,
    request: Option<u64>,
    caster: Option<Gd<Node3D>>,
    error: Option<String>,
    kit_request: Option<(u32, u32, u32)>,
    prepared: Option<VisualKit>,
    fallbacks: Option<HashMap<u16, u16>>,
    played_animation: Option<u16>,
}

#[godot_api]
impl INode3D for SpellCensusFixture {
    fn init(base: Base<Node3D>) -> Self {
        let path = ProjectSettings::singleton().globalize_path("res://../data");
        let data = PathBuf::from(path.to_string());
        Self {
            base,
            world: WorldUnits::new(data.clone()),
            effects: SpellEffects::new(data.clone()),
            data,
            legal: None,
            outfits: None,
            outfit: None,
            appearance: None,
            request: None,
            caster: None,
            error: None,
            kit_request: None,
            prepared: None,
            fallbacks: None,
            played_animation: None,
        }
    }

    fn exit_tree(&mut self) {
        self.effects.reset();
        self.world.reset();
    }
}

#[godot_api]
impl SpellCensusFixture {
    #[func]
    fn begin_caster(&mut self, race: i64, class: i64, sex: i64) -> GString {
        GString::from(
            self.request_caster(race, class, sex)
                .err()
                .unwrap_or_default()
                .as_str(),
        )
    }

    #[func]
    fn poll_caster(&mut self) -> VarDictionary {
        self.world.attach_loaded_visuals(&Replica::default());
        if let Some(request) = self.request
            && let Some(result) = self.world.take_detached_visual(request)
        {
            self.request = None;
            match result {
                Ok(mut caster) => {
                    let placement = match (&self.appearance, &self.outfit) {
                        (Some(appearance), Some(outfit)) => {
                            self.world
                                .place_player_weapons(&caster, appearance, outfit.sheath)
                        }
                        _ => Ok(()),
                    };
                    if let Err(error) = placement {
                        caster.free();
                        self.error = Some(error);
                    } else {
                        // The detached owner uses the existing NpcModel attachment path.
                        caster.set_name("NpcModel");
                        self.base_mut().add_child(&caster);
                        if let Some(mut animation) =
                            caster.try_get_node_as::<WowAnimationPlayer>("M2Animation")
                        {
                            animation.set_process(false);
                        }
                        self.caster = Some(caster);
                    }
                }
                Err(error) => self.error = Some(error),
            }
        }
        self.state(self.caster.is_some())
    }

    #[func]
    fn outfit_state(&self) -> VarDictionary {
        let mut state = VarDictionary::new();
        state.set("found", self.outfit.is_some());
        state.set("source", "data/CharStartOutfit.csv");
        let item_ids: Vec<i32> = self
            .outfit
            .as_ref()
            .map(|outfit| outfit.item_ids.iter().map(|&id| id as i32).collect())
            .unwrap_or_default();
        state.set("item_ids", &PackedInt32Array::from(item_ids.as_slice()));
        let mut skipped = VarArray::new();
        if let Some(outfit) = &self.outfit {
            for item in &outfit.skipped {
                skipped.push(item.as_str());
            }
        }
        state.set("skipped", &skipped);
        state.set(
            "weapon_subclass",
            self.outfit
                .as_ref()
                .and_then(|outfit| outfit.weapon_subclass)
                .map(i32::from)
                .unwrap_or(-1),
        );
        let weapon = self
            .caster
            .as_ref()
            .and_then(|caster| {
                caster
                    .find_child_ex("EquipmentMainHand")
                    .owned(false)
                    .done()
            })
            .is_some();
        let offhand = self
            .caster
            .as_ref()
            .and_then(|caster| caster.find_child_ex("EquipmentOffHand").owned(false).done())
            .is_some();
        state.set("weapon_model_loaded", weapon);
        state.set("offhand_model_loaded", offhand);
        state
    }

    #[func]
    fn prepare_kit(&mut self, visual: i64, kit: i64, spell: i64) -> GString {
        GString::from(
            self.request_kit(visual, kit, spell)
                .err()
                .unwrap_or_default()
                .as_str(),
        )
    }

    #[func]
    fn poll_kit(&mut self) -> VarDictionary {
        if let Err(error) = self.effects.advance(0.0, None, 0.0, &mut self.world) {
            self.error = Some(error);
        }
        if let Some((visual, kit, _)) = self.kit_request {
            match self.effects.census_prepare(visual, kit) {
                Ok(Some(kit)) => self.prepared = Some(kit),
                Ok(None) => {}
                Err(error) => self.error = Some(error),
            }
        }
        self.state(self.prepared.is_some())
    }

    #[func]
    fn start_kit(&mut self) -> VarDictionary {
        let mut state = VarDictionary::new();
        match self.play_kit() {
            Ok((requested, played)) => {
                state.set("error", "");
                state.set(
                    "requested_animation",
                    requested.map(i32::from).unwrap_or(-1),
                );
                state.set("played_animation", played.map(i32::from).unwrap_or(-1));
            }
            Err(error) => state.set("error", error.as_str()),
        }
        state
    }

    #[func]
    fn tick(&mut self, delta: f64, camera: Transform3D) -> GString {
        GString::from(
            self.advance_capture(delta, camera)
                .err()
                .unwrap_or_default()
                .as_str(),
        )
    }

    #[func]
    fn capture_state(&self) -> VarDictionary {
        let owner = self.base().instance_id();
        let models: Vec<i32> = self
            .effects
            .census_active_models(owner)
            .into_iter()
            .map(|id| id as i32)
            .collect();
        let mut state = VarDictionary::new();
        state.set("active_models", &PackedInt32Array::from(models.as_slice()));
        state.set(
            "particles_drawn",
            self.effects.census_particles_drawn() as i64,
        );
        let action = self
            .caster
            .as_ref()
            .and_then(|caster| caster.try_get_node_as::<WowAnimationPlayer>("M2Animation"))
            .map(|animation| animation.bind().action_id())
            .unwrap_or(-1);
        state.set("action_id", action);
        state.set("effect_clock", f64::from(self.effects.clock()));
        state
    }
}

impl SpellCensusFixture {
    fn advance_capture(&mut self, delta: f64, camera: Transform3D) -> Result<(), String> {
        let mut errors = Vec::new();
        errors.extend(
            self.effects
                .advance(0.0, Some(camera), 0.0, &mut self.world)
                .err(),
        );
        if let Some(caster) = &self.caster
            && let Some(mut animation) = caster.try_get_node_as::<WowAnimationPlayer>("M2Animation")
            && !animation.bind_mut().advance_time_ms(delta * 1000.0)
        {
            errors.push("caster animation advance failed".into());
        }
        let owner = self.base().instance_id();
        errors.extend(self.effects.census_advance_animations(owner, delta).err());
        errors.extend(
            self.effects
                .advance(delta as f32, Some(camera), 0.0, &mut self.world)
                .err(),
        );
        errors.extend(self.effects.census_advance_animations(owner, 0.0).err());
        if errors.is_empty() {
            Ok(())
        } else {
            Err(errors.join("; "))
        }
    }

    fn state(&self, ready: bool) -> VarDictionary {
        let mut state = VarDictionary::new();
        state.set("ready", ready);
        state.set("error", self.error.as_deref().unwrap_or_default());
        state
    }

    fn request_caster(&mut self, race: i64, class: i64, sex: i64) -> Result<(), String> {
        let race = u8::try_from(race).map_err(|error| format!("race: {error}"))?;
        let class = u8::try_from(class).map_err(|error| format!("class: {error}"))?;
        let sex = u8::try_from(sex).map_err(|error| format!("sex: {error}"))?;
        if sex > 1 {
            return Err(format!("unsupported player sex {sex}"));
        }
        if self.legal.is_none() {
            self.legal = Some(CasterModels::load(
                &self.data.join("db2/active"),
                &self.data,
            )?);
        }
        if !self
            .legal
            .as_ref()
            .expect("loaded race data")
            .legal_races(class)
            .contains(&race)
        {
            return Err(format!(
                "race {race} is not legal for class {class} in CharBaseInfo"
            ));
        }
        if self.outfits.is_none() {
            self.outfits = Some(OutfitCatalog::load(&self.data)?);
        }
        self.outfit = self
            .outfits
            .as_ref()
            .expect("loaded outfit rows")
            .resolve(race, class, sex);
        if self.world.root().is_none() {
            let mut root = Node3D::new_alloc();
            root.set_name("CensusWorld");
            self.base_mut().add_child(&root);
            self.world.set_census_root(root);
        }
        if let Some(request) = self.request.take() {
            self.world.cancel_detached_visual(request);
        }
        let owner = self.base().instance_id();
        self.effects.census_clear(owner);
        self.kit_request = None;
        self.prepared = None;
        self.played_animation = None;
        if let Some(mut caster) = self.caster.take() {
            caster.queue_free();
        }
        self.error = None;
        let player = Player {
            name: "Spell Census".into(),
            race,
            class,
            appearance: CharacterAppearance {
                sex,
                ..Default::default()
            },
        };
        let equipment = self
            .outfit
            .as_ref()
            .map(|outfit| outfit.equipment.clone())
            .unwrap_or_default();
        let appearance = UnitAppearance::Player(player, equipment, None);
        self.request = Some(self.world.request_detached_visual(&appearance));
        self.appearance = Some(appearance);
        Ok(())
    }

    fn request_kit(&mut self, visual: i64, kit: i64, spell: i64) -> Result<(), String> {
        if self.caster.is_none() {
            return Err("caster is not loaded".into());
        }
        let visual = u32::try_from(visual).map_err(|error| format!("visual: {error}"))?;
        let kit = u32::try_from(kit).map_err(|error| format!("kit: {error}"))?;
        let spell = u32::try_from(spell).map_err(|error| format!("spell: {error}"))?;
        let owner = self.base().instance_id();
        self.effects.census_clear(owner);
        if let Some(played) = self.played_animation.take()
            && let Some(caster) = &self.caster
            && let Some(mut animation) = caster.try_get_node_as::<WowAnimationPlayer>("M2Animation")
        {
            animation.bind_mut().stop_action(played);
        }
        self.error = None;
        self.prepared = None;
        self.kit_request = Some((visual, kit, spell));
        Ok(())
    }

    fn play_kit(&mut self) -> Result<(Option<u16>, Option<u16>), String> {
        let kit = self.prepared.take().ok_or("kit is not prepared")?;
        let (_, _, spell) = self.kit_request.take().ok_or("kit request is missing")?;
        let requested = kit.animation.map(|animation| animation.anim_id);
        let played = if let Some(animation) = kit.animation {
            if self.fallbacks.is_none() {
                self.fallbacks = Some(read_animation_fallbacks(&self.data.join("db2/active"))?);
            }
            let caster = self.caster.as_ref().ok_or("caster is not loaded")?;
            let mut player = caster
                .try_get_node_as::<WowAnimationPlayer>("M2Animation")
                .ok_or("caster has no M2Animation")?;
            player.bind_mut().play_action(
                animation.anim_id,
                animation.looping,
                ActionPriority::Spell,
                self.fallbacks.as_ref().expect("loaded fallbacks"),
            )?
        } else {
            None
        };
        self.played_animation = played;
        self.effects
            .census_start(&kit, self.base().instance_id(), spell);
        Ok((requested, played))
    }
}
