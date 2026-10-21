//! Native wild PvE battle HUD and a single camera framing the two active pet models.
use crate::animation::WowAnimationPlayer;
use crate::character_frame::preview::{Scene, bind_sheet_light, pet_actor_bounds};
use crate::world_models::UnitAppearance;
use crate::{GameClient, frame_error::FrameError, ui::RegistryUi};
use game_engine_core::model_scene_actor::{ModelSceneActor, read_scene_actors};
use game_engine_session::SessionScreen;
use game_engine_ui_model::wild_pet_battle::{MODEL_SCENE, WildBattleView, wild_battle_screen};
use godot::classes::{CanvasLayer, Node3D};
use godot::prelude::*;
use shared::components::Npc;
use shared::protocol::WildPetBattleUpdate;
use std::collections::{BTreeMap, BTreeSet};

const BATTLE_PAD_OFFSET: f32 = 2.2;
/// PET_BATTLE_PAD_USES_JOURNAL_CARD_NORMALIZATION (project policy): the battlefield has no
/// sourced camera or actor scale, so each pad normalizes like the Pet Journal card actor
/// (BattlePetSpecies.CardUIModelSceneID 33, `unwrapped`) over the journal's own bounds.
const JOURNAL_CARD_SCENE: u32 = 33;
const JOURNAL_CARD_ACTOR: &str = "unwrapped";

#[derive(Default)]
pub(crate) struct WildBattleHud {
    pub(crate) view: WildBattleView,
    ui: Option<Gd<RegistryUi>>,
    wild_creatures: Option<BTreeSet<u32>>,
    journal_card: Option<ModelSceneActor>,
    scene: Option<Scene>,
    pending: Option<([Option<u64>; 2], [u32; 2])>,
    shown: [u32; 2],
    hidden_layers: BTreeMap<InstanceId, bool>,
}
impl WildBattleHud {
    pub(crate) fn visit_uis(
        &mut self,
        visit: &mut impl FnMut(&mut Gd<RegistryUi>) -> Result<(), String>,
    ) -> Result<(), String> {
        if let Some(ui) = &mut self.ui {
            visit(ui)?;
        }
        Ok(())
    }
}
impl GameClient {
    pub(crate) fn start_clicked_wild_pet(&mut self, unit: u64) -> Result<bool, FrameError> {
        let Some(npc) = self.replica.unit(unit).and_then(|unit| unit.get::<Npc>()) else {
            return Ok(false);
        };
        let creature = npc.template_id;
        if self.wild_pet_battle.wild_creatures.is_none() {
            let mut creatures = BTreeSet::new();
            // This is local visual/input metadata only; the server validates SQLite species,
            // ownership, range, map copy, target life and exclusive battle claims.
            game_engine_core::csv_util::read_numeric_rows(
                &self.data_root.join("db2/active/BattlePetSpecies.csv"),
                ["CreatureID", "SourceTypeEnum"],
                |[creature, source]| {
                    if source == 4 {
                        creatures.insert(creature as u32);
                    }
                },
            )?;
            self.wild_pet_battle.wild_creatures = Some(creatures);
        }
        if !self
            .wild_pet_battle
            .wild_creatures
            .as_ref()
            .expect("loaded catalog")
            .contains(&creature)
        {
            return Ok(false);
        }
        self.account.send_wild_battle_start(unit)?;
        Ok(true)
    }
    pub(super) fn receive_wild_pet_battle(&mut self, update: WildPetBattleUpdate) {
        if let WildPetBattleUpdate::Rejected(error) = &update {
            godot_warn!("Pet battle request rejected: {error}");
        }
        if matches!(update, WildPetBattleUpdate::Start(_)) {
            self.close_pet_journal();
            self.close_toybox();
        }
        self.wild_pet_battle.view.receive(update);
    }
    pub(super) fn update_wild_pet_battle(&mut self, delta: f32) -> Result<(), FrameError> {
        self.wild_pet_battle.view.advance_clock(delta);
        self.wild_pet_battle.view.feedback_seconds =
            (self.wild_pet_battle.view.feedback_seconds - delta).max(0.0);
        if self.account.session.screen != SessionScreen::InWorld
            || self.wild_pet_battle.view.state.is_none()
        {
            self.clear_battle_scene();
            if let Some(ui) = self.wild_pet_battle.ui.take() {
                ui.free();
            }
            return Ok(());
        }
        if let Some(mut ui) = self.wild_pet_battle.ui.clone() {
            loop {
                let action = ui.bind_mut().pop_action().to_string();
                if action.is_empty() {
                    break;
                }
                if let Some(request) = self.wild_pet_battle.view.action(&action) {
                    self.account.send_wild_battle_action(request)?;
                }
            }
        }
        if self.wild_pet_battle.view.state.is_none() {
            return Ok(());
        }
        self.sync_battle_hud()?;
        self.sync_battle_scene()?;
        Ok(())
    }
    fn sync_battle_hud(&mut self) -> Result<(), String> {
        let scale = self.effective_ui_scale();
        let size = self
            .base()
            .get_viewport()
            .ok_or("PetBattle has no viewport")?
            .get_visible_rect()
            .size;
        self.wild_pet_battle.view.viewport = [size.x / scale, size.y / scale];
        let view = self.wild_pet_battle.view.clone();
        self.extract_art(&crate::quests::screen_texture_fdids(
            view.clone(),
            wild_battle_screen,
        ));
        if let Some(ui) = &mut self.wild_pet_battle.ui {
            ui.bind_mut().set_ui_scale(scale)?;
            return ui.bind_mut().set_state(view);
        }
        let mut ui = RegistryUi::new_alloc();
        ui.set_name("PetBattleUI");
        ui.set_layer(7);
        self.base_mut().add_child(&ui);
        let result = {
            let mut host = ui.bind_mut();
            host.set_ui_scale(scale)
                .and_then(|()| host.show_wild_pet_battle(view))
        };
        if let Err(error) = result {
            ui.free();
            return Err(error);
        }
        self.wild_pet_battle.ui = Some(ui);
        Ok(())
    }
    fn clear_battle_scene(&mut self) {
        if let Some((ids, _)) = self.wild_pet_battle.pending.take() {
            for id in ids.into_iter().flatten() {
                self.world.cancel_detached_visual(id);
            }
        }
        if let Some(scene) = self.wild_pet_battle.scene.take() {
            scene.free();
        }
        self.wild_pet_battle.shown = [0; 2];
    }
    pub(super) fn reset_wild_pet_battle(&mut self) {
        self.wild_pet_battle.view.state = None;
        self.sync_wild_battle_ui_visibility();
        self.clear_battle_scene();
        if let Some(ui) = self.wild_pet_battle.ui.take() {
            ui.free();
        }
        self.wild_pet_battle = WildBattleHud::default();
    }
    pub(super) fn sync_wild_battle_ui_visibility(&mut self) {
        let active = self.account.session.screen == SessionScreen::InWorld
            && self.wild_pet_battle.view.state.is_some();
        let battle_ui = self.wild_pet_battle.ui.as_ref().map(Gd::instance_id);
        let menu_ui = self.game_menu_ui.as_ref().map(Gd::instance_id);
        let children = self.base().get_children();
        for child in children.iter_shared() {
            let Ok(mut layer) = child.try_cast::<CanvasLayer>() else {
                continue;
            };
            let id = layer.instance_id();
            if Some(id) == battle_ui || Some(id) == menu_ui || layer.get_name() == "FpsOverlay" {
                continue;
            }
            if active {
                self.wild_pet_battle
                    .hidden_layers
                    .entry(id)
                    .or_insert_with(|| layer.is_visible());
                layer.set_visible(false);
            } else if let Some(visible) = self.wild_pet_battle.hidden_layers.remove(&id) {
                layer.set_visible(visible);
            }
        }
        if !active {
            self.wild_pet_battle.hidden_layers.clear();
        }
    }
    fn sync_battle_scene(&mut self) -> Result<(), String> {
        let state = self
            .wild_pet_battle
            .view
            .state
            .as_ref()
            .expect("active battle");
        let displays = std::array::from_fn(|team| {
            state.teams[team][usize::from(state.active[team])].display_id
        });
        let requested = self
            .wild_pet_battle
            .pending
            .as_ref()
            .map(|(_, display)| *display)
            .unwrap_or(self.wild_pet_battle.shown);
        if displays != requested {
            self.clear_battle_scene();
            let requests = displays.map(|display_id| {
                self.world
                    .request_detached_visual(&UnitAppearance::Creature {
                        display_id,
                        items: Default::default(),
                    })
            });
            let host = self
                .wild_pet_battle
                .ui
                .as_ref()
                .expect("battle UI")
                .bind()
                .frame_control(MODEL_SCENE)
                .ok_or("PetBattleModelScene control missing")?;
            let mut group = Node3D::new_alloc();
            group.set_name("PetBattleActiveModels");
            let mut scene = Scene::new(host);
            scene.replace_model(group);
            self.wild_pet_battle.scene = Some(scene);
            self.wild_pet_battle.pending = Some((requests.map(Some), displays));
        }
        let Some((mut ids, displays)) = self.wild_pet_battle.pending else {
            if let Some(scene) = &mut self.wild_pet_battle.scene {
                scene.fit_viewport();
            }
            return Ok(());
        };
        for (team, request) in ids.iter_mut().enumerate() {
            let Some(id) = *request else {
                continue;
            };
            let Some(result) = self.world.take_detached_visual(id) else {
                continue;
            };
            *request = None;
            let model = match result {
                Ok(model) => model,
                Err(error) => {
                    godot_error!("Pet battle team {team} display {}: {error}", displays[team]);
                    continue;
                }
            };
            let pad_scale = match self.journal_card_pad_scale(&model) {
                Ok(scale) => scale,
                Err(error) => {
                    model.free();
                    godot_error!("Pet battle team {team} display {}: {error}", displays[team]);
                    continue;
                }
            };
            stage_battle_pet(
                self.wild_pet_battle
                    .scene
                    .as_mut()
                    .expect("battle scene created with requests"),
                model,
                team,
                pad_scale,
            )?;
        }
        if ids.iter().all(Option::is_none) {
            self.wild_pet_battle.shown = displays;
            self.wild_pet_battle.pending = None;
        } else {
            self.wild_pet_battle.pending = Some((ids, displays));
        }
        Ok(())
    }

    /// PetInstanceScale normalized by the journal card actor over the received visual's
    /// journal bounds, measured before pad staging re-orients it.
    fn journal_card_pad_scale(&mut self, model: &Gd<Node3D>) -> Result<f32, String> {
        if self.wild_pet_battle.journal_card.is_none() {
            let card = read_scene_actors(&self.data_root, JOURNAL_CARD_SCENE)?
                .remove(JOURNAL_CARD_ACTOR)
                .ok_or_else(|| {
                    format!("UiModelScene {JOURNAL_CARD_SCENE} has no {JOURNAL_CARD_ACTOR} actor")
                })?;
            self.wild_pet_battle.journal_card = Some(card);
        }
        let bounds = pet_actor_bounds(model)?;
        let size = glam::Vec3::new(bounds.size.x, bounds.size.y, bounds.size.z);
        let pet_instance_scale = model.get_meta("pet_instance_scale").to::<f32>();
        let card = self
            .wild_pet_battle
            .journal_card
            .as_ref()
            .expect("journal card loaded");
        Ok(card.normalized_scale_of(pet_instance_scale, size))
    }
}

fn repeat_battle_idle(model: &Gd<Node3D>) -> Result<(), String> {
    let animations = model
        .find_children_ex("*")
        .type_("WowAnimationPlayer")
        .owned(false)
        .done();
    for node in animations.iter_shared() {
        node.cast::<WowAnimationPlayer>()
            .bind_mut()
            .use_battle_idle()?;
    }
    Ok(())
}

fn stage_battle_pet(
    scene: &mut Scene,
    mut model: Gd<Node3D>,
    team: usize,
    pad_scale: f32,
) -> Result<(), String> {
    if let Err(error) = repeat_battle_idle(&model) {
        model.free();
        return Err(format!("Battle team {team} idle: {error}"));
    }
    bind_sheet_light(&model);
    model.set_rotation(Vector3::new(
        0.0,
        if team == 0 {
            std::f32::consts::FRAC_PI_2
        } else {
            -std::f32::consts::FRAC_PI_2
        },
        0.0,
    ));
    let mut pad = Node3D::new_alloc();
    pad.set_name(if team == 0 { "AllyPet" } else { "EnemyPet" });
    pad.add_child(&model);
    scene.add_model_child(&pad);
    pad.set_scale(Vector3::ONE * pad_scale);
    let side = if team == 0 { 1.0 } else { -1.0 };
    pad.set_position(Vector3::new(0.0, 0.0, side * BATTLE_PAD_OFFSET));
    Ok(())
}
