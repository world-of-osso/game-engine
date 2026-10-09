//! Offline window captures through production hosts and the masked portrait renderer.
use super::Portrait;
use crate::{ui::RegistryUi, world::WorldUnits, world_models::UnitAppearance};
use game_engine_network::replica::Replica;
use game_engine_ui_model::{
    auction::{AuctionGossipView, AuctionSession},
    auction_house_frame_component::PORTRAIT as AUCTION,
    bank_frame_component::{BankFrameState, PORTRAIT as BANK},
    guild_bank::NativeGuildBankView,
    guild_bank_frame_component::GuildBankFrameState,
    mail::NativeMailView,
    mail_frame_component::{MailFrameState, OpenMailView},
    merchant_frame_component::{MerchantFrameState, PORTRAIT as MERCHANT},
    trade::NativeTradeView,
    trade_frame_component::{PORTRAIT as TRADE, TradeFrameState},
};
use godot::{
    classes::{Node, ProjectSettings},
    prelude::*,
};
use shared::components::{EquipmentAppearance, Player};
use std::path::PathBuf;

#[derive(GodotClass)]
#[class(base=Node)]
struct NpcPortraitPreview {
    base: Base<Node>,
    world: WorldUnits,
    ui: Option<Gd<RegistryUi>>,
    portrait: Option<Portrait>,
    appearance: Option<UnitAppearance>,
}

#[godot_api]
impl INode for NpcPortraitPreview {
    fn init(base: Base<Node>) -> Self {
        let path = ProjectSettings::singleton().globalize_path("res://../data");
        Self {
            base,
            world: WorldUnits::new(PathBuf::from(path.to_string())),
            ui: None,
            portrait: None,
            appearance: None,
        }
    }
    fn exit_tree(&mut self) {
        if let Some(portrait) = &mut self.portrait {
            portrait.clear(&mut self.world);
        }
        self.world.reset();
    }
}

// Primary registration storage is required for the secondary preview API.
#[godot_api]
impl NpcPortraitPreview {}

#[godot_api(secondary)]
impl NpcPortraitPreview {
    #[func]
    fn initialize(&mut self, frame: GString, forever: bool) -> GString {
        self.mount_window(&frame.to_string(), forever)
            .err()
            .unwrap_or_default()
            .as_str()
            .into()
    }
    /// Offline Auctioneer Fitch uses the real creature portrait renderer. World DB
    /// creature_template_model / tdb_creature_template_model:8719 -> display7992.
    #[func]
    fn bind_auction_preview(&mut self, ui: Gd<RegistryUi>) {
        self.portrait = Some(Portrait::new(AUCTION));
        self.appearance = Some(UnitAppearance::Creature {
            display_id: 7992,
            items: EquipmentAppearance::default(),
        });
        self.ui = Some(ui);
    }

    #[func]
    fn tick(&mut self) -> GString {
        self.world.attach_loaded_visuals(&Replica::default());
        self.sync_portrait()
            .err()
            .unwrap_or_default()
            .as_str()
            .into()
    }
    #[func]
    fn portrait_state(&self) -> VarDictionary {
        self.portrait
            .as_ref()
            .map(Portrait::snapshot)
            .unwrap_or_default()
    }
}

/// Reuse the real masked portrait renderer on the trainer's existing offline canvas.
pub(crate) fn attach_trainer_preview(ui: Gd<RegistryUi>) -> Result<(), String> {
    let mut preview = NpcPortraitPreview::new_alloc();
    preview.set_name("TrainerPreviewPortrait");
    ui.clone()
        .upcast::<godot::classes::Node>()
        .add_child(&preview);
    let mut host = preview.bind_mut();
    host.portrait = Some(Portrait::new(game_engine_ui_model::trainer_frame::PORTRAIT));
    host.appearance = Some(UnitAppearance::Player(
        Player {
            name: "Offline human trainer".into(),
            race: 1,
            class: 1,
            appearance: Default::default(),
        },
        EquipmentAppearance::default(),
    ));
    host.ui = Some(ui);
    // The caller still holds RegistryUi's mutable Godot binding. First tick runs after it returns.
    Ok(())
}

impl NpcPortraitPreview {
    fn mount_window(&mut self, frame: &str, forever: bool) -> Result<(), String> {
        if self.ui.is_some() {
            return Err("Preview already mounted".into());
        }
        let path = ProjectSettings::singleton().globalize_path("res://../data");
        game_engine_ui_model::paths::set_data_root(PathBuf::from(path.to_string()))?;
        ui_toolkit::atlas::set_thread_skin(if forever {
            ui_toolkit::atlas::ActiveSkin::Forever
        } else {
            ui_toolkit::atlas::ActiveSkin::Modern
        });
        let mut ui = RegistryUi::new_alloc();
        self.base_mut().add_child(&ui);
        ui.bind_mut().set_ui_scale(1.0)?;
        self.show_window(&mut ui, frame)?;
        let slot = match frame {
            "bank" => Some(BANK),
            "auction" | "auction-gossip" => Some(AUCTION),
            "trade" => Some(TRADE),
            "merchant" => Some(MERCHANT),
            _ => None,
        };
        if let Some(slot) = slot {
            self.portrait = Some(Portrait::new(slot));
            self.appearance = Some(UnitAppearance::Player(
                Player {
                    name: if frame == "trade" {
                        "Local player"
                    } else {
                        "Offline NPC"
                    }
                    .into(),
                    race: if frame == "trade" { 10 } else { 1 },
                    class: 1,
                    appearance: Default::default(),
                },
                EquipmentAppearance::default(),
            ));
        }
        self.ui = Some(ui);
        self.sync_portrait()
    }

    fn show_window(&self, ui: &mut Gd<RegistryUi>, frame: &str) -> Result<(), String> {
        let mut host = ui.bind_mut();
        match frame {
            "bank" => host.show_bank(BankFrameState {
                visible: true,
                title: "Bank".into(),
                header: "Bank storage".into(),
                slots: vec![None; 98],
                ..Default::default()
            }),
            "mail" | "open-mail" => host.show_mail(NativeMailView {
                frame: MailFrameState {
                    visible: true,
                    open: (frame == "open-mail").then(|| OpenMailView {
                        sender: "Auction House".into(),
                        subject: "Linen Cloth".into(),
                        body: "Your item is enclosed.".into(),
                        ..Default::default()
                    }),
                    ..Default::default()
                },
                bags: Default::default(),
            }),
            "guild-bank" => host.show_guild_bank(NativeGuildBankView {
                frame: GuildBankFrameState {
                    visible: true,
                    title: "Guild Bank".into(),
                    slots: vec![None; 98],
                    ..Default::default()
                },
                bags: Default::default(),
            }),
            "trade" => host.show_trade(NativeTradeView {
                frame: TradeFrameState {
                    visible: true,
                    player_name: "Local player".into(),
                    recipient_name: "Recipient".into(),
                    ..Default::default()
                },
            }),
            "merchant" => host.show_merchant(crate::ui::MerchantStates {
                frame: MerchantFrameState {
                    visible: true,
                    title: "Offline NPC".into(),
                    ..Default::default()
                },
                bags: Default::default(),
                split: Default::default(),
            }),
            "auction" => {
                let mut session = AuctionSession::default();
                session.open(42);
                session.net.is_open = true;
                host.show_auction(session.native_view(&Default::default()))
            }
            "auction-gossip" => host.show_auction_gossip(AuctionGossipView {
                text: "Welcome to the auction house.".into(),
                options: Vec::new(),
            }),
            _ => Err(format!("Unknown portrait window {frame}")),
        }
    }

    fn sync_portrait(&mut self) -> Result<(), String> {
        let Some(portrait) = &mut self.portrait else {
            return Ok(());
        };
        let host = self
            .ui
            .as_ref()
            .and_then(|ui| ui.bind().frame_control(portrait.slot.frame));
        portrait.sync(&mut self.world, host, self.appearance.clone())
    }
}
