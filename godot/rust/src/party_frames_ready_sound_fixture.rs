//! Debug-only decoded playback proof using the production HUD visibility/edge helpers and UiKits.
use std::path::PathBuf;

use game_engine_core::client_options_data::SoundOptionsFile;
use godot::{
    classes::{INode, Node, ProjectSettings},
    prelude::*,
};
use shared::protocol::{ReadyCheckAnswer, ReadyCheckMemberSnapshot, ReadyCheckUpdate};

use super::{GroupFramesHud, GroupViewer, group_frames_state, ready_check_show_sound};
use crate::{sound_ui_kits::UiKits, ui::RegistryUi};
use game_engine_ui_model::group_state::GroupState;

#[derive(GodotClass)]
#[class(base=Node)]
struct ReadyCheckSoundFixture {
    base: Base<Node>,
    hud: GroupFramesHud,
    group: GroupState,
    viewer: Option<String>,
    kits: UiKits,
    data_root: PathBuf,
}

#[godot_api]
impl INode for ReadyCheckSoundFixture {
    fn init(base: Base<Node>) -> Self {
        Self {
            base,
            hud: GroupFramesHud::default(),
            group: GroupState::default(),
            viewer: Some("Bob".into()),
            kits: UiKits::new(),
            data_root: PathBuf::from(
                ProjectSettings::singleton()
                    .globalize_path("res://../data")
                    .to_string(),
            ),
        }
    }

    fn exit_tree(&mut self) {
        self.kits.stop();
        self.hud.close();
    }
}

#[godot_api]
impl ReadyCheckSoundFixture {
    #[func]
    fn initialize(&mut self, forever: bool) -> GString {
        let result = (|| {
            game_engine_ui_model::paths::set_data_root(self.data_root.clone())?;
            ui_toolkit::atlas::set_thread_skin(if forever {
                ui_toolkit::atlas::ActiveSkin::Forever
            } else {
                ui_toolkit::atlas::ActiveSkin::Modern
            });
            let mut ui = RegistryUi::new_alloc();
            self.base_mut().add_child(&ui);
            ui.bind_mut().show_group_frames(Default::default())?;
            self.hud.frames = Some(ui);
            let sound_root = self.kits.root.clone();
            self.base_mut().add_child(&sound_root);
            self.sync_and_play()
        })();
        error_string(result)
    }

    #[func]
    fn step(&mut self, phase: GString) -> GString {
        let phase = phase.to_string();
        let result = (|| {
            match phase.as_str() {
                "hidden" => self.group.ready_check = None,
                "initiator" => {
                    self.viewer = Some("Ann".into());
                    self.group.apply_ready_check(pending_update());
                }
                "listener" | "reopen" => {
                    self.viewer = Some("Bob".into());
                    self.group.apply_ready_check(pending_update());
                }
                "refresh" => {
                    let mut update = pending_update();
                    update.time_remaining_secs = 21.0;
                    self.group.apply_ready_check(update);
                }
                "answered" => {
                    let mut update = pending_update();
                    update.members[1].answer = ReadyCheckAnswer::Ready;
                    self.group.apply_ready_check(update);
                }
                "finished" => {
                    let mut update = pending_update();
                    update.finished = true;
                    self.group.apply_ready_check(update);
                }
                "no-viewer" => {
                    self.viewer = None;
                    self.group.apply_ready_check(pending_update());
                }
                _ => return Err(format!("Unknown ReadyCheck sound phase {phase}")),
            }
            self.sync_and_play()
        })();
        error_string(result)
    }

    #[func]
    fn state(&self) -> VarDictionary {
        let mut result = VarDictionary::new();
        result.set("shown", self.hud.listener_is_shown());
        result.set(
            "played",
            &Array::<i64>::from_iter(self.kits.played.iter().map(|kit| i64::from(*kit))),
        );
        result
    }
}

impl ReadyCheckSoundFixture {
    fn sync_and_play(&mut self) -> Result<(), String> {
        let state = group_frames_state(
            &self.group,
            &GroupViewer {
                local_name: self.viewer.as_deref(),
                target_name: None,
            },
            &|_| 0,
        );
        let ui = self
            .hud
            .frames
            .as_mut()
            .ok_or("Sound fixture is not initialized")?;
        ui.bind_mut().set_state(state)?;
        let shown = self.hud.listener_is_shown();
        let was_shown = std::mem::replace(&mut self.hud.ready_check_shown, shown);
        if let Some(kit) = ready_check_show_sound(was_shown, shown) {
            self.kits.play(
                &self.data_root,
                kit,
                &SoundOptionsFile {
                    master_volume: 1.0,
                    effects_volume: 1.0,
                    muted: false,
                    ..Default::default()
                },
            )?;
        }
        Ok(())
    }
}

fn pending_update() -> ReadyCheckUpdate {
    ReadyCheckUpdate {
        initiator_name: "Ann".into(),
        time_remaining_secs: 30.0,
        members: vec![
            ReadyCheckMemberSnapshot {
                name: "Ann".into(),
                answer: ReadyCheckAnswer::Ready,
            },
            ReadyCheckMemberSnapshot {
                name: "Bob".into(),
                answer: ReadyCheckAnswer::Pending,
            },
        ],
        finished: false,
    }
}

fn error_string(result: Result<(), String>) -> GString {
    result.err().unwrap_or_default().as_str().into()
}
