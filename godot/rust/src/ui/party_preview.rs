//! Offline portrait party capture data; never connected to GroupState.
use std::path::PathBuf;

use game_engine_ui_model::portrait_party_frame_component::{
    PortraitPartyFrameState, PortraitPartyMemberView,
};
use godot::classes::ProjectSettings;

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
