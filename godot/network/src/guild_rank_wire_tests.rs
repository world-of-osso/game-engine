//! GuildChannel proof through real loopback UDP. Own ephemeral fixture, no dev server.
use super::*;
use shared::protocol::*;

#[derive(Resource, Default)]
struct Requests(Vec<GuildRankRequest>);
fn capture_requests(
    mut receivers: Query<&mut MessageReceiver<GuildRankRequest>>,
    mut requests: ResMut<Requests>,
) {
    for mut receiver in &mut receivers {
        requests.0.extend(receiver.receive());
    }
}
fn install(app: &mut App) {
    app.init_resource::<Requests>();
    app.add_systems(Update, capture_requests);
}
fn connect(server: &mut App, bridge: &mut NetworkBridge) {
    let deadline = Instant::now() + Duration::from_secs(10);
    loop {
        assert!(Instant::now() < deadline, "guild connection timeout");
        server.update();
        if bridge
            .drain_events()
            .unwrap()
            .iter()
            .any(|event| matches!(event, Event::Connected))
        {
            return;
        }
        thread::sleep(Duration::from_millis(5));
    }
}
#[test]
fn guild_member_rank_assignment_target_and_zero_based_destination_cross_real_udp() {
    let (mut server, address) = crate::wire_tests::start_fixture_server_with(install);
    let mut bridge = NetworkBridge::connect(address, 8277).unwrap();
    connect(&mut server, &mut bridge);
    let request = GuildRankRequest::SetMemberRank {
        character_name: "Cara".into(),
        rank: 1,
    };
    bridge.send::<_, GuildChannel>(request.clone()).unwrap();
    let deadline = Instant::now() + Duration::from_secs(3);
    while server.world().resource::<Requests>().0.is_empty() && Instant::now() < deadline {
        server.update();
        thread::sleep(Duration::from_millis(5));
    }
    bridge.stop().unwrap();
    assert_eq!(server.world().resource::<Requests>().0, vec![request]);
}

#[test]
fn guild_charter_registrar_crosses_real_udp() {
    let (mut server, address) = crate::wire_tests::start_fixture_server_with(|_| {});
    let mut bridge = NetworkBridge::connect(address, 8264).unwrap();
    connect(&mut server, &mut bridge);
    let reply = GuildCharterReply::Registrar(GuildRegistrarSnapshot {
        npc: 810,
        npc_name: "Aldwin".into(),
        charter_cost: 1000,
        charter_guid: None,
    });
    let world = server.world_mut();
    world
        .query::<&mut MessageSender<GuildCharterReply>>()
        .single_mut(world)
        .unwrap()
        .send::<GuildChannel>(reply.clone());
    let deadline = Instant::now() + Duration::from_secs(3);
    let mut received = None;
    while received.is_none() && Instant::now() < deadline {
        server.update();
        for event in bridge.drain_events().unwrap() {
            if let Event::Message(message) = event {
                if message.is::<GuildCharterReply>() {
                    received = Some(message.downcast::<GuildCharterReply>().ok().unwrap());
                }
            }
        }
        thread::sleep(Duration::from_millis(5));
    }
    bridge.stop().unwrap();
    assert_eq!(received, Some(reply));
}

#[derive(Resource, Default)]
struct Declines(usize);
fn capture_declines(
    mut receivers: Query<&mut MessageReceiver<GuildDeclineInviteRequest>>,
    mut declines: ResMut<Declines>,
) {
    for mut receiver in &mut receivers {
        declines.0 += receiver.receive().count();
    }
}
#[test]
fn guild_invitation_and_decline_cross_real_udp() {
    let (mut server, address) = crate::wire_tests::start_fixture_server_with(|app| {
        app.init_resource::<Declines>()
            .add_systems(Update, capture_declines);
    });
    let mut bridge = NetworkBridge::connect(address, 8262).unwrap();
    connect(&mut server, &mut bridge);
    for tabard in [
        None,
        Some(shared::guild_creation::GuildTabardDesign {
            icon: 1,
            icon_color: 2,
            border: 3,
            border_color: 4,
            background_color: 5,
        }),
    ] {
        let invitation = GuildInvitation {
            guild_id: 7,
            guild_name: "Raid Team".into(),
            inviter_name: "Alice".into(),
            tabard,
        };
        let world = server.world_mut();
        world
            .query::<&mut MessageSender<GuildInvitation>>()
            .single_mut(world)
            .unwrap()
            .send::<AuthChannel>(invitation.clone());
        let deadline = Instant::now() + Duration::from_secs(10);
        let mut received = None;
        while received.is_none() && Instant::now() < deadline {
            server.update();
            for event in bridge.drain_events().unwrap() {
                if let Event::Message(message) = event {
                    if message.is::<GuildInvitation>() {
                        received = Some(message.downcast::<GuildInvitation>().ok().unwrap());
                    }
                }
            }
            thread::sleep(Duration::from_millis(5));
        }
        assert_eq!(received, Some(invitation));
    }
    bridge
        .send::<_, AuthChannel>(GuildDeclineInviteRequest)
        .unwrap();
    let deadline = Instant::now() + Duration::from_secs(10);
    while server.world().resource::<Declines>().0 == 0 && Instant::now() < deadline {
        server.update();
        thread::sleep(Duration::from_millis(5));
    }
    bridge.stop().unwrap();
    assert_eq!(server.world().resource::<Declines>().0, 1);
}

#[test]
fn guild_member_note_profile_fields_cross_real_udp() {
    let (mut server, address) = crate::wire_tests::start_fixture_server_with(|_| {});
    let mut bridge = NetworkBridge::connect(address, 8265).unwrap();
    connect(&mut server, &mut bridge);
    let reply = GuildStateUpdate {
        guild: Some(GuildSnapshot {
            guild_id: 7,
            guild_name: "Guild Testers".into(),
            tabard: None,
            created_at: None,
            motd: String::new(),
            info_text: String::new(),
            members: [("Cara", "Alchemy", true), ("Dora", "Cooking", false)]
                .into_iter()
                .map(|(name, note, is_self)| GuildMemberSnapshot {
                    character_name: name.into(),
                    level: 80,
                    class_name: "Priest".into(),
                    rank_name: "Member".into(),
                    is_online: true,
                    public_note: note.into(),
                    officer_note: String::new(),
                    is_self,
                    last_logout_unix_secs: None,
                    online_zone: None,
                })
                .collect(),
        }),
        message: None,
        error: None,
    };
    let world = server.world_mut();
    world
        .query::<&mut MessageSender<GuildStateUpdate>>()
        .single_mut(world)
        .unwrap()
        .send::<GuildChannel>(reply.clone());
    let deadline = Instant::now() + Duration::from_secs(10);
    let mut received = None;
    while received.is_none() && Instant::now() < deadline {
        server.update();
        for event in bridge.drain_events().unwrap() {
            if let Event::Message(message) = event {
                if message.is::<GuildStateUpdate>() {
                    received = Some(message.downcast::<GuildStateUpdate>().ok().unwrap());
                }
            }
        }
        thread::sleep(Duration::from_millis(5));
    }
    bridge.stop().unwrap();
    let received = received.expect("guild member note profile over UDP");
    assert_eq!(received.guild, reply.guild);
    assert_eq!(received.message, reply.message);
    assert_eq!(received.error, reply.error);
}

#[test]
fn guild_event_log_wire_preserves_six_kinds_order_and_typed_refusal() {
    let (mut server, address) = crate::wire_tests::start_fixture_server_with(install);
    let mut bridge = NetworkBridge::connect(address, 8262).unwrap();
    connect(&mut server, &mut bridge);
    bridge
        .send::<_, GuildChannel>(GuildRankRequest::QueryEventLog)
        .unwrap();
    let deadline = Instant::now() + Duration::from_secs(10);
    while server.world().resource::<Requests>().0.is_empty() && Instant::now() < deadline {
        server.update();
        thread::sleep(Duration::from_millis(5));
    }
    assert_eq!(
        server.world().resource::<Requests>().0,
        vec![GuildRankRequest::QueryEventLog]
    );
    let history = GuildEventLog {
        events: [
            GuildEventKind::Invite {
                actor: "Ada".into(),
                member: "Ben".into(),
            },
            GuildEventKind::Join {
                member: "Ben".into(),
            },
            GuildEventKind::Promote {
                actor: "Ada".into(),
                member: "Ben".into(),
                rank: "Veteran".into(),
            },
            GuildEventKind::Demote {
                actor: "Ada".into(),
                member: "Ben".into(),
                rank: "Member".into(),
            },
            GuildEventKind::Remove {
                actor: "Ada".into(),
                member: "Ben".into(),
            },
            GuildEventKind::Quit {
                member: "Ada".into(),
            },
        ]
        .into_iter()
        .map(|kind| GuildEvent {
            timestamp: 123,
            kind,
        })
        .collect(),
        server_time: 456,
        error: None,
    };
    let refused = GuildEventLog {
        events: vec![],
        server_time: 457,
        error: Some(GuildRankError::NotInGuild),
    };
    for reply in [&history, &refused] {
        let world = server.world_mut();
        world
            .query::<&mut MessageSender<GuildEventLog>>()
            .single_mut(world)
            .unwrap()
            .send::<GuildChannel>(reply.clone());
        server.update();
    }
    let mut received = Vec::new();
    let deadline = Instant::now() + Duration::from_secs(10);
    while received.len() < 2 && Instant::now() < deadline {
        server.update();
        for event in bridge.drain_events().unwrap() {
            if let Event::Message(message) = event
                && message.is::<GuildEventLog>()
            {
                received.push(message.downcast::<GuildEventLog>().ok().unwrap());
            }
        }
        thread::sleep(Duration::from_millis(5));
    }
    bridge.stop().unwrap();
    assert_eq!(received, vec![history, refused]);
}

#[test]
fn guild_ranks_wire_sends_exact_requests_and_receives_ordered_authority() {
    let (mut server, address) = crate::wire_tests::start_fixture_server_with(install);
    let mut bridge = NetworkBridge::connect(address, 8261).unwrap();
    connect(&mut server, &mut bridge);
    let requests = vec![
        GuildRankRequest::Query,
        GuildRankRequest::SetMemberNote {
            character_name: "Cara".into(),
            note: "Alchemy".into(),
            officer: false,
        },
        GuildRankRequest::SetMemberNote {
            character_name: "Cara".into(),
            note: "Raid healer".into(),
            officer: true,
        },
        GuildRankRequest::Add {
            name: "Raider".into(),
        },
        GuildRankRequest::Rename {
            rank: 2,
            name: "Officer".into(),
        },
        GuildRankRequest::Move { rank: 2, up: true },
        GuildRankRequest::SetPermissions {
            rank: 2,
            rights: GUILD_RIGHT_CHAT_LISTEN | GUILD_RIGHT_WITHDRAW_GOLD,
            gold_per_day: 70_000,
        },
        GuildRankRequest::SetTab {
            rank: 2,
            tab: 0,
            view: true,
            deposit: false,
            withdrawals_per_day: 5,
        },
        GuildRankRequest::Promote {
            character_name: "Cara".into(),
        },
        GuildRankRequest::Demote {
            character_name: "Cara".into(),
        },
        GuildRankRequest::Remove { rank: 3 },
        GuildRankRequest::RemoveMember {
            character_name: "Cara".into(),
        },
        GuildRankRequest::Leave,
    ];
    for request in &requests {
        bridge.send::<_, GuildChannel>(request.clone()).unwrap();
    }
    let deadline = Instant::now() + Duration::from_secs(10);
    while server.world().resource::<Requests>().0.len() < requests.len()
        && Instant::now() < deadline
    {
        server.update();
        thread::sleep(Duration::from_millis(5));
    }
    assert_eq!(server.world().resource::<Requests>().0, requests);
    let state = GuildRanksState {
        own_rank: 0,
        ranks: vec![GuildRankSettings {
            name: "Officer".into(),
            rights: GUILD_RIGHT_CHAT_LISTEN,
            gold_per_day: 70_000,
            tabs: vec![GuildRankTab {
                view: true,
                deposit: false,
                withdrawals_per_day: 5,
            }],
        }],
        members: vec![GuildRankMember {
            character_name: "Cara".into(),
            rank: 0,
        }],
        tab_names: vec!["Materials".into()],
        error: None,
    };
    let mut refused = state.clone();
    refused.error = Some(GuildRankError::Permissions);
    for state in [&state, &refused] {
        let world = server.world_mut();
        world
            .query::<&mut MessageSender<GuildRanksState>>()
            .single_mut(world)
            .unwrap()
            .send::<GuildChannel>(state.clone());
        server.update();
    }
    let deadline = Instant::now() + Duration::from_secs(10);
    let mut received = Vec::new();
    while received.len() < 2 && Instant::now() < deadline {
        server.update();
        for event in bridge.drain_events().unwrap() {
            if let Event::Message(message) = event {
                if message.is::<GuildRanksState>() {
                    received.push(message.downcast::<GuildRanksState>().ok().unwrap());
                }
            }
        }
        thread::sleep(Duration::from_millis(5));
    }
    bridge.stop().unwrap();
    assert_eq!(received, vec![state, refused]);
}
