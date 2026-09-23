use bevy::prelude::*;
use game_engine::input_bindings::InputAction;
use game_engine::mail_data::MailState;
use game_engine::ui::plugin::{UiState, sync_registry_to_primary_window};
use game_engine::ui::screens::mail_frame_component::{
    InboxEntry, MailFrameState, MailTab, SendMailState, mail_frame_screen,
};
use ui_toolkit::screen::{Screen, SharedContext};

use crate::game::inworld_scene_stage::inworld_scene_stage_allows_ui;
use crate::game_state::GameState;
use crate::window_manager::{WindowId, WindowManager};

struct MailFrameRes {
    screen: Screen,
    shared: SharedContext,
}

unsafe impl Send for MailFrameRes {}
unsafe impl Sync for MailFrameRes {}

#[derive(Resource)]
struct MailFrameWrap(MailFrameRes);

#[derive(Resource, Clone, PartialEq)]
struct MailFrameModel(MailFrameState);

pub struct MailFramePlugin;

impl Plugin for MailFramePlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<MailState>();
        app.add_systems(
            OnEnter(GameState::InWorld),
            build_mail_frame_ui.run_if(inworld_scene_stage_allows_ui),
        );
        app.add_systems(OnExit(GameState::InWorld), teardown_mail_frame_ui);
        app.add_systems(
            Update,
            (toggle_mail_frame, sync_mail_frame_state)
                .run_if(in_state(GameState::InWorld))
                .run_if(inworld_scene_stage_allows_ui),
        );
    }
}

fn build_mail_frame_ui(
    mut ui: ResMut<UiState>,
    mut commands: Commands,
    windows: Query<&Window, With<bevy::window::PrimaryWindow>>,
    mail: Res<MailState>,
    window_manager: Res<WindowManager>,
) {
    sync_registry_to_primary_window(&mut ui.registry, &windows);
    let state = build_state(&mail, window_manager.is_open(WindowId::Mail));
    let mut shared = SharedContext::new();
    shared.insert(state.clone());
    let mut screen = Screen::new(mail_frame_screen);
    screen.sync(&shared, &mut ui.registry);
    commands.insert_resource(MailFrameWrap(MailFrameRes { screen, shared }));
    commands.insert_resource(MailFrameModel(state));
}

fn teardown_mail_frame_ui(
    mut ui: ResMut<UiState>,
    mut commands: Commands,
    mut wrap: Option<ResMut<MailFrameWrap>>,
) {
    if let Some(res) = wrap.as_mut() {
        res.0.screen.teardown(&mut ui.registry);
    }
    commands.remove_resource::<MailFrameWrap>();
    commands.remove_resource::<MailFrameModel>();
}

fn sync_mail_frame_state(
    mut ui: ResMut<UiState>,
    mut wrap: Option<ResMut<MailFrameWrap>>,
    mut last_model: Option<ResMut<MailFrameModel>>,
    mail: Res<MailState>,
    window_manager: Res<WindowManager>,
) {
    let (Some(mut wrap), Some(mut last_model)) = (wrap.take(), last_model.take()) else {
        return;
    };
    let state = build_state(&mail, window_manager.is_open(WindowId::Mail));
    if last_model.0 == state {
        return;
    }
    last_model.0 = state.clone();
    let res = &mut wrap.0;
    res.shared.insert(state);
    res.screen.sync(&res.shared, &mut ui.registry);
}

fn toggle_mail_frame(
    keybinds: crate::ui_input_mode::WorldKeybinds,
    mut window_manager: ResMut<WindowManager>,
) {
    if keybinds.just_pressed(InputAction::ToggleMail) {
        window_manager.toggle(WindowId::Mail);
    }
}

fn build_state(mail: &MailState, open: bool) -> MailFrameState {
    MailFrameState {
        visible: open,
        tabs: vec![
            MailTab {
                name: "Inbox".into(),
                active: true,
            },
            MailTab {
                name: "Send".into(),
                active: false,
            },
        ],
        inbox: mail
            .inbox
            .iter()
            .map(|message| InboxEntry {
                subject: message.subject.clone(),
                sender: message.sender.clone(),
                has_attachment: message.has_attachments(),
                read: message.read,
            })
            .collect(),
        send: SendMailState {
            recipient: mail.send_recipient.clone(),
            subject: mail.send_subject.clone(),
            body: mail.send_body.clone(),
            gold: mail.send_money.gold().to_string(),
            silver: mail.send_money.silver().to_string(),
            copper: mail.send_money.copper().to_string(),
        },
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use game_engine::auction_house_data::Money;
    use game_engine::mail_data::{MailAttachment, MailMessage};

    #[test]
    fn build_state_maps_runtime_mail_into_ui_state() {
        let mail = MailState {
            inbox: vec![MailMessage {
                id: 1,
                sender: "Jaina".into(),
                subject: "Supplies".into(),
                body: "Inside.".into(),
                money: Money::from_gold_silver_copper(1, 2, 3),
                attachments: vec![MailAttachment {
                    item_name: "Linen".into(),
                    icon_fdid: 123,
                    count: 2,
                }],
                read: false,
                expires_in: 3600.0,
            }],
            send_recipient: "Thrall".into(),
            send_subject: "Reply".into(),
            send_body: "See attached.".into(),
            send_money: Money::from_gold_silver_copper(4, 5, 6),
            send_attachments: vec![],
        };

        let state = build_state(&mail, true);

        assert!(state.visible);
        assert_eq!(state.inbox.len(), 1);
        assert_eq!(state.inbox[0].sender, "Jaina");
        assert!(state.inbox[0].has_attachment);
        assert_eq!(state.send.recipient, "Thrall");
        assert_eq!(state.send.gold, "4");
        assert_eq!(state.send.silver, "5");
        assert_eq!(state.send.copper, "6");
    }

    #[test]
    fn build_state_hides_frame_when_closed() {
        let state = build_state(&MailState::default(), false);
        assert!(!state.visible);
        assert_eq!(state.inbox, Vec::<InboxEntry>::new());
    }
}
