use std::sync::mpsc;

use bevy::ecs::system::RunSystemOnce;
use game_engine::mail_data::MailDraft;
use game_engine::network_runtime::messages::{ConnectionSender, Inbox};
use game_engine::network_runtime::worker::NetworkCommand;
use shared::protocol::{MailAction, MailError};

use super::*;

/// Server entity bits of the Goldshire mailbox in these fixtures.
const MAILBOX: u64 = 0x0000_0001_0000_2A01;

fn fixture() -> App {
    let mut app = App::new();
    app.add_plugins(MinimalPlugins)
        .add_message::<MailRequest>()
        .add_message::<NpcFrameEvent>()
        .init_resource::<MailState>()
        .init_resource::<UiErrors>()
        .init_resource::<ConnectionSender>()
        .init_resource::<Inbox<MailboxContents>>()
        .init_resource::<Inbox<MailSent>>()
        .init_resource::<Inbox<MailFailed>>()
        .init_resource::<Inbox<PendingMail>>();
    app
}

fn deliver<M: lightyear::prelude::Message>(app: &mut App, messages: Vec<M>) {
    app.world_mut().insert_resource(Inbox::new(messages));
}

fn run<M, S: IntoSystem<(), (), M>>(app: &mut App, system: S) {
    app.world_mut().run_system_once(system).unwrap();
}

fn contents() -> MailboxContents {
    MailboxContents {
        object: MAILBOX,
        mails: Vec::new(),
        now: 1_790_000_000,
    }
}

#[test]
fn the_mailbox_role_opens_the_inbox_and_the_server_answers_update_it() {
    let mut app = fixture();
    // The inbox can arrive in the same batch as InteractionOpened.
    deliver(&mut app, vec![contents()]);
    deliver(
        &mut app,
        vec![PendingMail {
            senders: vec!["Tradea".into()],
        }],
    );
    run(&mut app, receive_mail);
    app.world_mut().write_message(NpcFrameEvent::Opened {
        npc: MAILBOX,
        role: NpcRole::Mailbox,
    });
    run(&mut app, follow_interactions);
    app.world_mut().resource_mut::<MailState>().attach(41);
    deliver(&mut app, vec![MailSent { object: MAILBOX }]);
    deliver(
        &mut app,
        vec![MailFailed {
            object: MAILBOX,
            error: MailError::RecipientNotFound,
        }],
    );
    run(&mut app, receive_mail);

    let mail = app.world().resource::<MailState>();
    assert_eq!(mail.object, Some(MAILBOX));
    assert!(mail.contents.is_some());
    assert_eq!(mail.pending_senders, vec!["Tradea".to_string()]);
    assert!(mail.attachments.is_empty());
    let lines: Vec<_> = app
        .world()
        .resource::<UiErrors>()
        .lines
        .iter()
        .map(|line| line.text.clone())
        .collect();
    assert_eq!(lines, vec!["Cannot find mail recipient.", "Mail sent."]);

    app.world_mut()
        .write_message(NpcFrameEvent::Closed { npc: MAILBOX });
    run(&mut app, follow_interactions);
    assert!(!app.world().resource::<MailState>().is_open());
}

#[test]
fn mail_requests_go_out_only_while_a_mailbox_is_open() {
    let mut app = fixture();
    let (sender, commands) = mpsc::channel();
    app.insert_resource(ConnectionSender::new(Some(sender)));
    let take = MailRequest::Act {
        mail_id: 4,
        action: MailAction::TakeMoney,
    };
    app.world_mut().write_message(take.clone());
    run(&mut app, send_mail_requests);
    assert!(commands.try_recv().is_err());
    app.world_mut()
        .resource_mut::<Messages<MailRequest>>()
        .clear();

    app.world_mut().resource_mut::<MailState>().open(MAILBOX);
    app.world_mut().write_message(take);
    app.world_mut().write_message(MailRequest::Send(MailDraft {
        recipient: "Tradeb".into(),
        attachments: vec![41],
        money: 5_000,
        ..MailDraft::default()
    }));
    run(&mut app, send_mail_requests);
    for _ in 0..2 {
        assert!(matches!(commands.try_recv(), Ok(NetworkCommand::Apply(_))));
    }
    assert!(commands.try_recv().is_err());
}
