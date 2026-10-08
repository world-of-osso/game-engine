//! Offline process fixture for res://tests/party_portraits.gd; no client or server.
use std::path::PathBuf;

use game_engine_core::ui_layout_data::LayoutSettings;
use game_engine_network::replica::Replica;
use game_engine_ui_model::group_state::GroupState;
use godot::classes::{Node, ProjectSettings};
use godot::prelude::*;
use shared::components::{
    CharacterAppearance, EquipmentVisualSlot, EquippedAppearanceEntry, Position, PowerEntry,
    PowerType,
};
use shared::death::DeathState;
use shared::protocol::{
    GroupMemberSnapshot, GroupMemberState, GroupPortraitAppearance, GroupRoleSnapshot,
};

use super::{PartyPortraits, party};
#[path = "party_settings.rs"]
mod settings_preview;
use crate::party_frames::{GroupViewer, group_frames_state};
use crate::ui::RegistryUi;
use crate::world::WorldUnits;
use game_engine_ui_model::options_menu_component::{LayoutOptionsView, LayoutSystem};

#[derive(GodotClass)]
#[class(base=Node)]
struct PartyPortraitFixture {
    base: Base<Node>,
    ui: Option<Gd<RegistryUi>>,
    world: WorldUnits,
    portraits: PartyPortraits,
    group: GroupState,
    compact: bool,
    layout: LayoutOptionsView,
    settings_ui: Option<Gd<RegistryUi>>,
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
            layout: LayoutOptionsView {
                system: LayoutSystem::PartyFrames,
                ..Default::default()
            },
            settings_ui: None,
        }
    }

    fn input(&mut self, event: Gd<godot::classes::InputEvent>) {
        if let Some(ui) = self.settings_ui.as_mut() {
            if let Err(error) = ui.bind_mut().scroll_list_input(&event) {
                godot_error!("Party settings fixture input: {error}");
            }
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
        fixture_error(self.initialize_ui(forever))
    }

    #[func]
    fn set_members(&mut self, names: Array<GString>, compact: bool, available: bool) -> GString {
        self.compact = compact;
        self.group = fixture_group(names.iter_shared().map(|name| name.to_string()), !available);
        let settings = LayoutSettings {
            use_raid_style_party_frames: Some(compact),
            ..Default::default()
        };
        self.layout.settings = settings;
        game_engine_ui_model::hud_layout::set_active_layout_settings(settings);
        fixture_error(self.sync_fixture())
    }

    /// Inject the protocol update through the same group HUD projection as live play.
    #[func]
    fn apply_ready_check(&mut self, finished: bool) -> GString {
        use shared::protocol::{ReadyCheckAnswer, ReadyCheckMemberSnapshot, ReadyCheckUpdate};
        self.group.apply_ready_check(ReadyCheckUpdate {
            initiator_name: "Ann".into(),
            time_remaining_secs: if finished { 0.0 } else { 30.0 },
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
            finished,
        });
        fixture_error(self.sync_fixture())
    }

    #[func]
    fn show_settings(&mut self, shown: bool) -> GString {
        if !shown {
            if let Some(ui) = self.settings_ui.take() {
                ui.free();
            }
            return GString::new();
        }
        if self.settings_ui.is_some() {
            return GString::new();
        }
        let mut ui = RegistryUi::new_alloc();
        self.base_mut().add_child(&ui);
        let result = ui
            .bind_mut()
            .show_game_menu_view(settings_preview::options_view(self.layout.clone()));
        self.settings_ui = Some(ui);
        self.base_mut().set_process_input(true);
        fixture_error(result)
    }

    #[func]
    fn settings_action(&mut self, action: GString, value: f64) -> GString {
        use game_engine_ui_model::options_menu_data as policy;
        let action = action.to_string();
        if let Some(action) = policy::parse_layout_action(&action) {
            policy::apply_layout_action(action, &mut self.layout);
        } else if let Some(policy::SliderField::Layout(slider)) =
            policy::parse_slider_action(&action)
        {
            policy::apply_layout_slider(slider, value as f32, &mut self.layout);
        } else {
            return format!("Not a party layout action: {action}")
                .as_str()
                .into();
        }
        self.compact = self
            .layout
            .settings
            .use_raid_style_party_frames
            .unwrap_or(true);
        game_engine_ui_model::hud_layout::set_active_layout_settings(self.layout.settings);
        if let Some(ui) = self.ui.as_mut() {
            if let Err(error) = ui.bind_mut().sync_skin() {
                return error.as_str().into();
            }
        }
        if let Some(ui) = self.settings_ui.as_mut() {
            let result = ui
                .bind_mut()
                .set_game_menu_view(settings_preview::options_view(self.layout.clone()));
            if let Err(error) = result {
                return error.as_str().into();
            }
        }
        fixture_error(self.sync_fixture())
    }

    #[func]
    fn settings_rect(&self, name: GString) -> Rect2 {
        self.settings_ui
            .as_ref()
            .and_then(|ui| ui.bind().frame_rect(&name.to_string()))
            .map(|(rect, _)| {
                Rect2::new(
                    Vector2::new(rect[0], rect[1]),
                    Vector2::new(rect[2], rect[3]),
                )
            })
            .unwrap_or_default()
    }

    #[func]
    fn member_rect(&self, name: GString) -> Rect2 {
        self.ui
            .as_ref()
            .and_then(|ui| ui.bind().frame_rect(&name.to_string()))
            .map(|(rect, _)| {
                Rect2::new(
                    Vector2::new(rect[0], rect[1]),
                    Vector2::new(rect[2], rect[3]),
                )
            })
            .unwrap_or_default()
    }

    #[func]
    fn tick(&mut self) -> GString {
        self.world.attach_loaded_visuals(&Replica::default());
        fixture_error(self.sync_fixture())
    }

    #[func]
    fn portrait_state(&self, name: GString) -> VarDictionary {
        self.portraits.snapshot(&name.to_string())
    }

    #[func]
    fn set_head(&mut self, name: GString, equipped: bool) -> GString {
        let Some(member) = self
            .group
            .members
            .iter_mut()
            .find(|member| member.name == name.to_string())
        else {
            return "Unknown roster member".into();
        };
        member.portrait.head = equipped.then_some(EquippedAppearanceEntry {
            slot: EquipmentVisualSlot::Head,
            item_id: None,
            display_info_id: Some(14903),
            inventory_type: 1,
            hidden: false,
        });
        fixture_error(self.sync_fixture())
    }

    #[func]
    fn set_offline(&mut self, name: GString) -> GString {
        let name = name.to_string();
        let Some(member) = self
            .group
            .members
            .iter_mut()
            .find(|member| member.name == name)
        else {
            return "Unknown roster member".into();
        };
        member.online = false;
        self.group.live.remove(&name);
        fixture_error(self.sync_fixture())
    }

    #[func]
    fn model_id(&self, name: GString) -> i64 {
        self.portraits
            .members
            .get(&name.to_string())
            .and_then(|member| member.portrait.scene.as_ref())
            .and_then(|scene| scene.model.as_ref())
            .map_or(0, |model| model.instance_id().to_i64())
    }

    #[func]
    fn ready_heads(&self) -> i64 {
        self.portraits
            .members
            .values()
            .filter(|member| {
                let state = member.portrait.snapshot();
                !state.get("pending").is_some_and(|value| value.to::<bool>())
                    && state
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
        ui_toolkit::atlas::set_thread_skin(skin);
        self.layout.active = if forever { "Forever" } else { "Modern" }.into();
        self.layout.skin = if forever {
            game_engine_core::ui_layout_data::LayoutSkin::Forever
        } else {
            game_engine_core::ui_layout_data::LayoutSkin::Modern
        };
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
        let bindings = party::bindings_with_sort(
            &self.group,
            Some("Bob"),
            self.compact,
            self.layout.settings.party.sort.unwrap_or_default(),
        );
        let appearances = party::roster_appearances(&self.group);
        self.portraits
            .sync(&mut self.world, Some(ui), bindings, |name| {
                appearances.get(name).cloned()
            })
    }
}

fn fixture_error(result: Result<(), String>) -> GString {
    GString::from(result.err().unwrap_or_default().as_str())
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
            character_id: 7,
            name: name.clone(),
            role: GroupRoleSnapshot::None,
            is_leader: name == "Jaina",
            online: !offline,
            subgroup: 1,
            class: 8,
            level: 10,
            entity: None,
            portrait: fixture_appearance(&name),
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

fn fixture_appearance(name: &str) -> GroupPortraitAppearance {
    let sex = u8::from(matches!(name, "Jaina" | "Valeera"));
    GroupPortraitAppearance {
        race: if matches!(name, "Theron" | "Valeera") {
            10
        } else {
            1
        },
        head: None,
        appearance: CharacterAppearance {
            sex,
            ..Default::default()
        },
    }
}
