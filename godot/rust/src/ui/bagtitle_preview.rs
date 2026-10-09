//! Offline container title regression through the production screen and projection.
use super::{RegistryUi, party_preview};
use game_engine_ui_model::bag_frame_component::{
    BACKPACK_PORTRAIT, BagContainerState, BagFrameState, BagSlotState, bag_frame_screen,
};
use godot::prelude::*;
use ui_toolkit::atlas::{ActiveSkin, set_thread_skin};

#[godot_api(secondary)]
impl RegistryUi {
    #[func]
    fn show_bagtitle_preview(&mut self, forever: bool) -> GString {
        let result = party_preview::load_data_root().and_then(|()| {
            set_thread_skin(if forever {
                ActiveSkin::Forever
            } else {
                ActiveSkin::Modern
            });
            self.set_ui_scale(1.0)?;
            let bags = [
                "Backpack",
                "Combined Bags",
                "Explorer's Enormous Expedition Satchel",
            ]
            .into_iter()
            .enumerate()
            .map(|(bag_index, title)| BagContainerState {
                bag_index,
                title: title.into(),
                portrait_fdid: BACKPACK_PORTRAIT,
                slots: (0..16)
                    .map(|_| BagSlotState {
                        icon_fdid: 0,
                        count: 0,
                        quality_border: String::new(),
                        locked: false,
                        name: String::new(),
                    })
                    .collect(),
                visible: true,
            })
            .collect();
            self.show_quest_window(
                BagFrameState {
                    bags,
                    ..Default::default()
                },
                bag_frame_screen,
            )
        });
        GString::from(result.err().unwrap_or_default().as_str())
    }
}
