use std::collections::VecDeque;
use std::sync::mpsc;

use bevy::ecs::system::SystemParam;
use bevy::prelude::*;
use lightyear::prelude::{Message as NetworkMessage, MessageReceiver, MessageSender};
use shared::protocol::{
    CollectionChannel, CollectionStateUpdate, DismissMount, DismissPet, SummonMount, SummonPet,
};

use crate::ipc::{Request, Response};
use crate::status::{CollectionMountEntry, CollectionPetEntry, CollectionStatusSnapshot};

#[derive(Resource, Default)]
pub struct CollectionRuntimeState {
    pending_actions: VecDeque<Action>,
    pending_replies: VecDeque<mpsc::Sender<Response>>,
}

enum Action {
    SummonMount { mount_id: u32 },
    DismissMount,
    SummonPet { pet_id: u32 },
    DismissPet,
}

pub struct CollectionPlugin;

impl Plugin for CollectionPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<CollectionRuntimeState>();
        app.add_systems(Update, send_pending_actions);
        app.add_systems(Update, receive_collection_updates);
    }
}

pub fn queue_ipc_request(
    runtime: &mut CollectionRuntimeState,
    request: &Request,
    respond: mpsc::Sender<Response>,
) -> bool {
    let action = match request {
        Request::CollectionSummonMount { mount_id } => Action::SummonMount {
            mount_id: *mount_id,
        },
        Request::CollectionDismissMount => Action::DismissMount,
        Request::CollectionSummonPet { pet_id } => Action::SummonPet { pet_id: *pet_id },
        Request::CollectionDismissPet => Action::DismissPet,
        _ => return false,
    };
    runtime.pending_actions.push_back(action);
    runtime.pending_replies.push_back(respond);
    true
}

#[derive(SystemParam)]
struct CollectionSenders<'w, 's> {
    summon_mount: Query<'w, 's, &'static mut MessageSender<SummonMount>>,
    dismiss_mount: Query<'w, 's, &'static mut MessageSender<DismissMount>>,
    summon_pet: Query<'w, 's, &'static mut MessageSender<SummonPet>>,
    dismiss_pet: Query<'w, 's, &'static mut MessageSender<DismissPet>>,
}

fn send_pending_actions(
    mut runtime: ResMut<CollectionRuntimeState>,
    mut senders: CollectionSenders,
) {
    while let Some(action) = runtime.pending_actions.pop_front() {
        let sent = match action {
            Action::SummonMount { mount_id } => {
                send_all(&mut senders.summon_mount, SummonMount { mount_id })
            }
            Action::DismissMount => send_all(&mut senders.dismiss_mount, DismissMount),
            Action::SummonPet { pet_id } => send_all(&mut senders.summon_pet, SummonPet { pet_id }),
            Action::DismissPet => send_all(&mut senders.dismiss_pet, DismissPet),
        };
        if !sent && let Some(reply) = runtime.pending_replies.pop_front() {
            let _ = reply.send(Response::Error(
                "collections are unavailable: not connected".into(),
            ));
        }
    }
}

fn send_all<T: Clone + NetworkMessage>(
    senders: &mut Query<&mut MessageSender<T>>,
    message: T,
) -> bool {
    let mut sent = false;
    for mut sender in senders.iter_mut() {
        sender.send::<CollectionChannel>(message.clone());
        sent = true;
    }
    sent
}

fn receive_collection_updates(
    mut runtime: ResMut<CollectionRuntimeState>,
    mut snapshot: ResMut<CollectionStatusSnapshot>,
    mut receivers: Query<&mut MessageReceiver<CollectionStateUpdate>>,
) {
    for mut receiver in receivers.iter_mut() {
        for update in receiver.receive() {
            apply_collection_state_update(&mut snapshot, update);
            if let Some(reply) = runtime.pending_replies.pop_front() {
                let response = if let Some(error) = &snapshot.last_error {
                    Response::Error(error.clone())
                } else {
                    Response::Text(
                        snapshot
                            .last_server_message
                            .clone()
                            .unwrap_or_else(|| "collection updated".into()),
                    )
                };
                let _ = reply.send(response);
            }
        }
    }
}

pub fn apply_collection_state_update(
    snapshot: &mut CollectionStatusSnapshot,
    update: CollectionStateUpdate,
) {
    if let Some(collection_snapshot) = update.snapshot {
        snapshot.mounts = collection_snapshot
            .mounts
            .into_iter()
            .map(|mount| CollectionMountEntry {
                mount_id: mount.mount_id,
                name: mount.name,
                known: mount.known,
                active: mount.active,
            })
            .collect();
        snapshot.pets = collection_snapshot
            .pets
            .into_iter()
            .map(|pet| CollectionPetEntry {
                pet_id: pet.pet_id,
                name: pet.name,
                known: pet.known,
                active: pet.active,
            })
            .collect();
    }
    snapshot.last_server_message = update.message;
    snapshot.last_error = update.error;
}

pub fn reset_runtime(runtime: &mut CollectionRuntimeState) {
    *runtime = CollectionRuntimeState::default();
}
