//! `mail status|send|take-money` through the public CLI at a scripted mailbox: the
//! server answers `UseGameObject` with the Mailbox role and one inbox mail, a
//! `SendMail` with `MailSent`, and records the inbox action.
use shared::protocol::{
    GameObjectInfo, ItemStack, MailAction, MailAttachment, MailChannel, MailHeader, MailRequest,
    MailSent, MailboxContents, SendMail, UseGameObject,
};

use super::*;

/// AzerothCore `GAMEOBJECT_TYPE_MAILBOX`; display 1727 is model FDID 198004.
const MAILBOX_TYPE: u8 = 19;
const MAILBOX_DISPLAY: u32 = 1727;

/// What the client sent the scripted mailbox, as the server decoded it.
#[derive(Resource, Default)]
struct Mailbox {
    object: Option<Entity>,
    uses: Vec<u64>,
    sent: Vec<SendMail>,
    actions: Vec<MailRequest>,
}

pub(super) fn install(app: &mut App) {
    app.init_resource::<Mailbox>();
    app.add_systems(Update, answer_mailbox);
}

fn inbox(object: u64) -> MailboxContents {
    MailboxContents {
        object,
        mails: vec![MailHeader {
            mail_id: 4,
            sender: "Tradea".into(),
            subject: "Linen".into(),
            body: String::new(),
            money: 5_000,
            cod: 0,
            attachments: vec![MailAttachment {
                slot: 0,
                item: ItemStack {
                    item_guid: 90,
                    item_id: 2589,
                    count: 20,
                    durability: None,
                    soulbound: false,
                },
                name: "Linen Cloth".into(),
                quality: 1,
            }],
            expires_at: 1_800_000_000,
            read: false,
            returned: false,
            from_player: true,
            returnable: true,
        }],
        now: 0,
    }
}

fn answer_mailbox(
    mut uses: Query<&mut MessageReceiver<UseGameObject>>,
    mut sends: Query<&mut MessageReceiver<SendMail>>,
    mut actions: Query<&mut MessageReceiver<MailRequest>>,
    mut opened: Query<&mut MessageSender<InteractionOpened>>,
    mut contents: Query<&mut MessageSender<MailboxContents>>,
    mut sent: Query<&mut MessageSender<MailSent>>,
    mut mailbox: ResMut<Mailbox>,
) {
    for object in uses
        .iter_mut()
        .flat_map(|mut r| r.receive().collect::<Vec<_>>())
    {
        mailbox.uses.push(object.object);
        for mut sender in &mut opened {
            sender.send::<InteractionChannel>(InteractionOpened {
                npc: object.object,
                kind: InteractionKind::Role(NpcRole::Mailbox),
            });
        }
        for mut sender in &mut contents {
            sender.send::<MailChannel>(inbox(object.object));
        }
    }
    for mail in sends
        .iter_mut()
        .flat_map(|mut r| r.receive().collect::<Vec<_>>())
    {
        for mut sender in &mut sent {
            sender.send::<MailChannel>(MailSent {
                object: mail.object,
            });
        }
        mailbox.sent.push(mail);
    }
    for mut receiver in &mut actions {
        mailbox.actions.extend(receiver.receive());
    }
}

/// A Stormwind mailbox 2 yd north of the spawn.
fn spawn_mailbox(app: &mut App) -> u64 {
    let object = app
        .world_mut()
        .spawn((
            GameObjectInfo {
                entry: 197_134,
                go_type: MAILBOX_TYPE,
                display_id: MAILBOX_DISPLAY,
                name: "Mailbox".into(),
                scale: 1.0,
            },
            Position {
                x: FIRST[0],
                y: FIRST[1],
                z: FIRST[2] - 2.0,
            },
            Replicate::to_clients(NetworkTarget::All),
        ))
        .id();
    app.world_mut().resource_mut::<Mailbox>().object = Some(object);
    object.to_bits()
}

fn wait_mailbox(run: &mut Run, what: &str, seen: impl Fn(&Mailbox) -> bool) -> Result<(), String> {
    let deadline = Instant::now() + Duration::from_secs(10);
    while !seen(run.app.world().resource::<Mailbox>()) {
        if Instant::now() >= deadline {
            return Err(format!("the mailbox server never decoded {what}"));
        }
        run.pump()?;
        thread::sleep(TICK);
    }
    Ok(())
}

pub(super) fn check_mail(run: &mut Run) -> Result<(), String> {
    expect_exact(
        run,
        &["mail", "status"],
        "mailbox: closed\npending_mail: \nsent: 0\ninbox: 0",
    )?;
    match run.cli(&["mail", "take-money", "--mail-id", "4"])? {
        Err(error) if error.contains("no mailbox is open") => {}
        other => return Err(format!("take-money without a mailbox answered {other:?}")),
    }
    let object = spawn_mailbox(run.app);
    expect_eventually(
        run,
        &["quest", "interact", "--npc", "mailbox"],
        "use Mailbox",
    )?;
    wait_mailbox(run, "UseGameObject", |m| m.uses.contains(&object))?;
    let mail = "4 from=Tradea subject=Linen money=5000 cod=0 items=[0:2589x20] read=false returned=false expires_at=1800000000";
    expect_eventually(
        run,
        &["mail", "status"],
        &format!("mailbox: open {object}\npending_mail: \nsent: 0\ninbox: 1\n{mail}"),
    )?;
    expect_exact(
        run,
        &[
            "mail",
            "send",
            "--to",
            "Tradeb",
            "--subject",
            "Hi",
            "--body",
            "Yo",
            "--money",
            "100",
        ],
        "mail send queued to Tradeb",
    )?;
    wait_mailbox(run, "SendMail", |m| !m.sent.is_empty())?;
    let sent = run.app.world().resource::<Mailbox>().sent[0].clone();
    let expected = SendMail {
        object,
        recipient: "Tradeb".into(),
        subject: "Hi".into(),
        body: "Yo".into(),
        attachments: Vec::new(),
        money: 100,
        cod: 0,
    };
    if sent != expected {
        return Err(format!("SendMail decoded as {sent:?}"));
    }
    expect_eventually(
        run,
        &["mail", "status"],
        &format!("mailbox: open {object}\npending_mail: \nsent: 1\ninbox: 1\n{mail}"),
    )?;
    expect_exact(
        run,
        &["mail", "take-money", "--mail-id", "4"],
        "mail 4 TakeMoney queued",
    )?;
    wait_mailbox(run, "MailRequest", |m| !m.actions.is_empty())?;
    let action = run.app.world().resource::<Mailbox>().actions[0];
    let expected = MailRequest {
        object,
        mail_id: 4,
        action: MailAction::TakeMoney,
    };
    if action != expected {
        return Err(format!("MailRequest decoded as {action:?}"));
    }
    Ok(())
}
