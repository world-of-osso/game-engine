//! Offline player slot using the production asynchronous portrait owner and real HD model.
use game_engine_network::replica::Replica;
use game_engine_ui_model::inworld_unit_frames_component::{InWorldUnitFramesState, UnitFrameState};
use godot::classes::{Node, ProjectSettings};
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
    portrait: Portrait,
}

#[godot_api]
impl INode for PlayerPortraitFixture {
    fn init(base: Base<Node>) -> Self {
        let path = ProjectSettings::singleton().globalize_path("res://../data");
        Self {
            base,
            ui: None,
            world: WorldUnits::new(std::path::PathBuf::from(path.to_string())),
            portrait: Portrait::new(PLAYER_PORTRAIT),
        }
    }

    fn exit_tree(&mut self) {
        self.portrait.clear(&mut self.world);
        self.world.reset();
    }
}

#[godot_api]
impl PlayerPortraitFixture {
    #[func]
    fn initialize(&mut self) -> GString {
        GString::from(self.mount_player().err().unwrap_or_default().as_str())
    }

    #[func]
    fn tick(&mut self) -> GString {
        self.world.attach_loaded_visuals(&Replica::default());
        let host = self
            .ui
            .as_ref()
            .and_then(|ui| ui.bind().frame_control(PLAYER_PORTRAIT.frame));
        let player = Player {
            name: "Buffcancel".into(),
            race: 1,
            class: 8,
            appearance: CharacterAppearance::default(),
        };
        let appearance = UnitAppearance::Player(player, EquipmentAppearance::default());
        GString::from(
            self.portrait
                .sync(&mut self.world, host, Some(appearance))
                .err()
                .unwrap_or_default()
                .as_str(),
        )
    }

    #[func]
    fn portrait_state(&self) -> VarDictionary {
        self.portrait.snapshot()
    }

    /// Diagnostic control only, never used to satisfy the normal readiness oracle.
    #[func]
    fn redraw(&mut self) {
        if let Some(scene) = self.portrait.scene.as_mut() {
            scene.render_once();
        }
    }
}

impl PlayerPortraitFixture {
    fn mount_player(&mut self) -> Result<(), String> {
        let path = ProjectSettings::singleton().globalize_path("res://../data");
        game_engine_ui_model::paths::set_data_root(std::path::PathBuf::from(path.to_string()))?;
        ui_toolkit::atlas::set_thread_skin(ui_toolkit::atlas::ActiveSkin::Modern);
        let mut ui = RegistryUi::new_alloc();
        self.base_mut().add_child(&ui);
        ui.bind_mut().show_unit_frames(player_frame())?;
        self.ui = Some(ui);
        Ok(())
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
