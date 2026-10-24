//! Supplied offline HouseSettings draft through production screen/reducer/projection.
use super::{RegistryUi, party_preview};
use game_engine_ui_housing::{
    housing::{HousingAuthority, HousingDashboard, HousingSurface},
    housing_component::HousingView,
    housing_settings::HouseSettings,
};
use godot::prelude::*;
use std::sync::Arc;
use ui_toolkit::atlas::{ActiveSkin, set_thread_skin};

#[godot_api(secondary)]
impl RegistryUi {
    #[func]
    fn show_house_access_preview(&mut self, forever: bool, pending: bool) -> GString {
        let result = party_preview::load_data_root().and_then(|()| {
            set_thread_skin(if forever {
                ActiveSkin::Forever
            } else {
                ActiveSkin::Modern
            });
            self.set_ui_scale(1.0)?;
            let mut model = HousingDashboard::new(Arc::new(Default::default()));
            model.surface = HousingSurface::HouseSettings;
            model.settings = Some(HouseSettings::new(41, Some(Default::default())));
            model.pending = pending;
            self.show_housing(HousingView {
                model,
                authority: HousingAuthority::default(),
                viewport: [1920.0, 1080.0],
                money: None,
                missing_thumbnails: Default::default(),
                vendor_names: Default::default(),
            })
        });
        result.err().unwrap_or_default().as_str().into()
    }

    /// Mirror the live housing cursor consumer; releases are not click actions.
    #[func]
    fn house_access_preview_actions(&mut self) -> Array<GString> {
        self.drain_bag_inputs()
            .expect("HouseSettings cursor input must be initialized")
            .into_iter()
            .filter_map(|(_, input)| match input {
                crate::bag_cursor::BagInput::Click { action, .. } => Some(action.as_str().into()),
                crate::bag_cursor::BagInput::Release { .. } => None,
            })
            .collect()
    }

    /// Consume the observed pointer action with the real staging reducer, not a server.
    #[func]
    fn apply_house_access_preview_action(&mut self, action: GString) -> GString {
        let result = (|| {
            let mut view = self
                .model
                .as_ref()
                .ok_or("HouseSettings preview missing")?
                .shared
                .get::<HousingView>()
                .ok_or("HouseSettings view missing")?
                .clone();
            let command = view
                .model
                .settings_request(&view.authority, &action.to_string())?;
            if command.is_some() {
                return Err("HouseSettings preview does not send server commands".into());
            }
            self.set_state(view)
        })();
        result.err().unwrap_or_default().as_str().into()
    }
}
