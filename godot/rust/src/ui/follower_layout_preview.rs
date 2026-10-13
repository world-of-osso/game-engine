//! Offline layout proof of the same native follower screen.
use super::RegistryUi;
use game_engine_ui_model::follower_dungeons::View;
use godot::prelude::*;
use shared::protocol::{FollowerDungeonEntry, FollowerDungeons};
use ui_toolkit::atlas::{ActiveSkin, set_thread_skin};

#[godot_api(secondary)]
impl RegistryUi {
    #[func]
    fn show_follower_layout_preview(&mut self, forever: bool, type_open: bool) -> GString {
        let result = super::party_preview::load_data_root().and_then(|()| {
            set_thread_skin(if forever {
                ActiveSkin::Forever
            } else {
                ActiveSkin::Modern
            });
            let mut view = View {
                visible: true,
                type_open,
                ..Default::default()
            };
            view.receive_catalog(FollowerDungeons {
                entries: vec![FollowerDungeonEntry {
                    id: 2436,
                    name: "Ruby Life Pools".into(),
                    map_id: 2521,
                }],
            });
            view.selected = Some(2436);
            self.set_ui_scale(1.0)?;
            self.show_follower_dungeons(view)
        });
        GString::from(result.err().unwrap_or_default().as_str())
    }
}
