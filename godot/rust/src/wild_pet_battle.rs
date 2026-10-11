//! Native wild PvE battle HUD and a single camera framing the two active pet models.
use crate::character_frame::preview::{Scene, bind_sheet_light};
use crate::world_models::UnitAppearance;
use crate::{GameClient, frame_error::FrameError, ui::RegistryUi};
use game_engine_session::SessionScreen;
use game_engine_ui_model::wild_pet_battle::{MODEL_SCENE, WildBattleView, wild_battle_screen};
use godot::classes::{CanvasLayer, Node3D};
use godot::prelude::*;
use shared::components::Npc;
use shared::protocol::WildPetBattleUpdate;
use std::collections::{BTreeMap, BTreeSet};

#[derive(Default)]
pub(crate) struct WildBattleHud {
    pub(crate) view: WildBattleView,
    ui: Option<Gd<RegistryUi>>,
    wild_creatures: Option<BTreeSet<u32>>,
    scene: Option<Scene>,
    pending: Option<([u64; 2], [u32; 2])>,
    loaded: [Option<Gd<Node3D>>; 2],
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
                &self.data_root.join("db2/12.1.0.69933/BattlePetSpecies.csv"),
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
            for id in ids {
                self.world.cancel_detached_visual(id);
            }
        }
        for model in &mut self.wild_pet_battle.loaded {
            if let Some(model) = model.take() {
                model.free();
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
            self.wild_pet_battle.pending = Some((requests, displays));
        }
        let Some((ids, displays)) = self.wild_pet_battle.pending else {
            if let Some(scene) = &mut self.wild_pet_battle.scene {
                scene.fit_viewport();
            }
            return Ok(());
        };
        for (team, id) in ids.iter().enumerate() {
            if self.wild_pet_battle.loaded[team].is_none() {
                if let Some(model) = self.world.take_detached_visual(*id) {
                    self.wild_pet_battle.loaded[team] = Some(model?);
                }
            }
        }
        if self.wild_pet_battle.loaded.iter().any(Option::is_none) {
            return Ok(());
        }
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
        for team in 0..2 {
            let mut model = self.wild_pet_battle.loaded[team]
                .take()
                .expect("both pets loaded");
            bind_sheet_light(&model);
            model.set_name(if team == 0 { "AllyPet" } else { "EnemyPet" });
            model.set_position(Vector3::new(0.0, 0.0, if team == 0 { 1.5 } else { -1.5 }));
            model.set_rotation(Vector3::new(
                0.0,
                if team == 0 {
                    std::f32::consts::FRAC_PI_2
                } else {
                    -std::f32::consts::FRAC_PI_2
                },
                0.0,
            ));
            group.add_child(&model);
        }
        let mut scene = Scene::new(host);
        scene.replace_model(group);
        self.wild_pet_battle.scene = Some(scene);
        self.wild_pet_battle.shown = displays;
        self.wild_pet_battle.pending = None;
        Ok(())
    }
}
