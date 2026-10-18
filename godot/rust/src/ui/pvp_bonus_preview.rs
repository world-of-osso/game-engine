//! Serverless supplied-state BonusFrame input proof; never sends network commands.
use super::{RegistryUi, party_preview};
use game_engine_ui_model::battleground::{PvpAction, PvpView};
use godot::prelude::*;
use shared::protocol::PvpSnapshot;
use ui_toolkit::atlas::{ActiveSkin, set_thread_skin};

fn supplied_owner() -> PvpSnapshot {
    PvpSnapshot {
        lifetime_honor_xp: 0,
        honor_level: None,
        honor: 0,
        honor_max: 15000,
        conquest: 0,
        conquest_max: None,
        brackets: Vec::new(),
        queue: None,
        battleground: None,
        war_mode: None,
        talents: None,
    }
}

#[godot_api(secondary)]
impl RegistryUi {
    #[func]
    fn show_pvp_bonus_preview(&mut self, forever: bool) -> GString {
        let result = party_preview::load_data_root().and_then(|()| {
            set_thread_skin(if forever {
                ActiveSkin::Forever
            } else {
                ActiveSkin::Modern
            });
            self.set_ui_scale(1.0)?;
            let size = self
                .base()
                .get_viewport()
                .ok_or("Bonus preview has no viewport")?
                .get_visible_rect()
                .size;
            self.show_pvp(PvpView {
                visible: true,
                viewport: [size.x, size.y],
                selected_battleground: 1018,
                honor_faction: Some(game_engine_ui_model::char_create_data::Faction::Alliance),
                snapshot: Some(supplied_owner()),
                ..Default::default()
            })
        });
        result.err().unwrap_or_default().as_str().into()
    }

    /// Apply only the actual observed native action through the production reducer.
    #[func]
    fn apply_pvp_bonus_preview_action(&mut self, action: GString) -> VarDictionary {
        let result = (|| {
            let mut view = self
                .model
                .as_ref()
                .ok_or("Bonus preview missing")?
                .shared
                .get::<PvpView>()
                .ok_or("Bonus preview view missing")?
                .clone();
            let request = view.action(&action.to_string());
            let mut receipt = VarDictionary::new();
            receipt.set("error", "");
            receipt.set("queue_id", &Variant::nil());
            receipt.set("specific_id", view.selected_battleground);
            receipt.set("bonus", view.honor_bonus);
            receipt.set("pending", view.pending);
            if let Some(PvpAction::Queue(id)) = request {
                receipt.set("queue_id", id);
            }
            self.set_state(view)?;
            Ok::<_, String>(receipt)
        })();
        match result {
            Ok(receipt) => receipt,
            Err(error) => {
                let mut receipt = VarDictionary::new();
                receipt.set("error", error.as_str());
                receipt
            }
        }
    }
}
