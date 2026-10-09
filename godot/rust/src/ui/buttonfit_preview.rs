//! Debug-only offline cases supplementing the opt-in button-fit inventory.
use super::{MerchantStates, RegistryUi, party_preview};
use game_engine_ui_model::{
    mail::NativeMailView, mail_frame_component::*, merchant_frame_component::*,
};
use godot::prelude::*;
use ui_toolkit::atlas::{ActiveSkin, set_thread_skin};

#[godot_api(secondary)]
impl RegistryUi {
    #[func]
    fn buttonfit_reskin(&mut self, forever: bool) -> GString {
        let skin = if forever {
            ActiveSkin::Forever
        } else {
            ActiveSkin::Modern
        };
        set_thread_skin(skin);
        self.model
            .as_mut()
            .expect("mounted offline screen")
            .sync_skin(skin);
        self.sync_model().err().unwrap_or_default().as_str().into()
    }

    #[func]
    fn show_buttonfit_auction_preview(&mut self, forever: bool) -> GString {
        let result = party_preview::load_data_root().and_then(|()| {
            set_thread_skin(if forever {
                ActiveSkin::Forever
            } else {
                ActiveSkin::Modern
            });
            self.set_ui_scale(1.0)?;
            let mut view = game_engine_ui_model::auction::preview::preview_view("browse")?;
            view.search_page = 1;
            view.search_pages = 3;
            self.show_auction(view)
        });
        result.err().unwrap_or_default().as_str().into()
    }

    #[func]
    fn show_buttonfit_service_preview(&mut self, case: GString, forever: bool) -> GString {
        let result = party_preview::load_data_root().and_then(|()| {
            set_thread_skin(if forever {
                ActiveSkin::Forever
            } else {
                ActiveSkin::Modern
            });
            self.set_ui_scale(1.0)?;
            self.mount_buttonfit_service(&case.to_string())
        });
        result.err().unwrap_or_default().as_str().into()
    }
}

fn merchant_fixture(case: &str) -> MerchantStates {
    MerchantStates {
        frame: MerchantFrameState {
            visible: true,
            title: "Vendor".into(),
            buyback_tab: case == "buyback",
            page_text: Some("Page 2 of 3".into()),
            prev_enabled: true,
            next_enabled: true,
            repair: Some(true),
            ..Default::default()
        },
        bags: Default::default(),
        split: Default::default(),
    }
}

fn mail_fixture(case: &str) -> NativeMailView {
    NativeMailView {
        frame: MailFrameState {
            visible: true,
            tab: if case == "send" {
                MailFrameTab::Send
            } else {
                MailFrameTab::Inbox
            },
            opening_all: case == "opening",
            open: (case == "openmail").then(|| OpenMailView {
                sender: "Auction House".into(),
                subject: "Linen Cloth".into(),
                body: "Your won item is enclosed.".into(),
                can_delete: true,
                ..Default::default()
            }),
            ..Default::default()
        },
        bags: Default::default(),
    }
}

fn menu_fixture(
    category: Option<game_engine_ui_model::options_menu_component::OptionsCategory>,
) -> game_engine_ui_model::game_menu_component::GameMenuViewModel {
    use game_engine_ui_model::{game_menu_component::*, options_menu_component::OptionsViewModel};
    let mut options = OptionsViewModel::default();
    if let Some(category) = category {
        options.category = category;
    }
    GameMenuViewModel {
        logged_in: true,
        view: if category.is_some() {
            GameMenuView::Options
        } else {
            GameMenuView::MainMenu
        },
        options,
    }
}

impl RegistryUi {
    fn mount_buttonfit_service(&mut self, case: &str) -> Result<(), String> {
        match case {
            "merchant" | "buyback" => self.show_merchant(merchant_fixture(case)),
            "inbox" | "send" | "openmail" | "opening" => self.show_mail(mail_fixture(case)),
            "trade" => self.show_trade(game_engine_ui_model::trade::NativeTradeView {
                frame: game_engine_ui_model::trade_frame_component::TradeFrameState {
                    visible: true,
                    player_name: "Theron".into(),
                    recipient_name: "Jaina".into(),
                    ..Default::default()
                },
            }),
            "guildbank" => {
                self.show_guild_bank(game_engine_ui_model::guild_bank::NativeGuildBankView {
                    frame: game_engine_ui_model::guild_bank_frame_component::GuildBankFrameState {
                        visible: true,
                        title: "Guild Vault".into(),
                        ..Default::default()
                    },
                    bags: Default::default(),
                })
            }
            "character" | "reputation" => self.show_character_frame(
                game_engine_ui_model::character_frame::CharacterFrameView {
                    visible: true,
                    title: "Theron".into(),
                    race_id: 1,
                    class_id: 2,
                    tab: if case == "reputation" {
                        game_engine_ui_model::character_frame::CharacterTab::Reputation
                    } else {
                        game_engine_ui_model::character_frame::CharacterTab::PaperDoll
                    },
                    ..Default::default()
                },
            ),
            "menu" => self.show_game_menu_view(menu_fixture(None)),
            key if key.starts_with("options:") => {
                let category = game_engine_ui_model::options_menu_component::OptionsCategory::ALL
                    .into_iter()
                    .find(|category| Some(category.key()) == key.strip_prefix("options:"))
                    .ok_or_else(|| format!("Unknown options audit case {key}"))?;
                self.show_game_menu_view(menu_fixture(Some(category)))
            }
            other => Err(format!("Unknown buttonfit service {other}")),
        }
    }
}
