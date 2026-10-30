//! Offline physical-input fixture: production tabard screen/reducer, model worker and masked NPC.
//! Replies are simulated; independent server tests establish persistence/payment authority.
use super::Portrait;
use crate::{
    character_frame::preview::{Scene, bind_sheet_light},
    character_select::CharacterPreview,
    guild_tabards::{dress_draft_preview, held_button},
    ui::RegistryUi,
    world::WorldUnits,
    world_models::UnitAppearance,
};
use game_engine_network::replica::Replica;
use game_engine_ui_model::{
    char_select_data::CharSelectNames,
    char_select_state_from_roster, guild_tabard_frame,
    guild_tabards::{GuildTabards, calculate_rotation},
};
use godot::{
    classes::{Node, Node3D, ProjectSettings},
    prelude::*,
};
use shared::{
    components::{EquipmentAppearance, Player},
    guild_creation::GuildTabardDesign,
    protocol::{CharacterListEntry, GuildTabardReply, GuildTabardVendorSnapshot},
};
use std::path::PathBuf;

const NPC: u64 = 701;
const SAVED: GuildTabardDesign = GuildTabardDesign {
    icon: 1,
    icon_color: 2,
    border: 3,
    border_color: 4,
    background_color: 5,
};

#[derive(GodotClass)]
#[class(base=Node)]
struct GuildTabardFixture {
    base: Base<Node>,
    world: WorldUnits,
    view: GuildTabards,
    ui: Option<Gd<RegistryUi>>,
    communities: Option<Gd<RegistryUi>>,
    portrait: Portrait,
    scene: Option<Scene>,
    pending: Option<(u64, UnitAppearance)>,
    shown: Option<UnitAppearance>,
    tier: usize,
    yaw: f32,
    character_select: Option<CharacterPreview>,
    character_stage: Option<Gd<Node3D>>,
}

#[godot_api]
impl INode for GuildTabardFixture {
    fn init(base: Base<Node>) -> Self {
        let data = ProjectSettings::singleton().globalize_path("res://../data");
        Self {
            base,
            world: WorldUnits::new(PathBuf::from(data.to_string())),
            view: GuildTabards::default(),
            ui: None,
            communities: None,
            portrait: Portrait::new(guild_tabard_frame::PORTRAIT),
            scene: None,
            pending: None,
            shown: None,
            tier: 0,
            yaw: 0.0,
            character_select: None,
            character_stage: None,
        }
    }
    fn exit_tree(&mut self) {
        self.clear_vendor();
        if let Some(preview) = &mut self.character_select {
            preview.reset();
        }
        self.world.reset();
    }
}

#[godot_api]
impl GuildTabardFixture {
    #[func]
    fn initialize(&mut self, forever: bool) -> GString {
        report(self.mount_vendor(forever))
    }
    #[func]
    fn tick(&mut self, delta: f64) -> GString {
        if self.character_select.is_some() {
            return report(self.update_character_selection());
        }
        report(self.update_models(delta as f32))
    }
    #[func]
    fn show_character_selection(&mut self, forever: bool) -> GString {
        report(self.mount_character_selection(forever))
    }
    #[func]
    fn dispatch(&mut self) -> GString {
        match self.dispatch_actions() {
            Ok(receipt) => receipt.as_str().into(),
            Err(error) => format!("ERROR: {error}").as_str().into(),
        }
    }
    #[func]
    fn reply(&mut self, success: bool) -> GString {
        let reply = if success {
            GuildTabardReply::Saved { design: SAVED }
        } else {
            GuildTabardReply::Failed {
                message: "You can't afford to do that.".into(),
            }
        };
        report(self.view.receive(reply).and_then(|()| self.sync_ui()))
    }
    #[func]
    fn restore_saved_design(&mut self) -> GString {
        self.view.draft = Some(SAVED);
        report(self.sync_ui())
    }
    #[func]
    fn set_tier(&mut self, tier: i64) -> GString {
        let Ok(tier @ 0..=2) = usize::try_from(tier) else {
            return "Invalid source tabard tier".into();
        };
        self.tier = tier;
        GString::new()
    }
    #[func]
    fn receipt(&self) -> VarDictionary {
        let mut receipt = VarDictionary::new();
        receipt.set("draft", design_text(self.view.draft));
        receipt.set(
            "registered",
            design_text(self.view.vendor.as_ref().and_then(|v| v.design)),
        );
        receipt.set("waiting", self.view.waiting);
        receipt.set("yaw", f64::from(self.yaw));
        receipt.set(
            "character_select_ready",
            self.character_select
                .as_ref()
                .is_some_and(CharacterPreview::is_ready),
        );
        receipt.set(
            "model_ready",
            self.pending.is_none() && self.shown == self.appearance(),
        );
        receipt
    }
    #[func]
    fn portrait_state(&self) -> VarDictionary {
        self.portrait.snapshot()
    }
    #[func]
    fn show_communities(&mut self) -> GString {
        report(self.mount_communities())
    }
}

impl GuildTabardFixture {
    fn mount_character_selection(&mut self, forever: bool) -> Result<(), String> {
        let data = ProjectSettings::singleton().globalize_path("res://../data");
        let data = PathBuf::from(data.to_string());
        game_engine_ui_model::paths::set_data_root(data.clone())?;
        ui_toolkit::atlas::set_thread_skin(if forever {
            ui_toolkit::atlas::ActiveSkin::Forever
        } else {
            ui_toolkit::atlas::ActiveSkin::Modern
        });
        self.mount_roster_ui(&data)?;
        let mut stage = Node3D::new_alloc();
        stage.set_name("CharacterSelectStage");
        self.base_mut().add_child(&stage);
        self.character_stage = Some(stage);
        self.character_select = Some(CharacterPreview::new(data));
        Ok(())
    }
    fn mount_roster_ui(&mut self, data: &std::path::Path) -> Result<(), String> {
        let entry = registered_roster_entry();
        let names = CharSelectNames::load(data)?;
        let state = char_select_state_from_roster(std::slice::from_ref(&entry), Some(0), &names)?;
        let mut ui = RegistryUi::new_alloc();
        ui.set_name("CharacterSelectUI");
        self.base_mut().add_child(&ui);
        ui.bind_mut().set_ui_scale(1.0)?;
        let error = ui.bind_mut().show_character_select();
        if !error.is_empty() {
            return Err(error.to_string());
        }
        ui.bind_mut().set_state(state)?;
        self.ui = Some(ui);
        Ok(())
    }
    fn update_character_selection(&mut self) -> Result<(), String> {
        let preview = self
            .character_select
            .as_mut()
            .ok_or("Missing character preview")?;
        let stage = self
            .character_stage
            .as_mut()
            .ok_or("Missing character stage")?;
        preview.sync(
            stage,
            Some(&registered_roster_entry()),
            720.0,
            glam::Vec2::ZERO,
        )
    }
    fn mount_vendor(&mut self, forever: bool) -> Result<(), String> {
        self.clear_vendor();
        let mut ui = RegistryUi::new_alloc();
        ui.set_name("GuildTabardUI");
        self.base_mut().add_child(&ui);
        let data = ProjectSettings::singleton().globalize_path("res://../data");
        game_engine_ui_model::paths::set_data_root(PathBuf::from(data.to_string()))?;
        ui_toolkit::atlas::set_thread_skin(if forever {
            ui_toolkit::atlas::ActiveSkin::Forever
        } else {
            ui_toolkit::atlas::ActiveSkin::Modern
        });
        ui.bind_mut().set_ui_scale(1.0)?;
        self.view.gold = Some(250000);
        self.view.begin_open(NPC);
        self.view
            .receive(GuildTabardReply::Vendor(GuildTabardVendorSnapshot {
                npc: NPC,
                npc_name: "Jondor Steelbrow".into(),
                own_rank: Some(0),
                design: None,
                cost: 100000,
            }))?;
        let result = ui.bind_mut().show_guild_tabard(self.view.clone());
        self.ui = Some(ui);
        self.yaw = 0.0;
        self.tier = 0;
        result
    }
    fn clear_vendor(&mut self) {
        self.portrait.clear(&mut self.world);
        if let Some((id, _)) = self.pending.take() {
            self.world.cancel_detached_visual(id);
        }
        if let Some(scene) = self.scene.take() {
            scene.free();
        }
        if let Some(ui) = self.ui.take() {
            ui.free();
        }
        self.shown = None;
    }
    fn sync_ui(&mut self) -> Result<(), String> {
        if let Some(ui) = &mut self.ui {
            ui.bind_mut().set_state(self.view.clone())?;
        }
        Ok(())
    }
    fn dispatch_actions(&mut self) -> Result<String, String> {
        let Some(mut ui) = self.ui.clone() else {
            return Ok(String::new());
        };
        let mut receipt = String::new();
        loop {
            let action = ui.bind_mut().pop_action().to_string();
            if action.is_empty() {
                break;
            }
            if action == "tabard:close" {
                self.view.close();
                self.clear_vendor();
                return Ok("Close".into());
            }
            if let Some(request) = self.view.click(&action)? {
                receipt = format!("{request:?}");
            }
        }
        self.sync_ui()?;
        Ok(receipt)
    }
    fn appearance(&self) -> Option<UnitAppearance> {
        let mut appearance = dress_draft_preview(
            Player {
                name: "TabardFixture".into(),
                race: 1,
                class: 2,
                appearance: Default::default(),
            },
            EquipmentAppearance::default(),
            self.view.draft?,
        );
        let UnitAppearance::Player(_, equipment, _) = &mut appearance else {
            unreachable!()
        };
        equipment
            .entries
            .last_mut()
            .expect("preview tabard")
            .item_id = Some([5976, 69209, 69210][self.tier]);
        Some(appearance)
    }
    fn update_models(&mut self, delta: f32) -> Result<(), String> {
        if self.view.vendor.is_none() {
            return Ok(());
        }
        self.world.attach_loaded_visuals(&Replica::default());
        self.sync_portrait()?;
        self.request_model();
        self.attach_model()?;
        let ui = self.ui.as_ref().ok_or("Missing tabard fixture UI")?;
        self.yaw = calculate_rotation(
            self.yaw,
            delta,
            held_button(ui, "TabardCharacterModelRotateLeftButton")?,
            held_button(ui, "TabardCharacterModelRotateRightButton")?,
        );
        if let Some(scene) = &mut self.scene {
            scene.fit_viewport();
            if let Some(mut model) = scene.model() {
                model.set_rotation(Vector3::new(0.0, self.yaw, 0.0));
            }
        }
        Ok(())
    }
    fn sync_portrait(&mut self) -> Result<(), String> {
        let host = self
            .ui
            .as_ref()
            .and_then(|ui| ui.bind().frame_control(guild_tabard_frame::PORTRAIT.frame));
        // Canonical creature_template_model and tdb_creature_template_model:5130 ->3081.
        let npc = UnitAppearance::Creature {
            display_id: 3081,
            items: EquipmentAppearance::default(),
        };
        self.portrait.sync(&mut self.world, host, Some(npc))
    }
    fn request_model(&mut self) {
        let Some(appearance) = self.appearance() else {
            return;
        };
        let current = self
            .pending
            .as_ref()
            .map(|(_, appearance)| appearance)
            .or(self.shown.as_ref());
        if current == Some(&appearance) {
            return;
        }
        if let Some((id, _)) = self.pending.take() {
            self.world.cancel_detached_visual(id);
        }
        let id = self.world.request_detached_visual(&appearance);
        self.pending = Some((id, appearance));
    }
    fn attach_model(&mut self) -> Result<(), String> {
        let Some((id, _)) = self.pending else {
            return Ok(());
        };
        let Some(loaded) = self.world.take_detached_visual(id) else {
            return Ok(());
        };
        let (_, appearance) = self.pending.take().expect("pending tabard fixture");
        let model = loaded?;
        bind_sheet_light(&model);
        if self.scene.is_none() {
            let host = self
                .ui
                .as_ref()
                .ok_or("Missing tabard UI")?
                .bind()
                .frame_control(guild_tabard_frame::MODEL)
                .ok_or("Missing TabardModel")?;
            self.scene = Some(Scene::new(host));
        }
        self.scene
            .as_mut()
            .expect("scene mounted")
            .replace_model(model);
        self.shown = Some(appearance);
        Ok(())
    }
    fn mount_communities(&mut self) -> Result<(), String> {
        if let Some(ui) = self.communities.take() {
            ui.free();
        }
        let mut ui = RegistryUi::new_alloc();
        ui.set_name("CommunitiesUI");
        self.base_mut().add_child(&ui);
        let forever = ui_toolkit::atlas::thread_skin() == ui_toolkit::atlas::ActiveSkin::Forever;
        let error = ui.bind_mut().show_guild_communities_preview(forever);
        if !error.is_empty() {
            return Err(error.to_string());
        }
        let design = self
            .view
            .vendor
            .as_ref()
            .and_then(|v| v.design)
            .ok_or("Missing saved design")?;
        ui.bind_mut().update_guild_communities_design(design)?;
        self.communities = Some(ui);
        Ok(())
    }
}

fn registered_roster_entry() -> CharacterListEntry {
    let appearance = dress_draft_preview(
        Player {
            name: "Guildmaster".into(),
            race: 1,
            class: 2,
            appearance: Default::default(),
        },
        EquipmentAppearance::default(),
        SAVED,
    );
    let UnitAppearance::Player(player, equipment_appearance, tabard) = appearance else {
        unreachable!("player preview must produce a player appearance")
    };
    CharacterListEntry {
        character_id: 7,
        name: "Guildmaster".into(),
        level: 80,
        race: player.race,
        class: player.class,
        appearance: player.appearance,
        equipment_appearance,
        tabard,
    }
}

fn design_text(design: Option<GuildTabardDesign>) -> String {
    design.map_or_else(
        || "none".into(),
        |d| {
            format!(
                "{},{},{},{},{}",
                d.icon, d.icon_color, d.border, d.border_color, d.background_color
            )
        },
    )
}
fn report(result: Result<(), String>) -> GString {
    result.err().unwrap_or_default().as_str().into()
}
