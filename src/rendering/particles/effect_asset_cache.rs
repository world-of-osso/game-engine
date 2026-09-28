//! One `EffectAsset` per distinct emitter definition.
//!
//! bevy_hanabi gives every `EffectAsset` handle its own particle slab of at least
//! `ParticleSlab::MIN_CAPACITY` (65536) particles, while instances of one asset share
//! slabs. An asset per emitter entity made every Stormwind torch and lamp cost ~4 MiB
//! of VRAM, so emitters with identical build inputs reuse one asset.

use std::collections::HashMap;

use bevy::asset::AssetId;
use bevy::prelude::*;
use bevy_hanabi::EffectAsset;

use crate::asset::m2_particle::M2ParticleEmitter;

use super::effect_builder::build_effect_asset_with_mode;
use super::{ParticleSpawnMode, ParticleSpawnSource};

/// Every input of `build_effect_asset_with_mode`.
#[derive(PartialEq)]
pub(super) struct EffectAssetKey {
    pub(super) emitter: M2ParticleEmitter,
    pub(super) model_scale: f32,
    pub(super) particle_density_multiplier: f32,
    pub(super) spawn_mode: ParticleSpawnMode,
    pub(super) spawn_source: ParticleSpawnSource,
    pub(super) child_emitters: Vec<M2ParticleEmitter>,
}

/// Cheap hashable subset of the key; full equality is checked within a bucket.
#[derive(PartialEq, Eq, Hash)]
struct Bucket {
    flags: u32,
    texture_fdid: Option<u32>,
    model_scale: u32,
    spawn_mode: ParticleSpawnMode,
    spawn_source: ParticleSpawnSource,
}

impl Bucket {
    fn of(key: &EffectAssetKey) -> Self {
        Self {
            flags: key.emitter.flags,
            texture_fdid: key.emitter.texture_fdid,
            model_scale: key.model_scale.to_bits(),
            spawn_mode: key.spawn_mode,
            spawn_source: key.spawn_source,
        }
    }
}

#[derive(Default)]
pub(crate) struct EffectAssetCache {
    buckets: HashMap<Bucket, Vec<(EffectAssetKey, AssetId<EffectAsset>)>>,
}

impl EffectAssetCache {
    /// Handle of the live asset built from `key`, building and caching it on a miss.
    pub(super) fn handle(
        &mut self,
        key: EffectAssetKey,
        effects: &mut Assets<EffectAsset>,
    ) -> Handle<EffectAsset> {
        let entries = self.buckets.entry(Bucket::of(&key)).or_default();
        entries.retain(|(_, id)| effects.contains(*id));
        if let Some(handle) = entries
            .iter()
            .find(|(cached, _)| *cached == key)
            .and_then(|(_, id)| effects.get_strong_handle(*id))
        {
            return handle;
        }
        let handle = effects.add(build_effect_asset_with_mode(
            &key.emitter,
            key.model_scale,
            key.particle_density_multiplier,
            key.spawn_mode,
            key.spawn_source,
            &key.child_emitters,
        ));
        entries.push((key, handle.id()));
        handle
    }
}
