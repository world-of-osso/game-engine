//! Typed decoders for every component replicon may send, matched to the server's `FnsId`s.

use std::{
    any::{Any, TypeId},
    collections::HashMap,
    sync::Arc,
};

use bevy::{ecs::component::ComponentId, platform::hash::NoOpHash, prelude::*};
use bevy_replicon::{
    bytes::Bytes,
    postcard_utils,
    shared::replication::{registry::FnsId, rules::ReplicationRules},
};
use serde::{Serialize, de::DeserializeOwned};
use shared::{
    casting::CastState,
    components::{
        ActiveSpec, CombatRatings, CombatStatus, CreatureClassification, CreatureMotion, DerivedStats,
        EquipmentAppearance, Gold, GuildMembership, Health, Mana, ModelDisplay, Mounted,
        MovementControl, MovementSpeed, Npc, Player, PlayerMotion, PlayerStandState, Position,
        PresenceStatus, Rotation, UnitAuras, UnitFactionTemplate, UnitFlags, UnitLevel, UnitPose,
        UnitPowers, UnitRunes, UnitStats, UnitSummonedBy, UnitTap, UnitTarget, UnitThreatList,
        UnitVignette, WorldArrival, Zone,
    },
    level_scaling::LevelScaling,
    protocol::{GameObjectInfo, NpcFlags},
};

/// Every type the client and server register for replication, in any order. A type the
/// protocol registers but this list lacks fails the connection instead of skipping bytes.
fn codecs() -> Vec<Codec> {
    let mut codecs = lightyear_codecs();
    codecs.extend(shared_codecs());
    codecs
}

/// Lightyear's own replicated markers (`SharedComponentRegistrationPlugin`).
fn lightyear_codecs() -> Vec<Codec> {
    vec![
        Codec::of::<lightyear::prelude::Predicted>(),
        Codec::of::<lightyear::prelude::Interpolated>(),
        Codec::of::<lightyear::prelude::Controlled>(),
        // Lightyear sends the parent's server entity without replicon's mapping.
        Codec::with::<ChildOf>(|data| Ok(ChildOf(postcard_utils::entity_from_buf(data)?))),
    ]
}

/// `shared::ProtocolPlugin`'s replicated components.
fn shared_codecs() -> Vec<Codec> {
    vec![
        Codec::of::<Position>(),
        Codec::of::<Health>(),
        Codec::of::<Mana>(),
        Codec::of::<Gold>(),
        Codec::of::<Player>(),
        Codec::of::<ActiveSpec>(),
        Codec::of::<Npc>(),
        Codec::of::<ModelDisplay>(),
        Codec::of::<Rotation>(),
        Codec::of::<MovementSpeed>(),
        Codec::of::<CombatStatus>(),
        Codec::of::<Mounted>(),
        Codec::of::<MovementControl>(),
        Codec::of::<WorldArrival>(),
        Codec::of::<Zone>(),
        Codec::of::<GuildMembership>(),
        Codec::of::<PresenceStatus>(),
        Codec::of::<EquipmentAppearance>(),
        Codec::of::<CastState>(),
        Codec::of::<UnitPowers>(),
        Codec::of::<UnitAuras>(),
        Codec::of::<UnitLevel>(),
        Codec::of::<LevelScaling>(),
        Codec::of::<UnitFactionTemplate>(),
        Codec::of::<UnitFlags>(),
        Codec::of::<UnitTarget>(),
        Codec::of::<CreatureMotion>(),
        Codec::of::<PlayerMotion>(),
        Codec::of::<UnitPose>(),
        Codec::of::<UnitThreatList>(),
        Codec::of::<UnitTap>(),
        Codec::of::<NpcFlags>(),
        Codec::of::<GameObjectInfo>(),
        Codec::of::<UnitRunes>(),
        Codec::of::<UnitStats>(),
        Codec::of::<CombatRatings>(),
        Codec::of::<DerivedStats>(),
        Codec::of::<PlayerStandState>(),
        Codec::of::<CreatureClassification>(),
        Codec::of::<UnitVignette>(),
        Codec::of::<UnitSummonedBy>(),
    ]
}

type DecodeFn<C> = fn(&mut Bytes) -> Result<C, BevyError>;

/// Decoder and storage factory for one component type.
pub(crate) struct Codec {
    pub(crate) type_id: TypeId,
    pub(crate) name: &'static str,
    new_column: Box<dyn Fn() -> Box<dyn Column> + Send + Sync>,
    #[cfg(test)]
    pub(crate) encode_entity: fn(EntityRef) -> Option<Vec<u8>>,
}

impl Codec {
    /// Replicon's default rule: the component's own postcard encoding.
    fn of<C: Component + Serialize + DeserializeOwned>() -> Self {
        Self::with::<C>(|data| Ok(postcard_utils::from_buf(data)?))
    }

    fn with<C: Component + Serialize>(decode: DecodeFn<C>) -> Self {
        Self {
            type_id: TypeId::of::<C>(),
            name: std::any::type_name::<C>(),
            new_column: Box::new(move || {
                Box::new(TypedColumn::<C> {
                    decode,
                    values: Vec::new(),
                })
            }),
            #[cfg(test)]
            encode_entity: |entity| entity.get::<C>().map(encode::<C>),
        }
    }
}

#[cfg(test)]
fn encode<C: Serialize>(value: &C) -> Vec<u8> {
    let mut bytes = Vec::new();
    postcard_utils::to_extend_mut(value, &mut bytes).expect("encode component");
    bytes
}

/// One component type's values, indexed by unit slot.
pub(crate) trait Column: Any {
    fn write(&mut self, slot: usize, data: &mut Bytes) -> Result<(), BevyError>;
    fn remove(&mut self, slot: usize);
    fn as_any(&self) -> &dyn Any;
    #[cfg(feature = "test-util")]
    fn as_any_mut(&mut self) -> &mut dyn Any;
    #[cfg(test)]
    fn encode(&self, slot: usize) -> Option<Vec<u8>>;
}

pub(crate) struct TypedColumn<C> {
    decode: DecodeFn<C>,
    pub(crate) values: Vec<Option<C>>,
}

impl<C: Component + Serialize> Column for TypedColumn<C> {
    fn write(&mut self, slot: usize, data: &mut Bytes) -> Result<(), BevyError> {
        let value = (self.decode)(data)?;
        if self.values.len() <= slot {
            self.values.resize_with(slot + 1, || None);
        }
        self.values[slot] = Some(value);
        Ok(())
    }

    fn remove(&mut self, slot: usize) {
        if let Some(value) = self.values.get_mut(slot) {
            *value = None;
        }
    }

    fn as_any(&self) -> &dyn Any {
        self
    }

    #[cfg(feature = "test-util")]
    fn as_any_mut(&mut self) -> &mut dyn Any {
        self
    }

    #[cfg(test)]
    fn encode(&self, slot: usize) -> Option<Vec<u8>> {
        self.values.get(slot)?.as_ref().map(encode::<C>)
    }
}

/// The protocol's `FnsId` order resolved to codecs. Shared by every connection of a build.
pub struct Schema {
    pub(crate) codecs: Vec<Codec>,
    /// Codec index for each `FnsId`.
    by_fns: Vec<usize>,
    /// Looked up on every component read; `TypeId` is already a hash.
    by_type: TypeIndex,
}

type TypeIndex = HashMap<TypeId, usize, NoOpHash>;

impl Schema {
    /// Resolve the replication rules the worker registered (the same registrations that
    /// produce the protocol hash) against the codec list.
    pub(crate) fn from_world(world: &World) -> Result<Arc<Self>, String> {
        let codecs = codecs();
        assert!(codecs.len() <= 64, "ComponentSet holds at most 64 codecs");
        let by_type: TypeIndex = codecs
            .iter()
            .enumerate()
            .map(|(index, codec)| (codec.type_id, index))
            .collect();
        let mut by_fns = Vec::new();
        for rule in world.resource::<ReplicationRules>().iter() {
            for component in &rule.components {
                let codec = codec_of(world, component.id, &by_type)?;
                let fns = fns_index(component.fns_id);
                if by_fns.len() <= fns {
                    by_fns.resize(fns + 1, usize::MAX);
                }
                by_fns[fns] = codec;
            }
        }
        if let Some(missing) = by_fns.iter().position(|&codec| codec == usize::MAX) {
            return Err(format!("replication FnsId {missing} has no rule"));
        }
        Ok(Arc::new(Self {
            codecs,
            by_fns,
            by_type,
        }))
    }

    /// No codecs: a host before its first connection.
    pub(crate) fn empty() -> Arc<Self> {
        Arc::new(Self {
            codecs: Vec::new(),
            by_fns: Vec::new(),
            by_type: TypeIndex::default(),
        })
    }

    /// Every codec without wire ids, for host tests that insert values directly.
    #[cfg(feature = "test-util")]
    pub(crate) fn codecs_only() -> Arc<Self> {
        let codecs = codecs();
        let by_type = codecs
            .iter()
            .enumerate()
            .map(|(index, codec)| (codec.type_id, index))
            .collect();
        Arc::new(Self {
            codecs,
            by_fns: Vec::new(),
            by_type,
        })
    }

    pub(crate) fn codec_for_fns(&self, fns: usize) -> Result<usize, BevyError> {
        self.by_fns
            .get(fns)
            .copied()
            .ok_or_else(|| format!("unknown replication FnsId {fns}").into())
    }

    pub(crate) fn index_of<C: 'static>(&self) -> Option<usize> {
        self.by_type.get(&TypeId::of::<C>()).copied()
    }

    pub(crate) fn new_columns(&self) -> Vec<Box<dyn Column>> {
        self.codecs
            .iter()
            .map(|codec| (codec.new_column)())
            .collect()
    }
}

fn codec_of(world: &World, component: ComponentId, by_type: &TypeIndex) -> Result<usize, String> {
    let info = world
        .components()
        .get_info(component)
        .ok_or("replication rule names an unregistered component")?;
    info.type_id()
        .and_then(|type_id| by_type.get(&type_id))
        .copied()
        .ok_or_else(|| {
            format!(
                "no replication codec for `{}`; add it to the client codec list",
                info.name()
            )
        })
}

/// `FnsId` keeps its index private; its serde form is the wire's varint index.
fn fns_index(fns_id: FnsId) -> usize {
    let mut bytes = Vec::new();
    postcard_utils::to_extend_mut(&fns_id, &mut bytes).expect("FnsId serializes");
    postcard_utils::from_buf(&mut bytes.as_slice()).expect("FnsId is a usize")
}
