//! Supplied offline states for named-art consumers without ordinary preview entry points.
use super::{RegistryUi, party_preview};
use godot::prelude::*;
use ui_toolkit::atlas::{ActiveSkin, set_thread_skin};

#[godot_api(secondary)]
impl RegistryUi {
    #[func]
    fn show_template_art_preview(&mut self, forever: bool, panel: GString) -> GString {
        let result = party_preview::load_data_root().and_then(|()| {
            set_thread_skin(if forever {
                ActiveSkin::Forever
            } else {
                ActiveSkin::Modern
            });
            self.set_ui_scale(1.0)?;
            match panel.to_string().as_str() {
                "Chat" => self.show_chat_frame(chat_view()),
                "ToyBox" => self.show_toybox(game_engine_ui_model::toybox_component::ToyBoxView {
                    model: game_engine_ui_model::toybox::ToyBox {
                        open: true,
                        ..Default::default()
                    },
                    viewport: [1920.0, 1080.0],
                }),
                "Loading" => {
                    let error = self.show_loading().to_string();
                    if error.is_empty() { Ok(()) } else { Err(error) }
                }
                other => Err(format!("Unknown template art preview {other}")),
            }
        });
        result.err().unwrap_or_default().as_str().into()
    }

    #[func]
    fn show_template_guild_art(&mut self, forever: bool, panel: GString) -> GString {
        let error = self.show_guild_communities_preview(forever);
        if !error.is_empty() {
            return error;
        }
        let mut session = self
            .model
            .as_ref()
            .expect("mounted Communities fixture")
            .shared
            .get::<game_engine_ui_model::guild_ranks::GuildRanksSession>()
            .expect("Communities fixture session")
            .clone();
        match panel.to_string().as_str() {
            "GuildMember" => session.member_detail = Some("Member01".into()),
            "GuildLog" => session.log_open = true,
            "GuildNews" => session.news.filters_open = true,
            other => return format!("Unknown guild art preview {other}").as_str().into(),
        }
        self.set_state(session)
            .err()
            .unwrap_or_default()
            .as_str()
            .into()
    }
}

fn chat_view() -> game_engine_ui_model::chat_frame_component::ChatFrameView {
    use game_engine_ui_model::{
        chat_frame::{ChatRow, ChatRun},
        chat_frame_component::{ChatFrameView, ChatMessageView},
    };
    ChatFrameView {
        input_open: true,
        scrolled_up: true,
        messages: vec![ChatMessageView {
            timestamp: Some("12:34:56".into()),
            rows: vec![ChatRow {
                runs: vec![ChatRun {
                    text: "Northshire Valley".into(),
                    color: [1.0; 4],
                    x: 0.0,
                    width: 220.0,
                    spell_id: None,
                    achievement_id: None,
                }],
            }],
        }],
        ..Default::default()
    }
}
