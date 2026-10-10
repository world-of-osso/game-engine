//! Offline native pointer regression/capture, not a live-server acceptance claim.
use super::{RegistryUi, party_preview};
use game_engine_ui_model::toybox::ToyBox;
use game_engine_ui_model::toybox_component::ToyBoxView;
use godot::prelude::*;
use shared::protocol::{SpellCooldownUpdate, ToySnapshot};
use ui_toolkit::atlas::{ActiveSkin, set_thread_skin};

#[godot_api(secondary)]
impl RegistryUi {
    #[func]
    fn show_toybox_pointer_fixture(&mut self, forever: bool) -> GString {
        let result = party_preview::load_data_root().and_then(|()| {
            set_thread_skin(if forever {
                ActiveSkin::Forever
            } else {
                ActiveSkin::Modern
            });
            self.set_ui_scale(1.0)?;
            // Authentic toy/world.db row99, item32782; ownership/favourite/cooldown
            // are staged fixture state, never evidence that a server committed them.
            let toy = ToySnapshot {
                item_id: 32782,
                name: "Time-Lost Figurine".into(),
                icon_file_data_id: 134911,
                expansion_id: 1,
                flags: 0,
                source_type: 0,
                source_text: "|cFFFFD200Drop: |rTerokk|n|cFFFFD200Zone: |rTerokkar Forest".into(),
                spell_id: Some(41301),
                learned: true,
                favourite: true,
                unavailable_reason: None,
            };
            let mut model = ToyBox::default();
            model.receive(vec![toy]);
            model.cooldowns.apply(&SpellCooldownUpdate {
                spell_id: 41301,
                category: 0,
                duration_ms: 10000,
                remaining_ms: 5000,
                is_gcd: false,
            });
            self.show_toybox(ToyBoxView {
                model,
                viewport: [1920.0, 1080.0],
            })?;
            self.update_toy_swipe("ToySpellButton1Cooldown", 0.5)
        });
        GString::from(result.err().unwrap_or_default().as_str())
    }

    #[func]
    fn assert_toybox_pointer_fixture(&mut self) -> GString {
        let result = self.drain_bag_inputs().and_then(|inputs| {
            let clicks: Vec<_> = inputs
                .into_iter()
                .filter_map(|(_, input)| {
                    let crate::bag_cursor::BagInput::Click {
                        action, click, at, ..
                    } = input
                    else {
                        return None;
                    };
                    Some((action, click.right, at.is_some()))
                })
                .collect();
            let expected = vec![
                ("toy_use:32782".into(), false, true),
                ("toy_use:32782".into(), true, false),
            ];
            if clicks == expected {
                Ok(())
            } else {
                Err(format!("Toy Box native pointer: {clicks:?}"))
            }
        });
        GString::from(result.err().unwrap_or_default().as_str())
    }
}
