//! Retail pet comparison player in its own world/camera, never mount-attached.
use super::{GameClient, mount_preview};
use crate::{
    character_frame::preview::{Scene, bind_sheet_light},
    world_models::UnitAppearance,
};
use game_engine_core::{
    model_scene_actor::ModelSceneActor,
    trading_post_pet::{PetComparison, PetScenes},
};
use godot::{classes::Node3D, prelude::*};
use shared::components::SheathState;

#[derive(Default)]
pub(super) struct PetPlayerPreview {
    scene: Option<Scene>,
    pub(super) pending: Option<(u64, UnitAppearance)>,
    pub(super) shown: Option<UnitAppearance>,
    pub(super) actor: Option<InstanceId>,
}

pub(super) fn stage_actor(
    name: &str,
    model: Gd<Node3D>,
    authored: &ModelSceneActor,
) -> Result<Gd<Node3D>, String> {
    match mount_preview::stage_actor(name, model.clone(), authored) {
        Ok(actor) => Ok(actor),
        Err(error) => {
            model.free();
            Err(error)
        }
    }
}

impl GameClient {
    pub(super) fn read_trading_pet(&self) -> Result<PetComparison, String> {
        let view = &self.trading_post.view;
        let snapshot = view
            .snapshot
            .as_ref()
            .ok_or("Trading Post snapshot missing")?;
        let category_scene = |id| {
            snapshot
                .categories
                .iter()
                .find(|row| row.id == id)
                .map(|row| row.model_scene_id)
                .ok_or_else(|| format!("Trading Post category{id} scene missing"))
        };
        let scenes = PetScenes {
            category: category_scene(3)?,
            display: view.preview_scene_id()?,
            player: category_scene(1)?,
        };
        let Some(UnitAppearance::Player(player, ..)) = self.world.local_player_appearance() else {
            return Err("Trading Post comparison player appearance unavailable".into());
        };
        PetComparison::read(&self.data_root, scenes, player.race, player.appearance.sex)
    }

    pub(super) fn clear_trading_pet_player(&mut self) {
        let preview = std::mem::take(&mut self.trading_post.pet_player);
        if let Some((id, _)) = preview.pending {
            self.world.cancel_detached_visual(id);
        }
        if let Some(scene) = preview.scene {
            scene.free();
        }
    }

    pub(super) fn sync_trading_pet_player(&mut self) -> Result<(), String> {
        let Some(comparison) = self.trading_post.pet.clone() else {
            return Ok(());
        };
        self.request_trading_pet_player();
        let preview = &self.trading_post.pet_player;
        if preview.pending.is_none() && preview.shown.is_none() {
            return Ok(());
        }
        self.create_trading_pet_player_scene()?;
        self.attach_trading_pet_player(&comparison.player)?;
        let scene = self
            .trading_post
            .pet_player
            .scene
            .as_mut()
            .expect("comparison scene created");
        scene.fit_viewport();
        let camera = &comparison.player_camera;
        scene.set_authored_camera(camera, 0., camera.zoom_percent());
        Ok(())
    }

    fn request_trading_pet_player(&mut self) {
        // SetupModelSceneForTransmogs(nil): current dress, visible sheathed weapons.
        let desired = self.world.local_player_appearance().cloned();
        let preview = &self.trading_post.pet_player;
        let current = preview
            .pending
            .as_ref()
            .map(|(_, appearance)| appearance)
            .or(preview.shown.as_ref());
        if desired.as_ref() == current {
            return;
        }
        self.clear_trading_pet_player();
        if let Some(appearance) = desired {
            let id = self.world.request_detached_visual(&appearance);
            self.trading_post.pet_player.pending = Some((id, appearance));
        }
    }

    fn create_trading_pet_player_scene(&mut self) -> Result<(), String> {
        if self.trading_post.pet_player.scene.is_some() {
            return Ok(());
        }
        let host = self
            .trading_post
            .ui
            .as_ref()
            .ok_or("Trading Post UI missing")?
            .bind()
            .frame_control(game_engine_ui_model::trading_post::PLAYER_MODEL)
            .ok_or("Trading Post comparison player host missing")?;
        self.trading_post.pet_player.scene = Some(Scene::new(host));
        Ok(())
    }

    fn attach_trading_pet_player(&mut self, authored: &ModelSceneActor) -> Result<(), String> {
        let Some((id, _)) = &self.trading_post.pet_player.pending else {
            return Ok(());
        };
        let Some(loaded) = self.world.take_detached_visual(*id) else {
            return Ok(());
        };
        let (_, appearance) = self
            .trading_post
            .pet_player
            .pending
            .take()
            .expect("pending comparison player");
        let actor = self.sheath_and_stage_pet_player(loaded?, &appearance, authored)?;
        let preview = &mut self.trading_post.pet_player;
        preview.actor = Some(actor.instance_id());
        preview
            .scene
            .as_mut()
            .ok_or("Comparison player scene missing")?
            .replace_unframed_model(actor);
        preview.shown = Some(appearance);
        Ok(())
    }

    fn sheath_and_stage_pet_player(
        &mut self,
        model: Gd<Node3D>,
        appearance: &UnitAppearance,
        authored: &ModelSceneActor,
    ) -> Result<Gd<Node3D>, String> {
        if let Err(error) =
            self.world
                .place_player_weapons(&model, appearance, SheathState::Unarmed)
        {
            model.free();
            return Err(error);
        }
        bind_sheet_light(&model);
        stage_actor("TradingPostComparisonPlayerActor", model, authored)
    }
}
