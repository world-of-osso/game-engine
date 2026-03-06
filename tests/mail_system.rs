use game_engine::mail::{
    DeleteMail, ListMailQuery, MailState, ReadMail, SendMail,
};

#[test]
fn send_mail_lists_it_for_the_recipient() {
    let mut state = MailState::default();

    let delivery = state
        .send(SendMail {
            to: "Thrall".into(),
            from: "Jaina".into(),
            subject: "Supplies".into(),
            body: "Three crates are ready at the harbor.".into(),
            money: 1250,
        })
        .expect("mail should be accepted");

    let mailbox = state.list(ListMailQuery {
        character: Some("Thrall".into()),
        include_deleted: false,
    });

    assert_eq!(delivery.mail_id, 1);
    assert_eq!(mailbox.len(), 1);
    assert_eq!(mailbox[0].mail_id, delivery.mail_id);
    assert_eq!(mailbox[0].from, "Jaina");
    assert_eq!(mailbox[0].to, "Thrall");
    assert_eq!(mailbox[0].subject, "Supplies");
    assert_eq!(mailbox[0].money, 1250);
    assert!(!mailbox[0].claimed);
    assert!(!mailbox[0].deleted);
}

#[test]
fn read_mail_marks_it_as_read_without_claiming_it() {
    let mut state = MailState::default();
    let delivery = state
        .send(SendMail {
            to: "Thrall".into(),
            from: "Jaina".into(),
            subject: "Supplies".into(),
            body: "Three crates are ready at the harbor.".into(),
            money: 1250,
        })
        .expect("mail should be accepted");

    let entry = state
        .read(ReadMail {
            mail_id: delivery.mail_id,
        })
        .expect("mail should exist");

    assert_eq!(entry.body, "Three crates are ready at the harbor.");
    assert!(entry.read);
    assert!(!entry.claimed);
}

#[test]
fn claim_mail_marks_attached_money_as_claimed() {
    let mut state = MailState::default();
    let delivery = state
        .send(SendMail {
            to: "Thrall".into(),
            from: "Jaina".into(),
            subject: "Supplies".into(),
            body: "Three crates are ready at the harbor.".into(),
            money: 1250,
        })
        .expect("mail should be accepted");

    let claimed = state
        .claim(delivery.mail_id)
        .expect("mail should be claimable");

    let mailbox = state.list(ListMailQuery {
        character: Some("Thrall".into()),
        include_deleted: false,
    });

    assert_eq!(claimed.mail_id, delivery.mail_id);
    assert_eq!(claimed.money, 1250);
    assert!(mailbox[0].claimed);
}

#[test]
fn delete_mail_hides_it_from_default_listing() {
    let mut state = MailState::default();
    let delivery = state
        .send(SendMail {
            to: "Thrall".into(),
            from: "Jaina".into(),
            subject: "Supplies".into(),
            body: "Three crates are ready at the harbor.".into(),
            money: 1250,
        })
        .expect("mail should be accepted");

    state
        .delete(DeleteMail {
            mail_id: delivery.mail_id,
        })
        .expect("mail should be deletable");

    let visible = state.list(ListMailQuery {
        character: Some("Thrall".into()),
        include_deleted: false,
    });
    let all = state.list(ListMailQuery {
        character: Some("Thrall".into()),
        include_deleted: true,
    });

    assert!(visible.is_empty());
    assert_eq!(all.len(), 1);
    assert!(all[0].deleted);
}

#[test]
fn sending_mail_requires_non_empty_addresses_and_subject() {
    let mut state = MailState::default();

    let error = state
        .send(SendMail {
            to: "".into(),
            from: "Jaina".into(),
            subject: "".into(),
            body: "x".into(),
            money: 0,
        })
        .expect_err("invalid mail should be rejected");

    assert_eq!(error, "to, from, and subject are required");
}
