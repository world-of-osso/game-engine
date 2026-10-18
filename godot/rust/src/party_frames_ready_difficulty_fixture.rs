//! Debug-only native text proof: supplied authoritative-shaped context, production
//! Account selector, DB2 catalog and HUD projection. Not an authenticated session.
use std::path::PathBuf;

use game_engine_ui_model::ready_check_difficulty::ReadyCheckDifficulties;
use godot::{
    classes::{INode, Node, ProjectSettings},
    prelude::*,
};
use shared::protocol::{
    RaidDifficultySet, ReadyCheckAnswer, ReadyCheckMemberSnapshot, ReadyCheckUpdate,
    WorldServerInfo,
};

use super::{GroupFramesHud, GroupViewer, group_frames_state};
use crate::{account::Account, ui::RegistryUi};

#[derive(GodotClass)]
#[class(base=Node)]
struct ReadyCheckDifficultyFixture {
    base: Base<Node>,
    hud: GroupFramesHud,
    account: Account,
    catalog: Option<ReadyCheckDifficulties>,
    data_root: PathBuf,
}

#[godot_api]
impl INode for ReadyCheckDifficultyFixture {
    fn init(base: Base<Node>) -> Self {
        let data_root = PathBuf::from(
            ProjectSettings::singleton()
                .globalize_path("res://../data")
                .to_string(),
        );
        Self {
            base,
            hud: GroupFramesHud::default(),
            account: Account::new(data_root.clone()),
            catalog: None,
            data_root,
        }
    }

    fn exit_tree(&mut self) {
        self.hud.close();
    }
}

#[godot_api]
impl ReadyCheckDifficultyFixture {
    #[func]
    fn initialize(&mut self, forever: bool) -> GString {
        let result = (|| {
            game_engine_ui_model::paths::set_data_root(self.data_root.clone())?;
            ui_toolkit::atlas::set_thread_skin(if forever {
                ui_toolkit::atlas::ActiveSkin::Forever
            } else {
                ui_toolkit::atlas::ActiveSkin::Modern
            });
            self.catalog = Some(ReadyCheckDifficulties::load(
                &self.data_root.join("db2/active"),
            )?);
            let mut ui = RegistryUi::new_alloc();
            self.base_mut().add_child(&ui);
            ui.bind_mut().show_group_frames(Default::default())?;
            self.hud.frames = Some(ui);
            self.account.group.apply_ready_check(pending_check());
            Ok(())
        })();
        error_string(result)
    }

    /// Supplied long-name stress case; uses the same production account/UI projection.
    #[func]
    fn set_initiator_name(&mut self, name: GString) -> GString {
        let result = (|| {
            let ready = self
                .account
                .group
                .ready_check
                .as_mut()
                .ok_or("Ready check is absent")?;
            ready.update.initiator_name = name.to_string();
            self.sync_context()
        })();
        error_string(result)
    }

    #[func]
    fn step(&mut self, phase: GString) -> GString {
        let result = self.apply_phase(&phase.to_string());
        error_string(result)
    }
}

impl ReadyCheckDifficultyFixture {
    fn apply_phase(&mut self, phase: &str) -> Result<(), String> {
        let (current, selected, raid) = match phase {
            "actual3" => (Some(3), 14, true),
            "actual4" => (Some(4), 14, true),
            "actual5" => (Some(5), 14, true),
            "actual6" => (Some(6), 14, true),
            "continent-raid" => (Some(0), 14, true),
            "continent-dungeon" => (Some(0), 1, false),
            "selected4" => (Some(0), 4, true),
            "current14" => (Some(14), 3, true),
            "transfer" => (None, 14, true),
            _ => return Err(format!("Unknown ReadyCheck difficulty phase {phase}")),
        };
        self.account.world_server_info = current.map(|difficulty_id| WorldServerInfo {
            map_id: if difficulty_id == 0 { 0 } else { 631 },
            difficulty_id,
        });
        self.account.group.is_raid = raid;
        self.account.raid_difficulty = Some(RaidDifficultySet {
            difficulty_id: selected,
            legacy_difficulty_id: 6,
        });
        self.account.dungeon_difficulty = Some(selected);
        self.sync_context()
    }

    fn sync_context(&mut self) -> Result<(), String> {
        let mut state = group_frames_state(
            &self.account.group,
            &GroupViewer {
                local_name: Some("Bob"),
                target_name: None,
            },
            &|_| 0,
        );
        state.ready_check.difficulty_name = self.account.ready_check_difficulty_name(
            self.catalog
                .as_ref()
                .ok_or("Difficulty fixture not initialized")?,
        )?;
        self.hud
            .frames
            .as_mut()
            .ok_or("Difficulty HUD not initialized")?
            .bind_mut()
            .set_state(state)
    }
}

fn pending_check() -> ReadyCheckUpdate {
    ReadyCheckUpdate {
        initiator_name: "Ann".into(),
        time_remaining_secs: 30.0,
        finished: false,
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
    }
}

fn error_string(result: Result<(), String>) -> GString {
    result.err().unwrap_or_default().as_str().into()
}
