//! Party/raid networking: roster, live member states, invite prompts and ready checks
//! from the server fill [`GroupState`]; server notices go to chat as system lines; and
//! [`GroupCommand`]s go out on `GroupChannel`.

use bevy::ecs::system::SystemParam;
use bevy::prelude::*;
use game_engine::chat_data::{
    ChatChannelType, ChatMessage as RuntimeChatMessage, ChatState, now_timestamp,
};
use game_engine::group_state::{GroupCommand, GroupState};
use game_engine::network_runtime::messages::{MessageReceivers, MessageSenders};
use lightyear::prelude::Message as NetworkMessage;
use shared::protocol::{
    ConvertGroupToParty, ConvertGroupToRaid, GroupChannel, GroupCommandResponse,
    GroupInviteCancelled, GroupInviteIntent, GroupInvitePrompt, GroupMemberStates,
    GroupRosterSnapshot, GroupUninviteIntent, LeaveGroup, PromoteGroupLeader, ReadyCheckUpdate,
    RespondGroupInvite, RespondReadyCheck, SetGroupRole, StartReadyCheck,
};

use crate::game_state::GameState;

pub struct GroupNetworkPlugin;

impl Plugin for GroupNetworkPlugin {
    fn build(&self, app: &mut App) {
        use game_engine::network_events::{add_message_route, register_message_handler};

        app.init_resource::<GroupState>()
            .init_resource::<ChatState>()
            .add_message::<GroupCommand>();
        let handler =
            register_message_handler::<GroupRosterSnapshot, _>(app, receive_group, in_world);
        add_message_route::<GroupMemberStates>(app, handler);
        add_message_route::<GroupInvitePrompt>(app, handler);
        add_message_route::<GroupInviteCancelled>(app, handler);
        add_message_route::<ReadyCheckUpdate>(app, handler);
        add_message_route::<GroupCommandResponse>(app, handler);
        app.add_systems(
            Update,
            (send_group_commands, tick_ready_check).run_if(in_state(GameState::InWorld)),
        );
    }
}

fn in_world(world: &World) -> bool {
    *world.resource::<State<GameState>>().get() == GameState::InWorld
}

#[derive(SystemParam)]
struct GroupReceivers<'w, 's> {
    rosters: MessageReceivers<'w, 's, GroupRosterSnapshot>,
    states: MessageReceivers<'w, 's, GroupMemberStates>,
    prompts: MessageReceivers<'w, 's, GroupInvitePrompt>,
    cancelled: MessageReceivers<'w, 's, GroupInviteCancelled>,
    ready_checks: MessageReceivers<'w, 's, ReadyCheckUpdate>,
    responses: MessageReceivers<'w, 's, GroupCommandResponse>,
}

fn receive_group(
    mut rx: GroupReceivers,
    mut group: ResMut<GroupState>,
    mut chat: ResMut<ChatState>,
) {
    for inbox in rx.rosters.iter_mut() {
        for roster in inbox.receive() {
            group.apply_roster(roster);
        }
    }
    for inbox in rx.states.iter_mut() {
        for update in inbox.receive() {
            group.apply_member_states(update.members);
        }
    }
    for inbox in rx.prompts.iter_mut() {
        for prompt in inbox.receive() {
            group.pending_invite = Some(prompt.inviter_name);
        }
    }
    for inbox in rx.cancelled.iter_mut() {
        for cancelled in inbox.receive() {
            if group.pending_invite.as_deref() == Some(cancelled.inviter_name.as_str()) {
                group.pending_invite = None;
            }
        }
    }
    for inbox in rx.ready_checks.iter_mut() {
        for update in inbox.receive() {
            group.apply_ready_check(update);
        }
    }
    for inbox in rx.responses.iter_mut() {
        for response in inbox.receive() {
            add_system_line(&mut chat, &response.message);
            group.last_server_message = Some(response.message);
        }
    }
}

/// Retail prints group results (`ERR_*`, `READY_CHECK_*`) as yellow system chat lines.
fn add_system_line(chat: &mut ChatState, text: &str) {
    chat.add_message(RuntimeChatMessage {
        channel_type: ChatChannelType::System,
        channel_name: String::new(),
        sender: String::new(),
        text: text.to_string(),
        timestamp: now_timestamp(),
    });
}

fn tick_ready_check(time: Res<Time>, mut group: ResMut<GroupState>) {
    if group
        .ready_check
        .as_ref()
        .is_some_and(|v| v.finished_for.is_some())
    {
        group.tick_ready_check(time.delta_secs());
    }
}

#[derive(SystemParam)]
struct GroupSenders<'w, 's> {
    invite: MessageSenders<'w, 's, GroupInviteIntent>,
    uninvite: MessageSenders<'w, 's, GroupUninviteIntent>,
    promote: MessageSenders<'w, 's, PromoteGroupLeader>,
    leave: MessageSenders<'w, 's, LeaveGroup>,
    to_raid: MessageSenders<'w, 's, ConvertGroupToRaid>,
    to_party: MessageSenders<'w, 's, ConvertGroupToParty>,
    role: MessageSenders<'w, 's, SetGroupRole>,
    ready_start: MessageSenders<'w, 's, StartReadyCheck>,
    ready_answer: MessageSenders<'w, 's, RespondReadyCheck>,
    invite_answer: MessageSenders<'w, 's, RespondGroupInvite>,
}

fn send_group_commands(mut commands: MessageReader<GroupCommand>, mut senders: GroupSenders) {
    for command in commands.read() {
        send_group_command(command.clone(), &mut senders);
    }
}

fn send_group_command(command: GroupCommand, s: &mut GroupSenders) {
    match command {
        GroupCommand::Invite(name) => send(&mut s.invite, GroupInviteIntent { name }),
        GroupCommand::Uninvite(name) => send(&mut s.uninvite, GroupUninviteIntent { name }),
        GroupCommand::Promote(name) => send(&mut s.promote, PromoteGroupLeader { name }),
        GroupCommand::Leave => send(&mut s.leave, LeaveGroup),
        GroupCommand::ConvertToRaid => send(&mut s.to_raid, ConvertGroupToRaid),
        GroupCommand::ConvertToParty => send(&mut s.to_party, ConvertGroupToParty),
        GroupCommand::SetRole { name, role } => send(&mut s.role, SetGroupRole { name, role }),
        GroupCommand::StartReadyCheck => send(&mut s.ready_start, StartReadyCheck),
        GroupCommand::RespondReadyCheck(ready) => {
            send(&mut s.ready_answer, RespondReadyCheck { ready })
        }
        GroupCommand::RespondInvite(accept) => {
            send(&mut s.invite_answer, RespondGroupInvite { accept })
        }
    }
}

fn send<M: NetworkMessage>(senders: &mut MessageSenders<M>, message: M) {
    let Some(mut sender) = senders.iter_mut().next() else {
        warn!(
            "[Group] not connected; dropped {}",
            std::any::type_name::<M>()
        );
        return;
    };
    sender.send::<GroupChannel>(message);
}

#[cfg(test)]
#[path = "group_tests.rs"]
mod tests;
