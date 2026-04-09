use std::collections::HashMap;
use std::hash::Hash;
use std::sync::Mutex;

use bevy::asset::AssetId;
use bevy::prelude::{Asset, Assets, Handle};

pub(crate) fn lookup_cached_asset_handle<K, A>(
    cache: &Mutex<HashMap<K, AssetId<A>>>,
    key: &K,
    assets: &mut Assets<A>,
) -> Option<Handle<A>>
where
    K: Eq + Hash,
    A: Asset,
{
    let cached_id = {
        let cache = cache.lock().unwrap();
        cache.get(key).copied()
    }?;
    if let Some(handle) = assets.get_strong_handle(cached_id) {
        return Some(handle);
    }
    cache.lock().unwrap().remove(key);
    None
}

pub(crate) fn lookup_cached_result_asset_handle<K, A>(
    cache: &Mutex<HashMap<K, Result<AssetId<A>, String>>>,
    key: &K,
    assets: &mut Assets<A>,
) -> Option<Result<Handle<A>, String>>
where
    K: Eq + Hash,
    A: Asset,
{
    let cached = {
        let cache = cache.lock().unwrap();
        cache.get(key).cloned()
    }?;
    match cached {
        Ok(asset_id) => {
            if let Some(handle) = assets.get_strong_handle(asset_id) {
                Some(Ok(handle))
            } else {
                cache.lock().unwrap().remove(key);
                None
            }
        }
        Err(err) => Some(Err(err)),
    }
}

pub(crate) fn prune_unused_asset_handles<K, A>(
    cache: &Mutex<HashMap<K, AssetId<A>>>,
    assets: &Assets<A>,
) -> usize
where
    K: Eq + Hash,
    A: Asset,
{
    let mut cache = cache.lock().unwrap();
    let before = cache.len();
    cache.retain(|_, asset_id| assets.get(*asset_id).is_some());
    before - cache.len()
}

pub(crate) fn prune_unused_result_asset_handles<K, A>(
    cache: &Mutex<HashMap<K, Result<AssetId<A>, String>>>,
    assets: &Assets<A>,
) -> usize
where
    K: Eq + Hash,
    A: Asset,
{
    let mut cache = cache.lock().unwrap();
    let before = cache.len();
    cache.retain(|_, cached| match cached {
        Ok(asset_id) => assets.get(*asset_id).is_some(),
        Err(_) => true,
    });
    before - cache.len()
}

#[cfg(test)]
mod tests {
    use super::*;
    use bevy::image::Image;
    use bevy::prelude::*;

    fn image_test_app() -> App {
        let mut app = App::new();
        app.add_plugins(MinimalPlugins);
        app.add_plugins(bevy::asset::AssetPlugin::default());
        app.init_asset::<Image>();
        app
    }

    #[test]
    fn lookup_cached_asset_handle_drops_stale_entry() {
        let mut app = image_test_app();
        let handle = app
            .world_mut()
            .resource_mut::<Assets<Image>>()
            .add(Image::default());
        let id = handle.id();
        let cache = Mutex::new(HashMap::from([("alive", id)]));

        drop(handle);
        app.update();

        let cached = lookup_cached_asset_handle(
            &cache,
            &"alive",
            &mut app.world_mut().resource_mut::<Assets<Image>>(),
        );
        assert!(cached.is_none());
        assert!(cache.lock().unwrap().is_empty());
    }

    #[test]
    fn lookup_cached_result_asset_handle_rebuilds_live_strong_handle() {
        let mut app = image_test_app();
        let handle = app
            .world_mut()
            .resource_mut::<Assets<Image>>()
            .add(Image::default());
        let id = handle.id();
        let cache = Mutex::new(HashMap::from([("alive", Ok(id))]));

        let cached = lookup_cached_result_asset_handle(
            &cache,
            &"alive",
            &mut app.world_mut().resource_mut::<Assets<Image>>(),
        )
        .expect("cached handle should exist")
        .expect("cached handle should be valid");

        assert!(cached.is_strong());
        assert_eq!(cached.id(), id);
    }
}
