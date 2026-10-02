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
        let span = crate::profile::span(|| "startup.sound_catalog".to_owned());
        let loaded = sound.bind_mut().load_catalog(&self.data_root);
        drop(span);
        if let Err(error) = loaded {
            sound.queue_free();
            return Err(error);
        }
        let _span = crate::profile::span(|| "startup.footsteps".to_owned());
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

    /// `PlaySound(kit)`; nothing plays when native audio failed to start (already reported).
    pub(crate) fn play_ui_sound_kit(&mut self, kit: u32) -> Result<(), String> {
        let Some(sound) = &mut self.sound else {
            return Ok(());
        };
        sound
            .bind_mut()
            .ui_kits
            .play(&self.data_root, kit, &self.client_options.sound)
    }

    /// UI sound kits started so far, for automation.
    pub(crate) fn played_ui_sound_kits(&self) -> Vec<u32> {
        self.sound
            .as_ref()
            .map(|sound| sound.bind().ui_kits.played.clone())
            .unwrap_or_default()
    }

    pub(super) fn current_zone_id(&self) -> Option<u32> {
        if self.account.session.screen != SessionScreen::InWorld {
            return None;
        }
        self.current_zone.map(|(_, zone)| zone)
    }

    /// The original `track_player_zone`: the MCNK area under the local player and its
    /// root zone; a position no loaded chunk answers keeps the previous zone.
    pub(super) fn track_zone(&mut self) {
        if self.account.session.screen != SessionScreen::InWorld {
            return;
        }
        let Some(position) = self.world.local_player_transform().map(|t| t.origin) else {
            return;
        };
        let Some(area) = self.terrain.area_id_at(position.x, position.z) else {
            return;
        };
        if self
            .current_zone
            .is_some_and(|(current, _)| current == area)
        {
            return;
        }
        self.current_zone = Some((area, root_area(&self.area_parents, area)));
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
            .replica
            .unit(id)
            .and_then(|unit| unit.get::<shared::components::Player>())
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
