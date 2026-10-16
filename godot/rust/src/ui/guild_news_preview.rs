//! Read-only observation of actual authenticated news, never a history injector.
use super::RegistryUi;
use game_engine_ui_model::guild_ranks::GuildRanksSession;
use godot::prelude::*;

#[godot_api(secondary)]
impl RegistryUi {
    #[func]
    pub fn guild_news_observation(&self) -> GString {
        let Some(session) = self
            .model
            .as_ref()
            .and_then(|model| model.shared.get::<GuildRanksSession>())
        else {
            return GString::new();
        };
        let news = &session.news;
        let json = serde_json::to_string(&serde_json::json!({
            "waiting": news.waiting,
            "reply": news.state,
            "visible_ids": news.entries().iter().map(|entry| entry.id).collect::<Vec<_>>(),
            "pending_sticky": news.pending_sticky,
            "menu": news.menu,
        }))
        .expect("serialize real guild news receipt");
        GString::from(json.as_str())
    }
}
