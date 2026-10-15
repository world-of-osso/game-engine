//! Offline UI audit fixtures. No account, server or world startup.

use game_engine_ui_model::quest_frame_component::{
    QuestFramePage, QuestFrameState, RewardItemView, RewardView, quest_frame_screen,
};
use game_engine_ui_model::quest_log_frame_component::{
    QuestDifficulty, QuestLogDetails, QuestLogFrameState, QuestLogGroup, QuestLogObjectiveLine,
    QuestLogRow,
};
use game_engine_ui_model::world_map_frame_component::WorldMapFrameState;
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

fn build_encounter_timer_fixture()
-> game_engine_ui_model::encounter_timeline_component::EncounterTimelineScreen {
    use game_engine_ui_model::encounter_timeline::EncounterTimeline;
    use game_engine_ui_model::encounter_timeline_component::EncounterTimelineScreen;
    use shared::protocol::{EncounterTimelineEvent, EncounterTimer};
    let timers = [0, 5_000, 10_000]
        .into_iter()
        .enumerate()
        .map(|(index, remaining_ms)| EncounterTimer {
            event_id: index as u32 + 1,
            spell_id: 20805,
            remaining_ms,
            duration_ms: 10_000,
            conditional: false,
        })
        .collect();
    let mut timeline = EncounterTimeline::default();
    timeline.receive(
        EncounterTimelineEvent::Timers {
            caster: 3669,
            sample_unix_ms: 0,
            timers,
        },
        0,
        |_| ("Lightning Bolt".into(), 136048),
    );
    EncounterTimelineScreen {
        timeline,
        enabled: true,
        bars: true,
        warnings: false,
        preview: false,
    }
}

fn build_cooldown_buff_fixture() -> game_engine_ui_model::cooldown_assist::CooldownViewerView {
    use game_engine_ui_model::cooldown_assist::{CooldownIconView, CooldownViewerView};
    let mut state = CooldownViewerView {
        enabled: true,
        ..Default::default()
    };
    state.categories[3] = [0.0, 0.5, 1.0]
        .into_iter()
        .enumerate()
        .map(|(index, fraction)| CooldownIconView {
            cooldown_id: 900 + index as u32,
            spell_id: 31884,
            name: "Avenging Wrath".into(),
            icon_fdid: 135875,
            fraction,
            remaining: fraction * 10.0,
            charges: Some(2),
            assisted_highlight: false,
        })
        .collect();
    state
}

fn build_cooldown_settings_fixture(
    category: usize,
) -> game_engine_ui_model::cooldown_assist::CooldownViewerView {
    use game_engine_ui_model::cooldown_settings_component::{
        CooldownSettingEntry, CooldownSettingsView,
    };
    game_engine_ui_model::cooldown_assist::CooldownViewerView {
        settings: CooldownSettingsView {
            open: true,
            category,
            portrait_fdid: 132355,
            entries: [
                (26, 0, "Colossus Smash", 2065633),
                (27, 0, "Mortal Strike", 132355),
                (68661, 3, "Avenging Wrath", 135875),
            ]
            .into_iter()
            .map(|(id, category, label, icon_fdid)| CooldownSettingEntry {
                id,
                category,
                label: label.into(),
                icon_fdid,
                selected: true,
            })
            .collect(),
            ..Default::default()
        },
        ..Default::default()
    }
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
    fn mount_collections(&mut self, forever: bool, journal: GString) -> GString {
        if let Err(error) = set_audit_data_and_skin(forever) {
            return error.as_str().into();
        }
        let size = self
            .base()
            .get_viewport()
            .expect("audit viewport")
            .get_visible_rect()
            .size;
        let viewport = [size.x, size.y];
        let mut ui = RegistryUi::new_alloc();
        ui.set_name("CollectionsAuditUI");
        self.base_mut().add_child(&ui);
        if let Err(error) = ui.bind_mut().set_ui_scale(1.0) {
            return error.as_str().into();
        }
        let result = match journal.to_string().as_str() {
            "mounts" => ui.bind_mut().show_mount_journal(
                game_engine_ui_model::mount_journal::MountJournalView {
                    visible: true,
                    viewport,
                    ..Default::default()
                },
            ),
            "pets" => {
                ui.bind_mut()
                    .show_pet_journal(game_engine_ui_model::pet_journal::PetJournalView {
                        visible: true,
                        viewport,
                        ..Default::default()
                    })
            }
            other => return format!("Unknown audit journal {other}").as_str().into(),
        };
        result.err().unwrap_or_default().as_str().into()
    }

    #[func]
    fn mount_encounter_timer_bars(&mut self, forever: bool) -> GString {
        if let Err(error) = set_audit_data_and_skin(forever) {
            return error.as_str().into();
        }
        let mut ui = RegistryUi::new_alloc();
        ui.set_name("EncounterTimerAuditUI");
        self.base_mut().add_child(&ui);
        if let Err(error) = ui.bind_mut().set_ui_scale(1.0) {
            return error.as_str().into();
        }
        let result = ui
            .bind_mut()
            .show_encounter_timeline(build_encounter_timer_fixture());
        result.err().unwrap_or_default().as_str().into()
    }

    #[func]
    fn mount_cooldown_buff_bars(&mut self, forever: bool) -> GString {
        if let Err(error) = set_audit_data_and_skin(forever) {
            return error.as_str().into();
        }
        let mut ui = RegistryUi::new_alloc();
        ui.set_name("CooldownBuffAuditUI");
        self.base_mut().add_child(&ui);
        if let Err(error) = ui.bind_mut().set_ui_scale(1.0) {
            return error.as_str().into();
        }
        let result = ui
            .bind_mut()
            .show_cooldown_viewer(build_cooldown_buff_fixture());
        result.err().unwrap_or_default().as_str().into()
    }

    #[func]
    fn mount_cooldown_settings(&mut self, forever: bool, auras: bool) -> GString {
        if let Err(error) = set_audit_data_and_skin(forever) {
            return error.as_str().into();
        }
        let mut ui = RegistryUi::new_alloc();
        ui.set_name("CooldownSettingsAuditUI");
        self.base_mut().add_child(&ui);
        if let Err(error) = ui.bind_mut().set_ui_scale(1.0) {
            return error.as_str().into();
        }
        let category = if auras { 2 } else { 0 };
        let result = ui
            .bind_mut()
            .show_cooldown_viewer(build_cooldown_settings_fixture(category));
        result.err().unwrap_or_default().as_str().into()
    }

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
            ui.bind_mut().show_world_map(
                WorldMapFrameState {
                    viewport: [1920.0, 1080.0],
                    ..Default::default()
                }
                .with_quest_panel(
                    QuestLogFrameState {
                        visible: true,
                        quest_count: 1,
                        max_quests: 35,
                        groups: vec![QuestLogGroup {
                            sort_id: 1519,
                            name: "Stormwind City".into(),
                            collapsed: false,
                            quests: vec![QuestLogRow {
                                quest_id: 332,
                                title: "Wine Shop Advert".into(),
                                level: 1,
                                difficulty: QuestDifficulty::Trivial,
                                complete: true,
                                watched: false,
                                selected: true,
                            }],
                        }],
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
                            can_share: false,
                        }),
                    },
                    true,
                ),
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
                description: None, rewards: None, watched: false, can_share: false,
            }),
            ..Default::default()
        };
        let mut ui = RegistryUi::new_alloc();
        ui.set_name("QuestAuditUI");
        self.base_mut().add_child(&ui);
        let result = ui.bind_mut().show_world_map(
            WorldMapFrameState {
                viewport: [1920.0, 1080.0],
                ..Default::default()
            }
            .with_quest_panel(state, true),
        );
        result.err().unwrap_or_default().as_str().into()
    }
}
