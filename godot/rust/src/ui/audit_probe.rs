//! Offline UI audit fixtures. No account, server or world startup.

use game_engine_ui_model::quest_log_frame_component::{
    QuestLogDetails, QuestLogFrameState, QuestLogObjectiveLine, quest_log_frame_screen,
};
use godot::classes::Node;
use godot::prelude::*;
use ui_toolkit::atlas::{ActiveSkin, set_active_skin};

use super::RegistryUi;

fn set_audit_data_and_skin(forever: bool) -> Result<(), String> {
    let root = godot::classes::ProjectSettings::singleton().globalize_path("res://../data");
    game_engine_ui_model::paths::set_data_root(root.to_string().into())?;
    set_active_skin(if forever {
        ActiveSkin::Forever
    } else {
        ActiveSkin::Modern
    });
    Ok(())
}

#[derive(GodotClass)]
#[class(base = Node, init)]
struct UiAuditProbe {
    base: Base<Node>,
}

#[godot_api]
impl UiAuditProbe {
    #[func]
    fn mount_mail(&mut self, forever: bool) -> GString {
        use game_engine_ui_model::mail::NativeMailView;
        use game_engine_ui_model::mail_frame_component::{MailFrameState, MailFrameTab};
        if let Err(error) = set_audit_data_and_skin(forever) {
            return error.into();
        }
        let state = NativeMailView {
            frame: MailFrameState {
                visible: true,
                tab: MailFrameTab::Send,
                ..Default::default()
            },
            bags: Default::default(),
        };
        let mut ui = RegistryUi::new_alloc();
        ui.set_name("MailAuditUI");
        self.base_mut().add_child(&ui);
        let result = ui.bind_mut().show_mail(state);
        result.err().unwrap_or_default().into()
    }

    #[func]
    fn mount_quest(&mut self, forever: bool) -> GString {
        if let Err(error) = set_audit_data_and_skin(forever) {
            return error.into();
        }
        let state = QuestLogFrameState {
            visible: true,
            details: Some(QuestLogDetails {
                quest_id: 332,
                title: "Wine Shop Advert".into(),
                objectives_text: "Go to the Gallina Winery, and bring Suzetta Gallina the Wine Ticket for a free bottle of wine.".into(),
                objectives: vec![QuestLogObjectiveLine { text: "1/1 Wine Ticket".into(), done: true }],
                description: None, rewards: None, watched: false,
            }),
            ..Default::default()
        };
        let mut ui = RegistryUi::new_alloc();
        ui.set_name("QuestAuditUI");
        self.base_mut().add_child(&ui);
        let result = ui
            .bind_mut()
            .show_quest_window(state, quest_log_frame_screen);
        result.err().unwrap_or_default().into()
    }
}
