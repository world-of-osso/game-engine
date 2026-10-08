//! Offline BankFrame capture using the production bank projection; no server or account.
use super::{RegistryUi, party_preview};
use game_engine_ui_model::bank_art::SlotItem;
use game_engine_ui_model::bank_frame_component::{BankFrameState, PurchasePromptView, SideTab};
use godot::prelude::*;
use ui_toolkit::atlas::{ActiveSkin, set_thread_skin};

fn preview_state(purchase: bool) -> BankFrameState {
    BankFrameState {
        visible: true,
        title: "Bank".into(),
        header: "Cloth and ore".into(),
        tabs: vec![
            SideTab {
                icon_fdid: 133784,
                selected: !purchase,
            },
            SideTab {
                icon_fdid: 134400,
                selected: false,
            },
        ],
        purchase_tab: Some(purchase),
        slots: (0..98)
            .map(|index| {
                (index == 0).then_some(SlotItem {
                    icon_fdid: 133784,
                    count: 20,
                    quality_border: "1.0,1.0,1.0,1.0".into(),
                })
            })
            .collect(),
        purchase: purchase.then_some(PurchasePromptView {
            title: "Purchase a Bank Tab".into(),
            text: "Purchase additional storage for your character.".into(),
            cost: 5_000_000,
            can_afford: false,
        }),
        deposit_all_label: "Deposit All Reagents".into(),
        ..Default::default()
    }
}

#[godot_api(secondary)]
impl RegistryUi {
    #[func]
    fn show_bank_preview(&mut self) -> GString {
        self.show_bank_preview_skin(ActiveSkin::Modern)
    }
    #[func]
    fn show_forever_bank_preview(&mut self) -> GString {
        self.show_bank_preview_skin(ActiveSkin::Forever)
    }
    /// Capture fixture checks the production pointer boundary without sending requests.
    #[func]
    fn assert_bank_preview_clicks(&mut self) -> GString {
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
                    Some((action, click.right, click.shift, at.is_some()))
                })
                .collect();
            let expected = vec![
                ("bank_slot:0".to_string(), false, false, true),
                ("bank_slot:0".to_string(), false, true, false),
                ("bank_slot:0".to_string(), true, false, false),
            ];
            if clicks == expected {
                Ok(())
            } else {
                Err(format!("Bank preview pointer clicks: {clicks:?}"))
            }
        });
        GString::from(result.err().unwrap_or_default().as_str())
    }

    fn show_bank_preview_skin(&mut self, skin: ActiveSkin) -> GString {
        let result = party_preview::load_data_root().and_then(|()| {
            set_thread_skin(skin);
            self.set_ui_scale(1.0)?;
            let purchase = std::env::var("GODOT_BANK_PURCHASE").as_deref() == Ok("1");
            self.show_bank(preview_state(purchase))
        });
        GString::from(result.err().unwrap_or_default().as_str())
    }
}
