//! Offline process fixture for res://tests/party_portraits.gd; no client or server.
use std::path::PathBuf;

use game_engine_core::ui_layout_data::LayoutSettings;
use game_engine_network::replica::Replica;
use game_engine_ui_model::group_state::GroupState;
use godot::classes::{Node, ProjectSettings};
use godot::prelude::*;
use shared::components::{
    CharacterAppearance, EquipmentAppearance, Player, Position, PowerEntry, PowerType,
};
use shared::death::DeathState;
use shared::protocol::{GroupMemberSnapshot, GroupMemberState, GroupRoleSnapshot};

use super::{PartyPortraits, party};
use crate::party_frames::{GroupViewer, group_frames_state};
use crate::ui::RegistryUi;
use crate::world::WorldUnits;
use crate::world_models::UnitAppearance;

#[derive(GodotClass)]
#[class(base=Node)]
struct PartyPortraitFixture {
    base: Base<Node>,
    ui: Option<Gd<RegistryUi>>,
    world: WorldUnits,
    portraits: PartyPortraits,
    group: GroupState,
    compact: bool,
    available: bool,
}

#[godot_api]
impl INode for PartyPortraitFixture {
    fn init(base: Base<Node>) -> Self {
        let path = ProjectSettings::singleton().globalize_path("res://../data");
        Self {
            base,
            ui: None,
            world: WorldUnits::new(PathBuf::from(path.to_string())),
            portraits: Default::default(),
            group: Default::default(),
            compact: true,
            available: true,
        }
    }

    fn exit_tree(&mut self) {
        self.portraits.clear(&mut self.world);
        self.world.reset();
    }
}

#[godot_api]
impl PartyPortraitFixture {
    #[func]
    fn initialize(&mut self, forever: bool) -> GString {
        let result = self.initialize_ui(forever);
        result.err().unwrap_or_default().into()
    }

    #[func]
    fn set_members(&mut self, names: Array<GString>, compact: bool, available: bool) -> GString {
        self.compact = compact;
        self.available = available;
        self.group = fixture_group(names.iter_shared().map(|name| name.to_string()), !available);
        let settings = LayoutSettings {
            use_raid_style_party_frames: Some(compact),
            ..Default::default()
        };
        game_engine_ui_model::hud_layout::set_active_layout_settings(settings);
        self.sync_fixture().err().unwrap_or_default().into()
    }

    #[func]
    fn tick(&mut self) -> GString {
        self.world.attach_loaded_visuals(&Replica::default());
        self.sync_fixture().err().unwrap_or_default().into()
    }

    #[func]
    fn portrait_state(&self, name: GString) -> VarDictionary {
        self.portraits.snapshot(&name.to_string())
    }

    #[func]
    fn ready_heads(&self) -> i64 {
        self.portraits
            .members
            .values()
            .filter(|member| {
                let state = member.portrait.snapshot();
                state
                    .get("model_shown")
                    .is_some_and(|value| value.to::<bool>())
                    && state
                        .get("mask_loaded")
                        .is_some_and(|value| value.to::<bool>())
            })
            .count() as i64
    }
}

impl PartyPortraitFixture {
    fn initialize_ui(&mut self, forever: bool) -> Result<(), String> {
        let path = ProjectSettings::singleton().globalize_path("res://../data");
        game_engine_ui_model::paths::set_data_root(PathBuf::from(path.to_string()))?;
        let skin = if forever {
            ui_toolkit::atlas::ActiveSkin::Forever
        } else {
            ui_toolkit::atlas::ActiveSkin::Modern
        };
        ui_toolkit::atlas::set_active_skin(skin);
        game_engine_ui_model::hud_layout::set_active_layout_settings(LayoutSettings::default());
        let mut ui = RegistryUi::new_alloc();
        self.base_mut().add_child(&ui);
        let state = group_frames_state(
            &self.group,
            &GroupViewer {
                local_name: Some("Bob"),
                target_name: None,
            },
            &|_| 0,
        );
        ui.bind_mut().show_group_frames(state)?;
        self.ui = Some(ui);
        Ok(())
    }

    fn sync_fixture(&mut self) -> Result<(), String> {
        let ui = self.ui.as_mut().ok_or("Fixture not initialized")?;
        let viewer = GroupViewer {
            local_name: Some("Bob"),
            target_name: None,
        };
        let state = group_frames_state(&self.group, &viewer, &|_| 0);
        ui.bind_mut().set_state(state)?;
        // Use the production roster selector and resource owner, not a test renderer.
        let bindings = party::bindings(&self.group, Some("Bob"), self.compact);
        let available = self.available;
        self.portraits
            .sync(&mut self.world, Some(ui), bindings, |name| {
                available.then(|| fixture_appearance(name))
            })
    }
}

fn fixture_group(names: impl Iterator<Item = String>, disconnected: bool) -> GroupState {
    let mut group = GroupState::default();
    for name in names {
        let offline = disconnected && name == "Valeera";
        let death = if name == "Uther" {
            DeathState::Dead
        } else {
            DeathState::Alive
        };
        let health = match name.as_str() {
            "Theron" => 300,
            "Jaina" => 200,
            "Valeera" => 100,
            _ => 0,
        };
        group.members.push(GroupMemberSnapshot {
            name: name.clone(),
            role: GroupRoleSnapshot::None,
            is_leader: name == "Jaina",
            online: !offline,
            subgroup: 1,
            class: 8,
            level: 10,
            entity: None,
        });
        if !offline {
            group.live.insert(
                name.clone(),
                GroupMemberState {
                    name,
                    health,
                    max_health: 400,
                    power: Some(PowerEntry {
                        power: PowerType::Mana,
                        current: 250,
                        max: 1000,
                        partial: 0,
                        regen_per_sec: 0.0,
                    }),
                    death,
                    position: Position {
                        x: 200.0,
                        y: 0.0,
                        z: 0.0,
                    },
                    debuffs: Vec::new(),
                },
            );
        }
    }
    group
}

fn fixture_appearance(name: &str) -> UnitAppearance {
    let sex = u8::from(matches!(name, "Jaina" | "Valeera"));
    let player = Player {
        name: name.into(),
        race: 1,
        class: 8,
        appearance: CharacterAppearance {
            sex,
            ..Default::default()
        },
    };
    UnitAppearance::Player(player, EquipmentAppearance::default())
}
