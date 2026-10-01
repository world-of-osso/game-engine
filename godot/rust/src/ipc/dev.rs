//! Dev-tool requests answered from the live native client, in the original engine's
//! response text: `status network|sound|terrain` (src/ipc/format_status.rs,
//! src/ipc/format_terrain.rs), `map position` (src/ipc/format.rs), `camera set`
//! (src/ipc/plugin/camera_direction.rs), `hover` (src/ipc/plugin/hover.rs) and
//! `movement forward|stop` (src/ipc/plugin.rs).
use game_engine_network::{
    game_state_enum::GameState,
    ipc_wire::{Request, Response},
};
use godot::{
    classes::{AudioStreamPlayer, AudioStreamPlayer3D, InputEventMouseMotion},
    prelude::*,
};
use shared::components::Npc;

use crate::process_memory_status::current_process_memory_kb;

/// Height above an NPC's origin (its feet) that `hover --npc` aims at, in yards.
const NPC_AIM_HEIGHT: f32 = 1.0;

impl crate::GameClient {
    /// The dev-tool request's response, or the request when the client does not serve it.
    pub(crate) fn dev_request(&mut self, request: Request) -> Result<Response, Request> {
        let answer = match request {
            Request::NetworkStatus => Ok(self.network_status()),
            Request::SoundStatus => Ok(self.sound_status()),
            Request::TerrainStatus => Ok(self.terrain_status()),
            Request::MapPosition => self.map_position(),
            Request::SetCameraDirection {
                yaw_degrees,
                pitch_degrees,
            } => self.set_camera_direction(yaw_degrees, pitch_degrees),
            Request::ScriptedMovementForward {
                duration_secs,
                heading_degrees,
            } => self.start_scripted_movement(duration_secs, heading_degrees),
            Request::ScriptedMovementStop => {
                self.scripted_movement.stop();
                Ok("scripted movement stopped".into())
            }
            Request::HoverAt { x, y } => self.hover_at(Vector2::new(x, y)),
            Request::ExportScene { output_path } => {
                let semantic = self
                    .semantic_scene()
                    .map(|scene| scene.as_ref().map(crate::scene_export::snapshot));
                return Ok(super::export_scene(
                    &self.base().clone().upcast(),
                    semantic,
                    &output_path,
                ));
            }
            Request::HoverNpc { name } => self
                .npc_screen_point(&name)
                .and_then(|point| self.hover_at(point)),
            request => return Err(request),
        };
        Ok(match answer {
            Ok(text) => Response::Text(text),
            Err(error) => Response::Error(error),
        })
    }

    fn in_world(&self) -> bool {
        self.automation_game_state() == GameState::InWorld
    }

    fn network_status(&self) -> String {
        let link = self.account.link.as_ref();
        let connected = link.is_some_and(|link| link.connected);
        format!(
            "server_addr: {}\ngame_state: {:?}\nconnected: {}\nconnected_links: {}\nlocal_client_id: {}\nzone_id: {}\nremote_entities: {}\nlocal_players: {}\nchat_messages: {}",
            link.map_or_else(|| "-".into(), |link| link.server.to_string()),
            self.automation_game_state(),
            connected,
            usize::from(connected),
            link.map_or_else(|| "-".into(), |link| link.client_id.to_string()),
            self.current_zone_id().unwrap_or(0),
            self.replica.len(),
            usize::from(self.world.local_player_id().is_some()),
            self.chat.model.log.messages.len(),
        )
    }

    fn sound_status(&self) -> String {
        let options = &self.client_options.sound;
        let ambient = self
            .sound
            .as_ref()
            .is_some_and(|sound| sound.bind().ambient_playing());
        format!(
            "enabled: {}\nmuted: {}\nmaster_volume: {:.2}\nambient_volume: {:.2}\nambient_entities: {}\nactive_sinks: {}",
            self.sound.is_some(),
            options.muted,
            options.master_volume,
            options.ambient_volume,
            usize::from(ambient),
            self.playing_audio_players(),
        )
    }

    /// Audio players under the client that are playing: music, ambience, effects,
    /// footsteps and spell sounds.
    fn playing_audio_players(&self) -> usize {
        let client = self.base();
        let flat = client
            .find_children_ex("*")
            .type_("AudioStreamPlayer")
            .owned(false)
            .done()
            .iter_shared()
            .filter_map(|node| node.try_cast::<AudioStreamPlayer>().ok())
            .filter(|player| player.is_playing())
            .count();
        let spatial = client
            .find_children_ex("*")
            .type_("AudioStreamPlayer3D")
            .owned(false)
            .done()
            .iter_shared()
            .filter_map(|node| node.try_cast::<AudioStreamPlayer3D>().ok())
            .filter(|player| player.is_playing())
            .count();
        flat + spatial
    }

    /// The streamed map's request and tile state, and this process's memory.
    fn terrain_status(&self) -> String {
        let state = self.terrain.state();
        let memory = current_process_memory_kb();
        let primary = self
            .terrain
            .primary_tile()
            .map_or_else(|| "-".into(), |(y, x)| format!("{y},{x}"));
        format!(
            "map_name: {}\ninitial_tile: {primary}\ninitial_tiles: {}\nloaded_tiles: {}\npending_tiles: {}\nfailed_tiles: {}\nprocess_rss_kb: {}\nprocess_anon_kb: {}\nprocess_data_kb: {}",
            state.map.as_deref().unwrap_or("-"),
            self.terrain.initial_tiles().len(),
            state.parsed_tiles.len(),
            state.pending_tiles.len(),
            state.failures.len(),
            memory.rss_kb,
            memory.anon_kb,
            memory.data_kb,
        )
    }

    /// The native client has no map waypoint or graveyard marker state.
    fn map_position(&self) -> Result<String, String> {
        let origin = self
            .world
            .local_player_transform()
            .ok_or("map position requires a local player")?
            .origin;
        Ok(format!(
            "zone_id: {}\nposition: {:.2},{:.2}\nwaypoint: -\ngraveyard_marker: -",
            self.current_zone_id().unwrap_or(0),
            origin.x,
            origin.z
        ))
    }

    fn set_camera_direction(
        &mut self,
        yaw_degrees: Option<f32>,
        pitch_degrees: Option<f32>,
    ) -> Result<String, String> {
        if !self.in_world() {
            return Err("camera direction requires InWorld".into());
        }
        if self.world_camera.camera().is_none() {
            return Err("no active in-world camera".into());
        }
        self.world_camera
            .set_direction_degrees(yaw_degrees, pitch_degrees)?;
        Ok(format!(
            "camera yaw={:.3} pitch={:.3} degrees",
            self.world_camera.yaw().to_degrees(),
            self.world_camera.pitch().to_degrees()
        ))
    }

    fn start_scripted_movement(
        &mut self,
        duration_secs: f32,
        heading_degrees: Option<f32>,
    ) -> Result<String, String> {
        if !self.in_world() {
            return Err("scripted movement requires InWorld".into());
        }
        self.scripted_movement
            .start(duration_secs, heading_degrees)?;
        Ok("scripted movement started".into())
    }

    /// Window position of the nearest NPC named `name` in front of the world camera.
    fn npc_screen_point(&self, name: &str) -> Result<Vector2, String> {
        let camera = self
            .world_camera
            .camera()
            .ok_or("no active in-world camera")?;
        let eye = camera.get_global_position();
        self.replica
            .units()
            .filter(|unit| {
                unit.get::<Npc>()
                    .is_some_and(|npc| npc.name.eq_ignore_ascii_case(name))
            })
            .filter_map(|unit| {
                let aim = self.world.unit_node(unit.server_id)?.get_global_position()
                    + Vector3::UP * NPC_AIM_HEIGHT;
                (!camera.is_position_behind(aim))
                    .then(|| (aim.distance_squared_to(eye), camera.unproject_position(aim)))
            })
            .min_by(|a, b| a.0.total_cmp(&b.0))
            .map(|(_, point)| point)
            .ok_or_else(|| format!("no NPC named {name} in view"))
    }

    /// Moves the pointer to `point` through the root window's input, so hover-driven UI
    /// (unit and frame tooltips) follows it until a real pointer event moves it.
    fn hover_at(&self, point: Vector2) -> Result<String, String> {
        let mut root = self.base().get_tree().get_root();
        let size = root.get_visible_rect().size;
        if point.x < 0.0 || point.y < 0.0 || point.x > size.x || point.y > size.y {
            return Err(format!(
                "({:.0}, {:.0}) is outside the window",
                point.x, point.y
            ));
        }
        let mut motion = InputEventMouseMotion::new_gd();
        motion.set_position(point);
        motion.set_global_position(point);
        // This runs inside the client's process; the client's own input handler sees
        // the event after it returns.
        root.call_deferred("push_input", &[motion.to_variant(), true.to_variant()]);
        Ok(format!("cursor at ({:.0}, {:.0})", point.x, point.y))
    }
}
