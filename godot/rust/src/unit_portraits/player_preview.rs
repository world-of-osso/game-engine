//! Offline player portrait fixtures using the production asynchronous owner and real HD model.
use game_engine_network::replica::Replica;
use game_engine_ui_model::character_frame::{
    CharacterFrameView, MODE_TAB_PORTRAIT, PORTRAIT as CHARACTER_FRAME_PORTRAIT,
};
use game_engine_ui_model::inworld_unit_frames_component::{InWorldUnitFramesState, UnitFrameState};
use godot::classes::{Control, Node, ProjectSettings};
use godot::prelude::*;
use shared::components::{CharacterAppearance, EquipmentAppearance, Player};

use super::{PLAYER_PORTRAIT, Portrait};
use crate::{ui::RegistryUi, world::WorldUnits, world_models::UnitAppearance};

#[derive(GodotClass)]
#[class(base=Node)]
struct PlayerPortraitFixture {
    base: Base<Node>,
    ui: Option<Gd<RegistryUi>>,
    world: WorldUnits,
    player_frame_portrait: Portrait,
    character_frame_portrait: Portrait,
    character_tab_portrait: Portrait,
    character_frame_mode: bool,
}

#[godot_api]
impl INode for PlayerPortraitFixture {
    fn init(base: Base<Node>) -> Self {
        let path = ProjectSettings::singleton().globalize_path("res://../data");
        Self {
            base,
            ui: None,
            world: WorldUnits::new(std::path::PathBuf::from(path.to_string())),
            player_frame_portrait: Portrait::new(PLAYER_PORTRAIT),
            character_frame_portrait: Portrait::new(CHARACTER_FRAME_PORTRAIT),
            character_tab_portrait: Portrait::new(MODE_TAB_PORTRAIT),
            character_frame_mode: false,
        }
    }

    fn exit_tree(&mut self) {
        self.player_frame_portrait.clear(&mut self.world);
        self.character_frame_portrait.clear(&mut self.world);
        self.character_tab_portrait.clear(&mut self.world);
        self.world.reset();
    }
}

#[godot_api]
impl PlayerPortraitFixture {
    #[func]
    fn initialize(&mut self, forever: bool) -> GString {
        self.character_frame_mode = false;
        GString::from(
            self.mount_ui(forever, false)
                .err()
                .unwrap_or_default()
                .as_str(),
        )
    }

    #[func]
    fn initialize_character_tab(&mut self, forever: bool) -> GString {
        self.character_frame_mode = true;
        GString::from(
            self.mount_ui(forever, true)
                .err()
                .unwrap_or_default()
                .as_str(),
        )
    }

    #[func]
    fn tick(&mut self) -> GString {
        self.world.attach_loaded_visuals(&Replica::default());
        let result = if self.character_frame_mode {
            self.sync_character_frame_portraits()
        } else {
            self.sync_player_frame_portrait()
        };
        GString::from(result.err().unwrap_or_default().as_str())
    }

    #[func]
    fn portrait_state(&self) -> VarDictionary {
        if self.character_frame_mode {
            let mut state = self.character_tab_portrait.snapshot();
            state.set(
                "character_frame_model_ready",
                self.character_frame_portrait.model_ready(),
            );
            state.set(
                "character_tab_model_ready",
                self.character_tab_portrait.model_ready(),
            );
            state
        } else {
            self.player_frame_portrait.snapshot()
        }
    }

    #[func]
    fn portrait_model_ready(&self) -> bool {
        if self.character_frame_mode {
            self.character_frame_portrait.model_ready() && self.character_tab_portrait.model_ready()
        } else {
            self.player_frame_portrait.model_ready()
        }
    }

    /// Diagnostic control only, never used to satisfy the normal readiness oracle.
    #[func]
    fn redraw(&mut self) {
        if self.character_frame_mode {
            render_portrait(&mut self.character_frame_portrait);
            render_portrait(&mut self.character_tab_portrait);
        } else {
            render_portrait(&mut self.player_frame_portrait);
        }
    }
}

impl PlayerPortraitFixture {
    fn sync_player_frame_portrait(&mut self) -> Result<(), String> {
        let host = self.frame_host(self.player_frame_portrait.slot.frame);
        self.player_frame_portrait
            .sync(&mut self.world, host, Some(player_appearance()))
    }

    fn sync_character_frame_portraits(&mut self) -> Result<(), String> {
        let frame_host = self.frame_host(self.character_frame_portrait.slot.frame);
        let tab_host = self.frame_host(self.character_tab_portrait.slot.frame);
        self.character_frame_portrait.sync(
            &mut self.world,
            frame_host,
            Some(player_appearance()),
        )?;
        self.character_tab_portrait
            .sync(&mut self.world, tab_host, Some(player_appearance()))
    }

    fn frame_host(&self, frame: &'static str) -> Option<Gd<Control>> {
        self.ui
            .as_ref()
            .and_then(|ui| ui.bind().frame_control(frame))
    }

    fn mount_ui(&mut self, forever: bool, character_frame: bool) -> Result<(), String> {
        let path = ProjectSettings::singleton().globalize_path("res://../data");
        game_engine_ui_model::paths::set_data_root(std::path::PathBuf::from(path.to_string()))?;
        let skin = if forever {
            ui_toolkit::atlas::ActiveSkin::Forever
        } else {
            ui_toolkit::atlas::ActiveSkin::Modern
        };
        ui_toolkit::atlas::set_thread_skin(skin);
        let mut ui = RegistryUi::new_alloc();
        self.base_mut().add_child(&ui);
        if character_frame {
            ui.bind_mut().set_ui_scale(1.0)?;
            ui.bind_mut().show_character_frame(CharacterFrameView {
                visible: true,
                ..Default::default()
            })?;
        } else {
            ui.bind_mut().show_unit_frames(player_frame())?;
        }
        self.ui = Some(ui);
        Ok(())
    }
}

fn player_appearance() -> UnitAppearance {
    let player = Player {
        name: "Buffcancel".into(),
        race: 1,
        class: 8,
        appearance: CharacterAppearance::default(),
    };
    UnitAppearance::Player((&player).into(), EquipmentAppearance::default(), None)
}

fn render_portrait(portrait: &mut Portrait) {
    if let Some(scene) = portrait.scene.as_mut() {
        scene.render_once();
    }
}

fn player_frame() -> InWorldUnitFramesState {
    let mut player = UnitFrameState::named("Buffcancel");
    player.health_fraction = 1.0;
    InWorldUnitFramesState {
        show_player_frame: true,
        show_target_frame: false,
        target_cast: None,
        player,
        target: None,
        target_of_target: None,
        focus: None,
        pet: None,
        bosses: vec![],
        menu: Default::default(),
        personal_resource: None,
    }
}
