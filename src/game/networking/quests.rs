//! Quest and NPC interaction networking: the server quest runtime feeds
//! [`QuestRuntime`]; UI requests ([`NpcInteractionRequest`]) go out on
//! `QuestChannel` / `InteractionChannel`. NPCs are addressed by server entity bits.

use bevy::ecs::system::SystemParam;
use bevy::prelude::*;
use game_engine::chat_data::{ChatChannelType, ChatMessage, ChatState};
use game_engine::nameplate_data::QuestIndicator;
use game_engine::network_runtime::messages::{MessageReceivers, MessageSenders};
use game_engine::network_runtime::replication::ReplicationMirrorMap;
pub use game_engine::quest_runtime::{NpcFrameEvent, NpcInteractionRequest};
use game_engine::quest_runtime::{QuestRuntime, quest_failed_text};
use game_engine::status::QuestLogStatusSnapshot;
use game_engine::ui::ui_errors::UiErrors;
use lightyear::prelude::{Channel, Message as NetworkMessage};
use shared::components::Npc;
use shared::protocol::{
    AbandonQuest, CloseInteraction, InteractNpc, InteractionChannel, InteractionClosed,
    InteractionFailed, InteractionKind, InteractionOpened, NpcFlags, NpcRole, QuestChannel,
    QuestFailed, QuestGiverAcceptQuest, QuestGiverChooseReward, QuestGiverCompleteQuest,
    QuestGiverHello, QuestGiverOfferReward, QuestGiverQueryQuest, QuestGiverQuestComplete,
    QuestGiverQuestDetails, QuestGiverQuestList, QuestGiverRequestItems, QuestGiverStatus,
    QuestGiverStatusMultiple, QuestGiverStatusQuery, QuestLogSnapshot, QuestLogUpdate,
    SelectGossipOption, SetQuestWatched, UseGameObject,
};

use crate::game_state::GameState;
use crate::nameplate::NpcQuestIndicator;

pub struct QuestNetworkPlugin;

impl Plugin for QuestNetworkPlugin {
    fn build(&self, app: &mut App) {
        use game_engine::network_events::{add_message_route, register_message_handler};

        app.init_resource::<QuestRuntime>()
            .init_resource::<UiErrors>()
            .init_resource::<ReplicationMirrorMap>()
            .add_message::<NpcInteractionRequest>()
            .add_message::<NpcFrameEvent>();
        let log = register_message_handler::<QuestLogSnapshot, _>(app, receive_quest_log, in_world);
        add_message_route::<QuestLogUpdate>(app, log);
        add_message_route::<QuestGiverStatusMultiple>(app, log);
        let dialog =
            register_message_handler::<InteractionOpened, _>(app, receive_quest_dialog, in_world);
        add_message_route::<InteractionFailed>(app, dialog);
        add_message_route::<InteractionClosed>(app, dialog);
        add_message_route::<QuestGiverQuestList>(app, dialog);
        add_message_route::<QuestGiverQuestDetails>(app, dialog);
        add_message_route::<QuestGiverRequestItems>(app, dialog);
        add_message_route::<QuestGiverOfferReward>(app, dialog);
        add_message_route::<QuestGiverQuestComplete>(app, dialog);
        add_message_route::<QuestFailed>(app, dialog);
        app.add_systems(
            Update,
            (
                query_new_quest_givers,
                send_interaction_requests,
                sync_quest_indicators,
            )
                .chain()
                .run_if(in_state(GameState::InWorld)),
        );
        app.add_systems(OnExit(GameState::InWorld), reset_quest_runtime);
    }
}

fn in_world(world: &World) -> bool {
    *world.resource::<State<GameState>>().get() == GameState::InWorld
}

#[derive(SystemParam)]
struct QuestLogReceivers<'w, 's> {
    snapshots: MessageReceivers<'w, 's, QuestLogSnapshot>,
    updates: MessageReceivers<'w, 's, QuestLogUpdate>,
    statuses: MessageReceivers<'w, 's, QuestGiverStatusMultiple>,
}

fn receive_quest_log(
    mut receivers: QuestLogReceivers,
    mut runtime: ResMut<QuestRuntime>,
    mut status: ResMut<QuestLogStatusSnapshot>,
    mut chat: ResMut<ChatState>,
) {
    let mut log_changed = false;
    for inbox in receivers.snapshots.iter_mut() {
        for snapshot in inbox.receive() {
            runtime.apply_snapshot(snapshot);
            log_changed = true;
        }
    }
    for inbox in receivers.updates.iter_mut() {
        for update in inbox.receive() {
            let notices = runtime.apply_update(update);
            post_notices(notices, &mut chat);
            log_changed = true;
        }
    }
    for inbox in receivers.statuses.iter_mut() {
        for message in inbox.receive() {
            runtime.apply_statuses(
                message
                    .statuses
                    .into_iter()
                    .map(|entry| (entry.npc, entry.status)),
            );
        }
    }
    if log_changed {
        status.entries = runtime
            .log
            .iter()
            .cloned()
            .map(crate::networking_messages::map_quest_entry)
            .collect();
        status.watched_quest_ids = runtime.watched.clone();
    }
}

#[derive(SystemParam)]
struct QuestDialogReceivers<'w, 's> {
    opened: MessageReceivers<'w, 's, InteractionOpened>,
    failed: MessageReceivers<'w, 's, InteractionFailed>,
    closed: MessageReceivers<'w, 's, InteractionClosed>,
    lists: MessageReceivers<'w, 's, QuestGiverQuestList>,
    details: MessageReceivers<'w, 's, QuestGiverQuestDetails>,
    requests: MessageReceivers<'w, 's, QuestGiverRequestItems>,
    offers: MessageReceivers<'w, 's, QuestGiverOfferReward>,
    completes: MessageReceivers<'w, 's, QuestGiverQuestComplete>,
    quest_failed: MessageReceivers<'w, 's, QuestFailed>,
}

/// Server entity bits → NPC name and flags on the mirrored entity.
#[derive(SystemParam)]
struct NpcLookup<'w, 's> {
    mirror: Res<'w, ReplicationMirrorMap>,
    npcs: Query<'w, 's, (&'static Npc, Option<&'static NpcFlags>)>,
}

impl NpcLookup<'_, '_> {
    fn npc(&self, bits: u64) -> Option<(&Npc, Option<&NpcFlags>)> {
        let server = Entity::try_from_bits(bits)?;
        let main = self.mirror.server_to_main(server)?;
        self.npcs.get(main).ok()
    }

    fn name(&self, bits: u64) -> String {
        self.npc(bits)
            .map(|(npc, _)| npc.name.clone())
            .unwrap_or_default()
    }

    fn is_quest_giver(&self, bits: u64) -> bool {
        self.npc(bits)
            .and_then(|(_, flags)| flags)
            .is_some_and(|flags| flags.contains(NpcFlags::QUESTGIVER))
    }
}

fn receive_quest_dialog(
    mut receivers: QuestDialogReceivers,
    npcs: NpcLookup,
    mut runtime: ResMut<QuestRuntime>,
    mut requests: MessageWriter<NpcInteractionRequest>,
    mut frames: MessageWriter<NpcFrameEvent>,
    mut chat: ResMut<ChatState>,
    mut errors: ResMut<UiErrors>,
) {
    receive_interactions(
        &mut receivers,
        &npcs,
        &mut runtime,
        (&mut requests, &mut frames),
        &mut errors,
    );
    // A turn-in answers QuestGiverQuestComplete before the chain's next
    // QuestGiverQuestDetails; one batch can carry both.
    for inbox in receivers.completes.iter_mut() {
        for complete in inbox.receive() {
            let notices = runtime.complete_quest(&complete);
            post_notices(notices, &mut chat);
        }
    }
    for inbox in receivers.lists.iter_mut() {
        for list in inbox.receive() {
            runtime.apply_quest_list(list);
        }
    }
    for inbox in receivers.details.iter_mut() {
        for details in inbox.receive() {
            let name = npcs.name(details.npc);
            runtime.show_details(name, details);
        }
    }
    for inbox in receivers.requests.iter_mut() {
        for request in inbox.receive() {
            let name = npcs.name(request.npc);
            runtime.show_progress(name, request);
        }
    }
    for inbox in receivers.offers.iter_mut() {
        for offer in inbox.receive() {
            let name = npcs.name(offer.npc);
            runtime.show_reward(name, offer);
        }
    }
    for inbox in receivers.quest_failed.iter_mut() {
        for failed in inbox.receive() {
            errors.add(quest_failed_text(failed.reason));
        }
    }
}

/// `InteractionOpened` opens the greeting (and asks a quest giver for its quests) or,
/// for another role, tells that role's frame; failures show the Retail error text;
/// `InteractionClosed` closes the frame.
fn receive_interactions(
    receivers: &mut QuestDialogReceivers,
    npcs: &NpcLookup,
    runtime: &mut QuestRuntime,
    (requests, frames): (
        &mut MessageWriter<NpcInteractionRequest>,
        &mut MessageWriter<NpcFrameEvent>,
    ),
    errors: &mut UiErrors,
) {
    for inbox in receivers.opened.iter_mut() {
        for opened in inbox.receive() {
            let name = npcs.name(opened.npc);
            match opened.kind {
                InteractionKind::Gossip(menu) => {
                    runtime.open_gossip(opened.npc, name, menu);
                    if npcs.is_quest_giver(opened.npc) {
                        requests.write(NpcInteractionRequest::Hello { npc: opened.npc });
                    }
                }
                InteractionKind::Role(NpcRole::QuestGiver) => {
                    runtime.open_quest_giver(opened.npc, name);
                    requests.write(NpcInteractionRequest::Hello { npc: opened.npc });
                }
                InteractionKind::Role(role) => {
                    // A role frame replaces the gossip greeting it was picked from.
                    runtime.close_dialog_for(opened.npc);
                    frames.write(NpcFrameEvent::Opened {
                        npc: opened.npc,
                        role,
                    });
                }
            }
        }
    }
    for inbox in receivers.failed.iter_mut() {
        for failed in inbox.receive() {
            errors.add(failed.error.message());
        }
    }
    for inbox in receivers.closed.iter_mut() {
        for closed in inbox.receive() {
            runtime.close_dialog_for(closed.npc);
            frames.write(NpcFrameEvent::Closed { npc: closed.npc });
        }
    }
}

fn post_notices(notices: Vec<String>, chat: &mut ChatState) {
    for text in notices {
        chat.add_message(ChatMessage {
            channel_type: ChatChannelType::System,
            channel_name: String::new(),
            sender: String::new(),
            text,
            timestamp: game_engine::chat_data::now_timestamp(),
        });
    }
}

/// Asks for the marker of every quest giver that appears; the server then pushes
/// changes for them. Queued until connected.
fn query_new_quest_givers(
    added: Query<(Entity, &NpcFlags), Added<NpcFlags>>,
    mirror: Res<ReplicationMirrorMap>,
    mut pending: Local<Vec<u64>>,
    mut senders: MessageSenders<QuestGiverStatusQuery>,
) {
    pending.extend(
        added
            .iter()
            .filter(|(_, flags)| flags.contains(NpcFlags::QUESTGIVER))
            .filter_map(|(entity, _)| mirror.main_to_server(entity))
            .map(Entity::to_bits),
    );
    if pending.is_empty() || senders.is_empty() {
        return;
    }
    let npcs = std::mem::take(&mut *pending);
    for mut sender in senders.iter_mut() {
        sender.send::<QuestChannel>(QuestGiverStatusQuery { npcs: npcs.clone() });
    }
}

#[derive(SystemParam)]
struct InteractionSenders<'w, 's> {
    interact: MessageSenders<'w, 's, InteractNpc>,
    use_object: MessageSenders<'w, 's, UseGameObject>,
    gossip: MessageSenders<'w, 's, SelectGossipOption>,
    close: MessageSenders<'w, 's, CloseInteraction>,
    hello: MessageSenders<'w, 's, QuestGiverHello>,
    query: MessageSenders<'w, 's, QuestGiverQueryQuest>,
    accept: MessageSenders<'w, 's, QuestGiverAcceptQuest>,
    complete: MessageSenders<'w, 's, QuestGiverCompleteQuest>,
    choose: MessageSenders<'w, 's, QuestGiverChooseReward>,
    abandon: MessageSenders<'w, 's, AbandonQuest>,
    watch: MessageSenders<'w, 's, SetQuestWatched>,
}

fn send_interaction_requests(
    mut requests: MessageReader<NpcInteractionRequest>,
    mirror: Res<ReplicationMirrorMap>,
    mut senders: InteractionSenders,
) {
    for request in requests.read() {
        send_request(request, &mirror, &mut senders);
    }
}

fn send_request(
    request: &NpcInteractionRequest,
    mirror: &ReplicationMirrorMap,
    senders: &mut InteractionSenders,
) {
    use NpcInteractionRequest as R;
    match *request {
        R::Interact(entity) => {
            let Some(server) = mirror.main_to_server(entity) else {
                warn!("right-clicked NPC {entity} has no server entity");
                return;
            };
            send::<InteractionChannel, _>(
                &mut senders.interact,
                InteractNpc {
                    npc: server.to_bits(),
                },
            );
        }
        R::UseObject(entity) => {
            let Some(server) = mirror.main_to_server(entity) else {
                warn!("right-clicked game object {entity} has no server entity");
                return;
            };
            send::<InteractionChannel, _>(
                &mut senders.use_object,
                UseGameObject {
                    object: server.to_bits(),
                },
            );
        }
        R::SelectGossip { npc, option_id } => send::<InteractionChannel, _>(
            &mut senders.gossip,
            SelectGossipOption { npc, option_id },
        ),
        R::Close { npc } => {
            send::<InteractionChannel, _>(&mut senders.close, CloseInteraction { npc })
        }
        R::Hello { npc } => send::<QuestChannel, _>(&mut senders.hello, QuestGiverHello { npc }),
        R::QueryQuest { npc, quest_id } => {
            send::<QuestChannel, _>(&mut senders.query, QuestGiverQueryQuest { npc, quest_id })
        }
        R::Accept { npc, quest_id } => {
            send::<QuestChannel, _>(&mut senders.accept, QuestGiverAcceptQuest { npc, quest_id })
        }
        R::Complete { npc, quest_id } => send::<QuestChannel, _>(
            &mut senders.complete,
            QuestGiverCompleteQuest { npc, quest_id },
        ),
        R::ChooseReward {
            npc,
            quest_id,
            choice,
        } => send::<QuestChannel, _>(
            &mut senders.choose,
            QuestGiverChooseReward {
                npc,
                quest_id,
                choice_index: choice,
            },
        ),
        R::Abandon { quest_id } => {
            send::<QuestChannel, _>(&mut senders.abandon, AbandonQuest { quest_id })
        }
        R::SetWatched { quest_id, watched } => {
            send::<QuestChannel, _>(&mut senders.watch, SetQuestWatched { quest_id, watched })
        }
    }
}

fn send<C: Channel, M: NetworkMessage + Clone>(senders: &mut MessageSenders<M>, message: M) {
    for mut sender in senders.iter_mut() {
        sender.send::<C>(message.clone());
    }
}

/// Retail quest giver marker for a server status. Trivial (low-level) quests are
/// hidden, as with Retail's default "Trivial Quests" tracking off.
pub fn quest_indicator(status: QuestGiverStatus) -> QuestIndicator {
    match status {
        QuestGiverStatus::None | QuestGiverStatus::Trivial(_) => QuestIndicator::None,
        QuestGiverStatus::Future(_) => QuestIndicator::Unavailable,
        QuestGiverStatus::Incomplete(_) => QuestIndicator::Incomplete,
        QuestGiverStatus::Available(_) => QuestIndicator::Available,
        QuestGiverStatus::Reward(_) => QuestIndicator::TurnIn,
    }
}

/// Keeps each quest giver's `NpcQuestIndicator` equal to its server marker.
fn sync_quest_indicators(
    mut commands: Commands,
    runtime: Res<QuestRuntime>,
    mirror: Res<ReplicationMirrorMap>,
    givers: Query<(Entity, &NpcFlags, Option<&NpcQuestIndicator>)>,
) {
    for (entity, flags, current) in &givers {
        if !flags.contains(NpcFlags::QUESTGIVER) {
            continue;
        }
        let status = mirror
            .main_to_server(entity)
            .and_then(|server| runtime.giver_status.get(&server.to_bits()))
            .copied()
            .unwrap_or(QuestGiverStatus::None);
        let indicator = quest_indicator(status);
        if current.map(|current| current.0) != Some(indicator) {
            commands.entity(entity).insert(NpcQuestIndicator(indicator));
        }
    }
}

fn reset_quest_runtime(mut runtime: ResMut<QuestRuntime>) {
    *runtime = QuestRuntime::default();
}

#[cfg(test)]
#[path = "quests_tests.rs"]
mod tests;
