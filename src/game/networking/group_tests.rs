use std::sync::mpsc;

use bevy::ecs::system::RunSystemOnce;
use game_engine::network_runtime::messages::{ConnectionSender, Inbox};
use game_engine::network_runtime::worker::NetworkCommand;
use shared::components::Position;
use shared::death::DeathState;
use shared::loot::LootMode;
use shared::protocol::{
    GroupMemberSnapshot, GroupMemberState, GroupMessageCode, GroupRoleSnapshot, ReadyCheckAnswer,
    ReadyCheckMemberSnapshot,
};

use super::*;

fn fixture() -> App {
    let mut app = App::new();
    app.add_plugins(MinimalPlugins)
        .add_message::<GroupCommand>()
        .init_resource::<GroupState>()
        .init_resource::<ChatState>()
        .init_resource::<ConnectionSender>()
        .init_resource::<Inbox<GroupRosterSnapshot>>()
        .init_resource::<Inbox<GroupMemberStates>>()
        .init_resource::<Inbox<GroupInvitePrompt>>()
        .init_resource::<Inbox<GroupInviteCancelled>>()
        .init_resource::<Inbox<ReadyCheckUpdate>>()
        .init_resource::<Inbox<GroupCommandResponse>>();
    app
}

fn deliver<M: lightyear::prelude::Message>(app: &mut App, messages: Vec<M>) {
    app.world_mut().insert_resource(Inbox::new(messages));
}

fn receive(app: &mut App) {
    app.world_mut().run_system_once(receive_group).unwrap();
}

fn member(name: &str, leader: bool) -> GroupMemberSnapshot {
    GroupMemberSnapshot {
        name: name.into(),
        role: GroupRoleSnapshot::Healer,
        is_leader: leader,
        online: true,
        subgroup: 1,
        class: 5,
        level: 20,
        entity: Some(99),
    }
}

#[test]
fn roster_and_member_states_fill_group_state() {
    let mut app = fixture();
    deliver(
        &mut app,
        vec![GroupRosterSnapshot {
            is_raid: false,
            ready_count: 0,
            total_count: 0,
            members: vec![member("party_a", true), member("party_b", false)],
            loot_method: LootMode::PersonalLoot,
        }],
    );
    deliver(
        &mut app,
        vec![GroupMemberStates {
            members: vec![GroupMemberState {
                name: "party_b".into(),
                health: 310,
                max_health: 1200,
                power: None,
                death: DeathState::Dead,
                position: Position {
                    x: 5.0,
                    y: 6.0,
                    z: 7.0,
                },
                debuffs: Vec::new(),
            }],
        }],
    );
    receive(&mut app);

    let group = app.world().resource::<GroupState>();
    assert!(group.is_leader("party_a"));
    assert_eq!(group.members.len(), 2);
    assert_eq!(group.live["party_b"].health, 310);
    assert_eq!(group.live["party_b"].death, DeathState::Dead);
}

#[test]
fn invite_prompt_is_pending_until_the_same_inviter_cancels() {
    let mut app = fixture();
    deliver(
        &mut app,
        vec![GroupInvitePrompt {
            inviter_name: "party_a".into(),
            timeout_secs: 60.0,
        }],
    );
    receive(&mut app);
    assert_eq!(
        app.world()
            .resource::<GroupState>()
            .pending_invite
            .as_deref(),
        Some("party_a")
    );

    deliver(
        &mut app,
        vec![GroupInviteCancelled {
            inviter_name: "someone_else".into(),
        }],
    );
    receive(&mut app);
    assert!(
        app.world()
            .resource::<GroupState>()
            .pending_invite
            .is_some()
    );
    deliver(
        &mut app,
        vec![GroupInviteCancelled {
            inviter_name: "party_a".into(),
        }],
    );
    receive(&mut app);

    assert_eq!(app.world().resource::<GroupState>().pending_invite, None);
}

#[test]
fn ready_check_update_and_server_notice_reach_state_and_chat() {
    let mut app = fixture();
    deliver(
        &mut app,
        vec![ReadyCheckUpdate {
            initiator_name: "party_a".into(),
            time_remaining_secs: 30.0,
            members: vec![ReadyCheckMemberSnapshot {
                name: "party_b".into(),
                answer: ReadyCheckAnswer::Pending,
            }],
            finished: false,
        }],
    );
    deliver(
        &mut app,
        vec![GroupCommandResponse {
            message: "party_a has initiated a ready check.".into(),
            code: GroupMessageCode::ReadyCheckStarted,
        }],
    );
    receive(&mut app);

    assert!(
        app.world()
            .resource::<GroupState>()
            .awaits_ready_answer("party_b")
    );
    let chat = app.world().resource::<ChatState>();
    let last = chat.messages.last().expect("system line");
    assert_eq!(last.channel_type, ChatChannelType::System);
    assert_eq!(last.text, "party_a has initiated a ready check.");
}

#[test]
fn each_group_command_goes_to_the_network_worker() {
    let mut app = fixture();
    let (sender, commands) = mpsc::channel();
    app.insert_resource(ConnectionSender::new(Some(sender)));
    let all = [
        GroupCommand::Invite("party_b".into()),
        GroupCommand::Uninvite("party_b".into()),
        GroupCommand::Promote("party_b".into()),
        GroupCommand::Leave,
        GroupCommand::ConvertToRaid,
        GroupCommand::ConvertToParty,
        GroupCommand::SetRole {
            name: "party_b".into(),
            role: GroupRoleSnapshot::Tank,
        },
        GroupCommand::StartReadyCheck,
        GroupCommand::RespondReadyCheck(true),
        GroupCommand::RespondInvite(true),
    ];
    for command in all.clone() {
        app.world_mut().write_message(command);
    }

    app.world_mut()
        .run_system_once(send_group_commands)
        .unwrap();

    let sent = commands
        .try_iter()
        .filter(|c| matches!(c, NetworkCommand::Apply(_)))
        .count();
    assert_eq!(sent, all.len());
}
