//! Connect local-player terrain areas and persisted options to native audio.

use std::{fs::File, io::BufReader};

use game_engine_core::area_zone_data::{load_area_parents, root_area};
use game_engine_session::SessionScreen;
use godot::classes::INode;
use godot::prelude::*;

use crate::{GameClient, sound::NativeSound};

impl GameClient {
    pub(super) fn initialize_sound(&mut self) -> Result<(), String> {
        let path = self.data_root.join("AreaTable.csv");
        let file = File::open(&path)
            .map_err(|error| format!("Cannot open {}: {error}", path.display()))?;
        self.area_parents = load_area_parents(BufReader::new(file), &path)?;
        let mut sound = Gd::<NativeSound>::from_init_fn(NativeSound::init);
        sound.set_name("NativeSound");
        self.base_mut().add_child(&sound);
        let loaded = sound.bind_mut().load_catalog(&self.data_root);
        if let Err(error) = loaded {
            sound.queue_free();
            return Err(error);
        }
        if let Err(error) = sound.bind_mut().load_footsteps(&self.data_root) {
            godot_error!("Native footsteps unavailable: {error}");
        }
        self.sound = Some(sound);
        Ok(())
    }

    pub(super) fn play_ui_clicks(&mut self) -> Result<(), String> {
        let mut clicks = 0;
        self.for_each_registry_ui(|ui| {
            clicks += ui.bind_mut().sync_pointer_clicks()?;
            Ok(())
        })?;
        if let Some(sound) = &mut self.sound {
            let settings = &self.client_options.sound;
            for _ in 0..clicks {
                sound.bind_mut().play_ui_click(
                    settings.master_volume,
                    settings.effects_volume,
                    settings.muted,
                );
            }
        }
        Ok(())
    }

    pub(super) fn current_zone_id(&self) -> Option<u32> {
        if self.account.session.screen != SessionScreen::InWorld {
            return None;
        }
        let position = self.world.local_player_transform()?.origin;
        let area = self.terrain.area_id_at(position.x, position.z)?;
        Some(root_area(&self.area_parents, area))
    }

    pub(super) fn update_footsteps(&mut self) -> Result<(), String> {
        let Some(sound) = &mut self.sound else {
            return Ok(());
        };
        let sample = if self.account.session.screen == SessionScreen::InWorld {
            self.world.local_footstep_phase()
        } else {
            None
        };
        let Some((id, position, phase)) = sample else {
            sound.bind_mut().stop_footsteps();
            return Ok(());
        };
        let Some(race) = self
            .units
            .get(&id)
            .and_then(|unit| unit.player.as_ref())
            .map(|player| player.race)
        else {
            sound.bind_mut().stop_footsteps();
            return Ok(());
        };
        let surface = self
            .terrain
            .surface_at_position([position.x, position.y, position.z]);
        sound.bind_mut().observe_footstep(
            id,
            race,
            phase,
            position,
            surface,
            &self.client_options.sound,
        );
        Ok(())
    }

    pub(super) fn update_sound(&mut self) -> Result<(), String> {
        let zone = self.current_zone_id();
        if let Some(sound) = &mut self.sound {
            sound.bind_mut().sync(zone, &self.client_options.sound)?;
        }
        Ok(())
    }

    pub(super) fn stop_sound(&mut self) {
        if let Some(sound) = &mut self.sound {
            sound.bind_mut().stop();
        }
    }
}
