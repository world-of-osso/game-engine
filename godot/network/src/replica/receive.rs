//! Worker half: lightyear transport to raw replicon payloads, plus mutate acknowledgments.
//! Replaces `lightyear_replication::client::receive_client_packets` and replicon's
//! `ClientPlugin`; keeps no component data.

use bevy::prelude::*;
use bevy_replicon::{bytes::Bytes, postcard_utils, prelude::RepliconTick};
use lightyear::prelude::{Client, Transport};
use lightyear_replication::{channels::RepliconChannelMap, checkpoint::unwrap_server_payload};
use lightyear_transport::channel::{receivers::ChannelReceive, registry::ChannelId};

use super::{MUTATE_INDEX_SIZE, ReplicationBatch};

/// Drain replicon's update and mutation channels of every client transport. Mutate messages
/// are acknowledged in the frame they arrive, as replicon's client does before applying.
pub(crate) fn receive_batches(
    channels: &RepliconChannelMap,
    transports: &mut Query<&mut Transport, With<Client>>,
) -> Result<Vec<ReplicationBatch>, BevyError> {
    let (_, updates_channel) = channels.server_channels[0];
    let (_, mutations_channel) = channels.server_channels[1];
    let (acks_kind, _) = channels.client_channels[0];
    let mut batches = Vec::new();
    for mut transport in transports.iter_mut() {
        let updates = drain_channel(&mut transport, updates_channel)?;
        let mut mutations = drain_channel(&mut transport, mutations_channel)?;
        if !mutations.is_empty() {
            let mut acks = Vec::with_capacity(MUTATE_INDEX_SIZE * mutations.len());
            for message in &mutations {
                acks.extend_from_slice(mutate_header(message)?.index);
            }
            transport.send_mut_erased(acks_kind, acks.into(), 1.0)?;
            // With tracking on the server sends one every tick; empty ones change nothing.
            mutations
                .retain(|message| mutate_header(message).is_ok_and(|header| header.has_entities));
        }
        if !updates.is_empty() || !mutations.is_empty() {
            batches.push(ReplicationBatch { updates, mutations });
        }
    }
    Ok(batches)
}

fn drain_channel(transport: &mut Transport, channel: ChannelId) -> Result<Vec<Bytes>, BevyError> {
    let mut messages = Vec::new();
    if let Some(receiver) = transport.receivers.get_mut(&channel) {
        while let Some((_, message, _)) = receiver.receiver.read_message() {
            let (_, inner) = unwrap_server_payload(message)
                .map_err(|error| format!("malformed replicon server payload: {error:?}"))?;
            messages.push(inner);
        }
    }
    Ok(messages)
}

struct MutateHeader<'a> {
    /// The fixint `MutateIndex` bytes the server expects back.
    index: &'a [u8],
    has_entities: bool,
}

fn mutate_header(message: &Bytes) -> Result<MutateHeader<'_>, BevyError> {
    let mut header = &message[..];
    let _update_tick: RepliconTick = postcard_utils::from_buf(&mut header)?;
    let _message_tick: RepliconTick = postcard_utils::from_buf(&mut header)?;
    let _messages_count: usize = postcard_utils::from_buf(&mut header)?;
    if header.len() < MUTATE_INDEX_SIZE {
        return Err("mutate message ends before its index".into());
    }
    Ok(MutateHeader {
        index: &header[..MUTATE_INDEX_SIZE],
        has_entities: header.len() > MUTATE_INDEX_SIZE,
    })
}

/// Every `MutateIndex` of `mutations`, concatenated: replicon's `MutationAcks` message.
#[cfg(test)]
pub(crate) fn acknowledgments(mutations: &[Bytes]) -> Result<Bytes, BevyError> {
    let mut acks = Vec::new();
    for message in mutations {
        acks.extend_from_slice(mutate_header(message)?.index);
    }
    Ok(acks.into())
}
