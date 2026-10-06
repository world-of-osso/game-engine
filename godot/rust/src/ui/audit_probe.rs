//! Offline UI audit fixtures. No account, server or world startup.

use game_engine_ui_model::quest_frame_component::{
    QuestFramePage, QuestFrameState, RewardItemView, RewardView, quest_frame_screen,
};
use game_engine_ui_model::quest_log_frame_component::{
    QuestLogDetails, QuestLogFrameState, QuestLogObjectiveLine, quest_log_frame_screen,
};
use godot::classes::Node;
use godot::prelude::*;
use ui_toolkit::atlas::{ActiveSkin, set_thread_skin};

use super::RegistryUi;

fn set_audit_data_and_skin(forever: bool) -> Result<(), String> {
    let root = godot::classes::ProjectSettings::singleton().globalize_path("res://../data");
    game_engine_ui_model::paths::set_data_root(root.to_string().into())?;
    set_thread_skin(if forever {
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
impl INode for UiAuditProbe {
    fn input(&mut self, event: Gd<godot::classes::InputEvent>) {
        let Some(ui) = self.base().get_node_or_null("QuestOverflowUI") else {
            return;
        };
        let mut ui = ui.cast::<RegistryUi>();
        if let Err(error) = ui.bind_mut().scroll_list_input(&event) {
            godot_error!("Quest overflow input: {error}");
        }
    }
}

#[godot_api]
impl UiAuditProbe {
    #[func]
    fn mount_mail(&mut self, forever: bool) -> GString {
        use game_engine_ui_model::mail::NativeMailView;
        use game_engine_ui_model::mail_frame_component::{MailFrameState, MailFrameTab};
        if let Err(error) = set_audit_data_and_skin(forever) {
            return error.as_str().into();
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
        result.err().unwrap_or_default().as_str().into()
    }

    #[func]
    fn mount_quest_overflow(&mut self, forever: bool, page: GString, long: bool) -> GString {
        if let Err(error) = set_audit_data_and_skin(forever) {
            return error.as_str().into();
        }
        let text = if long {
            "Visit the winery and tell your friends about our wine. Bring the ticket to Suzetta Gallina for a free bottle.\n\n".repeat(50)
        } else {
            "Visit the winery.".into()
        };
        let rewards = RewardView {
            money: 55,
            items: vec![
                RewardItemView {
                    name: "Winery Boots".into(),
                    count: 1,
                    icon_fdid: None,
                },
                RewardItemView {
                    name: "Winery Gloves".into(),
                    count: 1,
                    icon_fdid: None,
                },
            ],
            ..Default::default()
        };
        let mut ui = RegistryUi::new_alloc();
        ui.set_name("QuestOverflowUI");
        self.base_mut().add_child(&ui);
        let result = if page.to_string() == "log" {
            ui.bind_mut().show_quest_window(
                QuestLogFrameState {
                    visible: true,
                    details: Some(QuestLogDetails {
                        quest_id: 332,
                        title: "Wine Shop Advert".into(),
                        objectives_text: "Bring Suzetta Gallina the Wine Ticket.".into(),
                        objectives: vec![QuestLogObjectiveLine {
                            text: "1/1 Wine Ticket".into(),
                            done: true,
                        }],
                        description: Some(text),
                        rewards: Some(rewards),
                        watched: false,
                    }),
                    ..Default::default()
                },
                quest_log_frame_screen,
            )
        } else {
            let title = "Wine Shop Advert".into();
            let page = match page.to_string().as_str() {
                "detail" => QuestFramePage::Detail {
                    title,
                    description: text,
                    objectives_text: "Bring the Wine Ticket.".into(),
                    rewards,
                },
                "progress" => QuestFramePage::Progress {
                    title,
                    text,
                    required: rewards.items,
                    can_complete: true,
                },
                "reward" => QuestFramePage::Reward {
                    title,
                    text,
                    rewards,
                },
                other => return format!("Unknown quest page {other}").as_str().into(),
            };
            ui.bind_mut().show_quest_window(
                QuestFrameState {
                    visible: true,
                    npc_name: "Suzetta Gallina".into(),
                    page,
                },
                quest_frame_screen,
            )
        };
        result.err().unwrap_or_default().as_str().into()
    }

    #[func]
    fn quest_scroll_metrics(&self, list: GString) -> Vector3 {
        let ui = self.base().get_node_as::<RegistryUi>("QuestOverflowUI");
        let ui = ui.bind();
        let model = ui.model.as_ref().unwrap();
        let list = list.to_string();
        let state = model.registry.scroll_lists.get(&list).unwrap();
        let child = game_engine_ui_model::quest_scroll::quest_scroll_child(&list);
        let child = model.registry.get_by_name(&child).unwrap();
        let height = model
            .registry
            .get(child)
            .unwrap()
            .layout_rect
            .as_ref()
            .unwrap()
            .height;
        Vector3::new(
            state.first_row as f32,
            state.geometry.max_first_row() as f32,
            height,
        )
    }

    #[func]
    fn mount_quest(&mut self, forever: bool) -> GString {
        if let Err(error) = set_audit_data_and_skin(forever) {
            return error.as_str().into();
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
        result.err().unwrap_or_default().as_str().into()
    }
}
