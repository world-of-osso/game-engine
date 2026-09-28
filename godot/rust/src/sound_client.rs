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
        if let Err(error) = sound.bind_mut().load_catalog(&self.data_root) {
            sound.queue_free();
            return Err(error);
        }
        self.sound = Some(sound);
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
