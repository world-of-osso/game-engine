//! Offline portrait party capture data; never connected to GroupState.
use godot::prelude::*;
use ui_toolkit::registry::FrameRegistry;
use ui_toolkit::screen::{Screen, SharedContext};

use super::{RegistryModel, RegistryUi, ScreenPostsetup, party_preview};
use std::path::PathBuf;

use game_engine_ui_model::portrait_party_frame_component::{
    PortraitPartyFrameState, PortraitPartyMemberView,
};
use godot::classes::ProjectSettings;
use godot::obj::Singleton;

pub(super) fn load_data_root() -> Result<(), String> {
    let path = ProjectSettings::singleton().globalize_path("res://../data");
    game_engine_ui_model::paths::set_data_root(PathBuf::from(path.to_string()))
}

pub(super) fn state() -> PortraitPartyFrameState {
    let members = ["Theron", "Jaina", "Valeera", "Uther"]
        .into_iter()
        .enumerate()
        .map(|(index, name)| PortraitPartyMemberView {
            health_fraction: [0.75, 0.5, 0.25, 0.0][index],
            power_fraction: [0.5, 0.25, 1.0, 0.0][index],
            class_rgb: [
                [0.96, 0.55, 0.73],
                [0.25, 0.78, 0.92],
                [1.0, 0.96, 0.41],
                [0.96, 0.55, 0.73],
            ][index],
            leader: index == 1,
            offline: index == 2,
            dead: index == 3,
            ..PortraitPartyMemberView::named(name)
        })
        .collect();
    PortraitPartyFrameState {
        members,
        show_pets: false,
    }
}

#[godot_api(secondary)]
impl RegistryUi {
    /// Offline authored party preview for capture_ui_screen.gd; no group/network state.
    #[func]
    pub fn show_portrait_party(&mut self) -> GString {
        self.show_portrait_party_skin(ui_toolkit::atlas::ActiveSkin::Modern)
    }

    #[func]
    pub fn show_forever_portrait_party(&mut self) -> GString {
        self.show_portrait_party_skin(ui_toolkit::atlas::ActiveSkin::Forever)
    }

    fn show_portrait_party_skin(&mut self, skin: ui_toolkit::atlas::ActiveSkin) -> GString {
        use game_engine_ui_model::portrait_party_frame_component::portrait_party_frame_screen;
        if self.model.is_some() {
            return "RegistryUi already has a screen".into();
        }
        let Some(viewport) = self.base().get_viewport() else {
            return "RegistryUi has no viewport".into();
        };
        let size = viewport.get_visible_rect().size;
        if let Err(error) = party_preview::load_data_root() {
            return GString::from(error.as_str());
        }
        ui_toolkit::atlas::set_thread_skin(skin);
        let mut shared = SharedContext::new();
        shared.insert(party_preview::state());
        let mut model = RegistryModel {
            screen: Screen::new(portrait_party_frame_screen),
            shared,
            registry: FrameRegistry::new(size.x, size.y),
            icon_masks: Default::default(),
            postsetup: ScreenPostsetup::PortraitParty,
        };
        model.sync();
        GString::from(
            self.initialize_model(model, size.x, size.y)
                .err()
                .unwrap_or_default()
                .as_str(),
        )
    }
}
