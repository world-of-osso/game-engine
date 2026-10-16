//! Supplied money/item rows through production projection/input; no loot/server mutation.
use super::{RegistryUi, party_preview};
use game_engine_ui_model::loot_frame_component::{LootFrameRow, LootFrameState};
use godot::prelude::*;
use ui_toolkit::atlas::{ActiveSkin, set_thread_skin};

#[godot_api(secondary)]
impl RegistryUi {
    #[func]
    fn show_loot_hit_preview(&mut self, forever: bool) -> GString {
        let result = (|| {
            party_preview::load_data_root()?;
            set_thread_skin(if forever {
                ActiveSkin::Forever
            } else {
                ActiveSkin::Modern
            });
            self.set_ui_scale(1.0)?;
            let state = LootFrameState {
                visible: true,
                left: 850.0,
                top: 430.0,
                rows: vec![
                    LootFrameRow {
                        slot: 5,
                        icon_fdid: 133788,
                        name: "3 Copper".into(),
                        color: "1,1,1,1",
                        quality_text: None,
                        count: 1,
                    },
                    LootFrameRow {
                        slot: 9,
                        icon_fdid: 132350,
                        name: "Sharp Shortsword".into(),
                        color: "0.12,1,0,1",
                        quality_text: Some("Uncommon"),
                        count: 2,
                    },
                ],
            };
            self.show_loot_frame(state)?;
            self.enable_cursor_inputs();
            Ok::<(), String>(())
        })();
        result.err().unwrap_or_default().as_str().into()
    }

    #[func]
    fn loot_hit_preview_actions(&mut self) -> Array<GString> {
        self.drain_bag_inputs()
            .expect("Loot hit-area preview cursor must be initialized")
            .into_iter()
            .filter_map(|(_, input)| match input {
                crate::bag_cursor::BagInput::Click { action, .. } => Some(action.as_str().into()),
                crate::bag_cursor::BagInput::Release { .. } => None,
            })
            .collect()
    }
}
