//! Connect local-player terrain areas and persisted options to native audio.

use std::{collections::HashMap, fs::File, io::BufReader, path::Path};

use game_engine_core::area_zone_data::{load_area_parents, root_area};
use game_engine_session::SessionScreen;
use godot::classes::INode;
use godot::prelude::*;

use crate::{
    GameClient,
    background_load::BackgroundLoad,
    sound::{NativeSound, ZoneTracks, read_zone_tracks},
    sound_footsteps::{FootstepFiles, read_footstep_files},
};

/// Sound tables read from local data on a thread from client start: zone parents, zone
/// music and ambience, and footsteps (whose files the 143 MB community listfile names).
pub(crate) struct SoundData {
    area_parents: Result<HashMap<u32, u32>, String>,
    zone_tracks: Result<ZoneTracks, String>,
    footsteps: Result<FootstepFiles, String>,
}

pub(crate) fn start_sound_data(data_root: &Path) -> BackgroundLoad<SoundData> {
    let data_root = data_root.to_owned();
    BackgroundLoad::start("sound-data", move || SoundData {
        area_parents: read_area_parents(&data_root),
        zone_tracks: read_zone_tracks(&data_root),
        footsteps: read_footstep_files(&data_root),
    })
}

fn read_area_parents(data_root: &Path) -> Result<HashMap<u32, u32>, String> {
    let path = data_root.join("AreaTable.csv");
    let file =
        File::open(&path).map_err(|error| format!("Cannot open {}: {error}", path.display()))?;
    load_area_parents(BufReader::new(file), &path)
}

impl GameClient {
    pub(super) fn initialize_sound(&mut self) {
        let mut sound = Gd::<NativeSound>::from_init_fn(NativeSound::init);
        sound.set_name("NativeSound");
        self.base_mut().add_child(&sound);
        self.sound = Some(sound);
    }

    /// Hand the sound tables to native audio once their thread is done; until then no
    /// zone music, ambience or footstep plays.
    pub(super) fn apply_loaded_sound(&mut self) -> Result<(), String> {
        let Some(load) = self.sound_data.as_mut() else {
            return Ok(());
        };
        if load.poll().is_none() {
            return Ok(());
        }
        let data = self
            .sound_data
            .take()
            .and_then(BackgroundLoad::into_loaded)
            .expect("polled above");
        let mut sound = self.sound.clone().expect("created at startup");
        let mut sound = sound.bind_mut();
        if let Err(error) = data
            .footsteps
            .and_then(|files| sound.apply_footsteps(files))
        {
            godot_error!("Native footsteps unavailable: {error}");
        }
        sound.apply_zone_tracks(data.zone_tracks?);
        self.area_parents = Some(data.area_parents?);
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
        let Some(parents) = &self.area_parents else {
            return;
        };
        if self
            .current_zone
            .is_some_and(|(current, _)| current == area)
        {
            return;
        }
        self.current_zone = Some((area, root_area(parents, area)));
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
