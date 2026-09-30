//! Client side of replicon 0.41.1 replication without an ECS replica.
//!
//! The worker forwards replicon's raw update and mutate payloads and acknowledges mutate
//! messages (`receive`). `Replica`, owned by the Godot host, applies them the way
//! `bevy_replicon::client::receive_replication` does and is the client's only copy of
//! replicated component values.

mod codec;
pub(crate) mod receive;

use std::sync::Arc;

use bevy::{platform::collections::HashMap, prelude::BevyError};
use bevy_replicon::{bytes::Buf, bytes::Bytes, postcard_utils, prelude::RepliconTick};

pub use codec::Schema;
use codec::{Column, TypedColumn};

/// Raw replicon payloads the worker received in one frame, in arrival order, with the
/// Lightyear checkpoint header already removed.
pub struct ReplicationBatch {
    pub(crate) updates: Vec<Bytes>,
    pub(crate) mutations: Vec<Bytes>,
}

/// Codec indices of components written or removed since the last `drain_changes`.
#[derive(Clone, Copy, Default, Debug, PartialEq, Eq)]
pub struct ComponentSet(u64);

impl ComponentSet {
    fn insert(&mut self, codec: usize) {
        self.0 |= 1 << codec;
    }

    pub fn is_empty(self) -> bool {
        self.0 == 0
    }
}

#[derive(Debug, PartialEq, Eq)]
pub enum UnitChange {
    /// Components of a replicated entity were inserted, mutated or removed.
    Changed {
        server_id: u64,
        components: ComponentSet,
    },
    /// The server despawned the entity or it left this client's visibility.
    Despawned(u64),
}

struct Entry {
    slot: usize,
    /// Newest tick applied to the entity (replicon's `ConfirmHistory::last_tick`).
    last_tick: RepliconTick,
    changed: ComponentSet,
    /// Listed in `Replica::dirty`.
    queued: bool,
}

/// A mutate message waiting for its update tick, like replicon's `BufferedMutate`.
struct BufferedMutate {
    update_tick: RepliconTick,
    message_tick: RepliconTick,
    data: Bytes,
}

/// Replicated entities keyed by server `Entity::to_bits()`, components stored per type.
pub struct Replica {
    schema: Arc<Schema>,
    columns: Vec<Box<dyn Column>>,
    entities: HashMap<u64, Entry>,
    free_slots: Vec<usize>,
    next_slot: usize,
    /// Newest update message tick (replicon's `ServerUpdateTick`).
    update_tick: RepliconTick,
    /// Sorted by message tick, newest first.
    buffered: Vec<BufferedMutate>,
    changes: Vec<UnitChange>,
    /// Entities with a pending `Changed`, in first-change order.
    dirty: Vec<u64>,
}

/// Read access to one replicated entity.
#[derive(Clone, Copy)]
pub struct Unit<'a> {
    replica: &'a Replica,
    slot: usize,
    pub server_id: u64,
}

impl<'a> Unit<'a> {
    pub fn get<C: 'static>(&self) -> Option<&'a C> {
        let codec = self.replica.schema.index_of::<C>()?;
        let column = self.replica.columns[codec]
            .as_any()
            .downcast_ref::<TypedColumn<C>>()
            .expect("column matches its codec type");
        column.values.get(self.slot)?.as_ref()
    }

    pub fn has<C: 'static>(&self) -> bool {
        self.get::<C>().is_some()
    }
}

impl Default for Replica {
    fn default() -> Self {
        Self::new(Schema::empty())
    }
}

impl Replica {
    pub fn new(schema: Arc<Schema>) -> Self {
        Self {
            columns: schema.new_columns(),
            schema,
            entities: HashMap::new(),
            free_slots: Vec::new(),
            next_slot: 0,
            update_tick: RepliconTick::default(),
            buffered: Vec::new(),
            changes: Vec::new(),
            dirty: Vec::new(),
        }
    }

    pub fn unit(&self, server_id: u64) -> Option<Unit<'_>> {
        let entry = self.entities.get(&server_id)?;
        Some(self.view(server_id, entry))
    }

    pub fn units(&self) -> impl Iterator<Item = Unit<'_>> {
        self.entities
            .iter()
            .map(|(&server_id, entry)| self.view(server_id, entry))
    }

    pub fn len(&self) -> usize {
        self.entities.len()
    }

    pub fn is_empty(&self) -> bool {
        self.entities.is_empty()
    }

    /// Whether `components` includes `C`.
    pub fn changed<C: 'static>(&self, components: ComponentSet) -> bool {
        self.schema
            .index_of::<C>()
            .is_some_and(|codec| components.0 & (1 << codec) != 0)
    }

    fn view(&self, server_id: u64, entry: &Entry) -> Unit<'_> {
        Unit {
            replica: self,
            slot: entry.slot,
            server_id,
        }
    }

    /// Forget every entity (the connection ended); each becomes a `Despawned` change.
    pub fn clear(&mut self) {
        let ids: Vec<u64> = self.entities.keys().copied().collect();
        for id in ids {
            self.despawn(id);
        }
        self.update_tick = RepliconTick::default();
        self.buffered.clear();
    }

    /// Apply one worker frame: update messages first, then every mutate message whose
    /// update tick has arrived. A malformed message is an error; replicon logs and drops it.
    pub fn apply(&mut self, batch: ReplicationBatch) -> Result<(), BevyError> {
        for message in batch.updates {
            self.apply_update(message)?;
        }
        for message in batch.mutations {
            self.buffer_mutate(message)?;
        }
        self.apply_buffered()
    }

    /// Changes since the previous call, in order.
    pub fn drain_changes(&mut self) -> Vec<UnitChange> {
        for id in self.dirty.drain(..) {
            // Visibility regain can queue the same server ID again before this drain.
            // Consume its current incarnation once, including when an older queue entry exists.
            if let Some(entry) = self.entities.get_mut(&id)
                && entry.queued
            {
                entry.queued = false;
                let components = std::mem::take(&mut entry.changed);
                self.changes.push(UnitChange::Changed {
                    server_id: id,
                    components,
                });
            }
        }
        std::mem::take(&mut self.changes)
    }

    fn apply_update(&mut self, mut message: Bytes) -> Result<(), BevyError> {
        let flags: u8 = postcard_utils::from_buf(&mut message)?;
        if flags == 0 || flags & !(MAPPINGS | DESPAWNS | REMOVALS | CHANGES) != 0 {
            return Err(format!("invalid replication update flags {flags:#04x}").into());
        }
        let tick: RepliconTick = postcard_utils::from_buf(&mut message)?;
        self.update_tick = tick;
        let last_flag = 1 << (u8::BITS - 1 - flags.leading_zeros());
        for flag in [MAPPINGS, DESPAWNS, REMOVALS, CHANGES] {
            if flags & flag == 0 {
                continue;
            }
            let sized = flag != last_flag;
            let len = if sized {
                Some(postcard_utils::from_buf::<usize, _>(&mut message)?)
            } else {
                None
            };
            let mut index = 0;
            while len.map_or(message.has_remaining(), |len| index < len) {
                match flag {
                    MAPPINGS => skip_mapping(&mut message)?,
                    DESPAWNS => {
                        let entity = postcard_utils::entity_from_buf(&mut message)?;
                        self.despawn(entity.to_bits());
                    }
                    REMOVALS => self.apply_removals(&mut message, tick)?,
                    _ => self.apply_changes(&mut message, tick)?,
                }
                index += 1;
            }
        }
        Ok(())
    }

    fn apply_removals(&mut self, message: &mut Bytes, tick: RepliconTick) -> Result<(), BevyError> {
        let id = postcard_utils::entity_from_buf(message)?.to_bits();
        let mut data = split_entity_data(message)?;
        let entry = self
            .entities
            .get_mut(&id)
            .ok_or_else(|| format!("received removal for unknown server entity {id:#x}"))?;
        entry.last_tick = tick;
        let slot = entry.slot;
        while data.has_remaining() {
            let codec = self.read_codec(&mut data)?;
            self.columns[codec].remove(slot);
            self.mark(id, codec);
        }
        Ok(())
    }

    fn apply_changes(&mut self, message: &mut Bytes, tick: RepliconTick) -> Result<(), BevyError> {
        let id = postcard_utils::entity_from_buf(message)?.to_bits();
        let mut data = split_entity_data(message)?;
        let slot = match self.entities.get_mut(&id) {
            Some(entry) => {
                entry.last_tick = tick;
                entry.slot
            }
            None => self.spawn(id, tick),
        };
        self.write_components(id, slot, &mut data)
    }

    fn buffer_mutate(&mut self, mut message: Bytes) -> Result<(), BevyError> {
        let update_tick = postcard_utils::from_buf(&mut message)?;
        let message_tick = postcard_utils::from_buf(&mut message)?;
        // The server tracks mutate messages (lightyear enables it), so a count and the
        // acknowledged index precede the entities; the worker already acknowledged it.
        let _messages_count: usize = postcard_utils::from_buf(&mut message)?;
        advance(&mut message, MUTATE_INDEX_SIZE)?;
        let mutate = BufferedMutate {
            update_tick,
            message_tick,
            data: message,
        };
        let index = self
            .buffered
            .partition_point(|other| mutate.message_tick < other.message_tick);
        self.buffered.insert(index, mutate);
        Ok(())
    }

    fn apply_buffered(&mut self) -> Result<(), BevyError> {
        let mut index = 0;
        while index < self.buffered.len() {
            if self.buffered[index].update_tick > self.update_tick {
                index += 1;
                continue;
            }
            let mut mutate = self.buffered.remove(index);
            while mutate.data.has_remaining() {
                self.apply_mutations(&mut mutate.data, mutate.message_tick)?;
            }
        }
        Ok(())
    }

    fn apply_mutations(
        &mut self,
        message: &mut Bytes,
        tick: RepliconTick,
    ) -> Result<(), BevyError> {
        let id = postcard_utils::entity_from_buf(message)?.to_bits();
        let mut data = split_entity_data(message)?;
        // A mutation can arrive after the update message that despawned its entity.
        let Some(entry) = self.entities.get_mut(&id) else {
            return Ok(());
        };
        if tick <= entry.last_tick {
            return Ok(());
        }
        entry.last_tick = tick;
        let slot = entry.slot;
        self.write_components(id, slot, &mut data)
    }

    fn write_components(
        &mut self,
        id: u64,
        slot: usize,
        data: &mut Bytes,
    ) -> Result<(), BevyError> {
        while data.has_remaining() {
            let codec = self.read_codec(data)?;
            self.columns[codec]
                .write(slot, data)
                .map_err(|error| format!("decode {}: {error}", self.schema.codecs[codec].name))?;
            self.mark(id, codec);
        }
        Ok(())
    }

    fn read_codec(&self, data: &mut Bytes) -> Result<usize, BevyError> {
        let fns: usize = postcard_utils::from_buf(data)?;
        self.schema.codec_for_fns(fns)
    }

    fn spawn(&mut self, id: u64, tick: RepliconTick) -> usize {
        let slot = self.free_slots.pop().unwrap_or_else(|| {
            self.next_slot += 1;
            self.next_slot - 1
        });
        self.entities.insert(
            id,
            Entry {
                slot,
                last_tick: tick,
                changed: ComponentSet::default(),
                queued: true,
            },
        );
        self.dirty.push(id);
        slot
    }

    fn despawn(&mut self, id: u64) {
        let Some(entry) = self.entities.remove(&id) else {
            return;
        };
        for column in &mut self.columns {
            column.remove(entry.slot);
        }
        self.free_slots.push(entry.slot);
        self.changes.push(UnitChange::Despawned(id));
    }

    fn mark(&mut self, id: u64, codec: usize) {
        let entry = self.entities.get_mut(&id).expect("marked entity exists");
        if !entry.queued {
            entry.queued = true;
            self.dirty.push(id);
        }
        entry.changed.insert(codec);
    }

    /// Store `value` as if replicated; host tests build units this way.
    #[cfg(feature = "test-util")]
    pub fn for_tests() -> Self {
        Self::new(Schema::codecs_only())
    }

    #[cfg(feature = "test-util")]
    pub fn insert<C: bevy::prelude::Component + serde::Serialize>(
        &mut self,
        server_id: u64,
        value: C,
    ) {
        let codec = self.schema.index_of::<C>().expect("replicated type");
        let slot = match self.entities.get(&server_id) {
            Some(entry) => entry.slot,
            None => self.spawn(server_id, RepliconTick::default()),
        };
        let column = self.columns[codec]
            .as_any_mut()
            .downcast_mut::<TypedColumn<C>>()
            .expect("column matches its codec type");
        if column.values.len() <= slot {
            column.values.resize_with(slot + 1, || None);
        }
        column.values[slot] = Some(value);
        self.mark(server_id, codec);
    }

    #[cfg(feature = "test-util")]
    pub fn remove<C: 'static>(&mut self, server_id: u64) {
        let codec = self.schema.index_of::<C>().expect("replicated type");
        let slot = self.entities[&server_id].slot;
        self.columns[codec].remove(slot);
        self.mark(server_id, codec);
    }

    #[cfg(test)]
    pub(crate) fn encoded(&self, server_id: u64, codec: usize) -> Option<Vec<u8>> {
        let entry = self.entities.get(&server_id)?;
        self.columns[codec].encode(entry.slot)
    }

    #[cfg(test)]
    pub(crate) fn schema(&self) -> &Schema {
        &self.schema
    }
}

// `UpdateMessageFlags` bits.
const MAPPINGS: u8 = 0b0001;
const DESPAWNS: u8 = 0b0010;
const REMOVALS: u8 = 0b0100;
const CHANGES: u8 = 0b1000;

/// `MutateIndex` is a fixint little-endian u16.
pub(crate) const MUTATE_INDEX_SIZE: usize = 2;

/// Signature mappings pair a server entity with a client-spawned one; this client spawns
/// none, so every hash is unknown and replicon skips it the same way.
fn skip_mapping(message: &mut Bytes) -> Result<(), BevyError> {
    postcard_utils::entity_from_buf(message)?;
    advance(message, size_of::<u64>())
}

fn split_entity_data(message: &mut Bytes) -> Result<Bytes, BevyError> {
    let size: usize = postcard_utils::from_buf(message)?;
    if message.remaining() < size {
        return Err("entity data exceeds message".into());
    }
    Ok(message.split_to(size))
}

fn advance(message: &mut Bytes, count: usize) -> Result<(), BevyError> {
    if message.remaining() < count {
        return Err("replication message ends early".into());
    }
    message.advance(count);
    Ok(())
}

#[cfg(test)]
mod tests;
