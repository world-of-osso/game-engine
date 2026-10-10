//! Local-only creature model companions and textures for the native M2 loader.
use std::{
    collections::{BTreeSet, HashMap, HashSet},
    fs,
    path::{Path, PathBuf},
    sync::{Arc, LazyLock, Mutex},
};

use game_engine_core::{blp, creature_display_data::CreatureDisplay, m2};
use godot::{classes::Node3D, prelude::*};
use osso_asset_resolver::{AssetResolverConfig, CascListfileResolver};

use super::{
    appearance::PreparedAppearance,
    build_model,
    equipment::{attach_each_equipment, place_equipment},
    material::insert_shared_texture,
    read_model_file,
};
use crate::equipment_appearance_data::{RuntimeModelAppearance, model_attachment_id};

/// What a creature model holds: its display's armor item models and its virtual items.
#[derive(Default)]
pub(crate) struct CreatureGear {
    /// Armor item models, at their slots' default attachments.
    pub(crate) armor_models: Vec<RuntimeModelAppearance>,
    /// Virtual item models, each with the attachment its sheath state places it on
    /// (`None`: not shown).
    pub(crate) items: Vec<(RuntimeModelAppearance, Option<u32>)>,
}

/// A creature display's model parsed and its textures decoded, off the main thread.
pub(crate) struct CreatureModelParts {
    pub(crate) model: Arc<CachedModel>,
    pub(crate) textures: DecodedTextures,
}

/// A file's complete namespace survives worker decode and GPU publication.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub(crate) struct TextureAddress {
    pub(crate) dir: PathBuf,
    pub(crate) fdid: u32,
}

pub(crate) type DecodedTextures = Vec<(TextureAddress, blp::GpuImage)>;

/// Worker: extract and parse the display's model and its armor and item models, and
/// decode their textures, so `build_creature_model` does no file work.
pub(crate) fn prepare_creature_model(
    resolver: &CascListfileResolver,
    data_root: &Path,
    display: &CreatureDisplay,
    gear: &CreatureGear,
) -> Result<CreatureModelParts, String> {
    let model = load_model_files(resolver, data_root, display.model_fdid)?;
    let fdids = cache_model_textures(resolver, data_root, &display.skin_fdids, &model.model)?;
    let mut textures = decode_new_textures(model_asset_root(&model)?, &fdids)?;
    let items = gear
        .armor_models
        .iter()
        .chain(gear.items.iter().map(|(item, _)| item));
    for item in items {
        // An item that cannot load is reported when it is attached.
        if let Ok(parts) = load_model_files(resolver, data_root, item.fdid)
            && let Ok(item_fdids) =
                cache_model_textures(resolver, data_root, &item.skin_fdids, &parts.model)
        {
            let decoded = decode_new_textures(model_asset_root(&parts)?, &item_fdids)?;
            textures.extend(decoded);
        }
    }
    Ok(CreatureModelParts { model, textures })
}

/// Main thread: the nodes of the display's `model` with its armor and held items; their
/// decoded textures are inserted first (`insert_decoded_textures`).
pub(crate) fn build_creature_model(
    data_root: &Path,
    display: &CreatureDisplay,
    model: &CachedModel,
    appearance: Option<&PreparedAppearance>,
    gear: &CreatureGear,
) -> Result<(Gd<Node3D>, PackedInt32Array), String> {
    let resolver = local_resolver(data_root);
    let parsed = &model.model;
    let path = GString::from(model.path.to_string_lossy().as_ref());
    let (mut model, missing) = build_model(parsed, &path, &display.skin_fdids, appearance)?;
    let items = held_items(&model, &resolver, &gear.items);
    let models: Vec<_> = gear
        .armor_models
        .iter()
        .chain(items.iter().map(|(item, _)| item))
        .cloned()
        .collect();
    let report = |item: &RuntimeModelAppearance, error: String| {
        godot_error!(
            "Creature model {}: {:?} item FDID {}: {error}",
            display.model_fdid,
            item.slot,
            item.fdid
        );
    };
    if let Err(error) =
        attach_each_equipment(&mut model, parsed, &resolver, data_root, &models, report)
    {
        model.free();
        return Err(error);
    }
    for (item, attachment) in items {
        if let Err(error) = place_equipment(&model, item.slot, *attachment) {
            godot_error!("Creature model {}: {error}", display.model_fdid);
        }
    }
    Ok((model, missing))
}

/// The virtual items `model` can hold: it has the attachment of the item's slot
/// (`model_attachment_id`). A transformed unit keeps its virtual items while it shows a
/// creature model without hands (Polymorph's sheep), which then holds none (inferred
/// from the retail client, where a sheep shows no weapons).
fn held_items<'a>(
    model: &Gd<Node3D>,
    resolver: &CascListfileResolver,
    items: &'a [(RuntimeModelAppearance, Option<u32>)],
) -> Vec<&'a (RuntimeModelAppearance, Option<u32>)> {
    items
        .iter()
        .filter(|(item, _)| {
            let Some(authored) = resolver.resolve_path(item.fdid) else {
                // Attaching reports the unresolved path.
                return true;
            };
            let id = model_attachment_id(item.slot, Path::new(&authored));
            model.has_node(&format!("Skeleton3D/AttachmentBone{id}/Attachment{id}"))
        })
        .collect()
}

pub(crate) fn authored_model_resolver(
    data_root: &Path,
    product: game_engine_core::asset_product::AssetProduct,
    fdid: u32,
) -> Result<CascListfileResolver, String> {
    authored_asset_resolver(data_root, product, fdid, "m2")
}

pub(crate) fn authored_asset_resolver(
    data_root: &Path,
    product: game_engine_core::asset_product::AssetProduct,
    fdid: u32,
    kind: &str,
) -> Result<CascListfileResolver, String> {
    let index = load_asset_index(data_root)?;
    let receipt = index.receipt(product, fdid, kind)?;
    let identity = osso_asset_resolver::AssetIdentity::new(product.as_str(), &receipt.build_key)?;
    Ok(CascListfileResolver::new(
        AssetResolverConfig::new()
            .with_data_root(data_root)
            .with_shared_data_root(data_root)
            .with_identity(identity),
    ))
}

fn load_asset_index(
    data_root: &Path,
) -> Result<Arc<game_engine_core::model_asset_index::ModelAssetIndex>, String> {
    use game_engine_core::model_asset_index::ModelAssetIndex;
    static INDEXES: LazyLock<Mutex<HashMap<PathBuf, Arc<ModelAssetIndex>>>> =
        LazyLock::new(|| Mutex::new(HashMap::new()));
    if let Some(index) = INDEXES
        .lock()
        .expect("asset receipt indexes")
        .get(data_root)
        .cloned()
    {
        return Ok(index);
    }
    let index = Arc::new(ModelAssetIndex::load(data_root)?);
    INDEXES
        .lock()
        .expect("asset receipt indexes")
        .insert(data_root.to_owned(), Arc::clone(&index));
    Ok(index)
}

pub(crate) fn local_resolver(data_root: &Path) -> CascListfileResolver {
    CascListfileResolver::new(
        AssetResolverConfig::new()
            .with_data_root(data_root)
            .with_shared_data_root(data_root),
    )
}

/// A model extracted from local CASC with its companions and parsed.
pub(crate) struct CachedModel {
    pub(crate) path: PathBuf,
    pub(crate) model: m2::Model,
}

pub(crate) fn model_asset_root(model: &CachedModel) -> Result<&Path, String> {
    model
        .path
        .parent()
        .and_then(Path::parent)
        .ok_or_else(|| format!("Model {} has no asset root", model.path.display()))
}

/// Parsed models by `.m2` path: every unit, item and doodad of a model shares one parse.
static MODELS: LazyLock<Mutex<HashMap<PathBuf, Arc<CachedModel>>>> =
    LazyLock::new(|| Mutex::new(HashMap::new()));

/// Parsed models held by the process-wide cache.
pub(crate) fn model_cache_entries() -> usize {
    MODELS.lock().expect("model cache").len()
}

/// Model `fdid`, extracted and parsed once per process (workers load it ahead of the
/// main thread); a failure is not kept, so the next use tries again.
pub(crate) fn load_model_files(
    resolver: &CascListfileResolver,
    data_root: &Path,
    fdid: u32,
) -> Result<Arc<CachedModel>, String> {
    let destination = data_root.join("models").join(format!("{fdid}.m2"));
    let key = resolver.cache_path(&destination)?;
    if let Some(cached) = MODELS.lock().expect("model cache").get(&key) {
        return Ok(Arc::clone(cached));
    }
    let io = crate::profile::span(|| "phase.asset_io.m2_cache".to_owned());
    let path = cache_model_files(resolver, data_root, fdid)?;
    drop(io);
    let model = read_model_file(&path)?;
    let cached = Arc::new(CachedModel { path, model });
    MODELS
        .lock()
        .expect("model cache")
        .insert(key, Arc::clone(&cached));
    Ok(cached)
}

/// Model `fdid` if a worker already parsed it (`load_model_files`), without file work.
pub(crate) fn cached_model(data_root: &Path, fdid: u32) -> Option<Arc<CachedModel>> {
    let key = data_root.join("models").join(format!("{fdid}.m2"));
    MODELS.lock().expect("model cache").get(&key).cloned()
}

/// Only successfully published images suppress a later worker decode. A failed
/// prepare or decode cannot mark an image whose pixels never reached the GPU.
static PUBLISHED_TEXTURES: LazyLock<Mutex<HashSet<TextureAddress>>> =
    LazyLock::new(|| Mutex::new(HashSet::new()));

pub(crate) fn decode_new_textures(
    data_root: &Path,
    fdids: &BTreeSet<u32>,
) -> Result<DecodedTextures, String> {
    let fresh: Vec<TextureAddress> = {
        let published = PUBLISHED_TEXTURES.lock().expect("published textures");
        fdids
            .iter()
            .map(|&fdid| TextureAddress {
                dir: data_root.join("textures"),
                fdid,
            })
            .filter(|address| !published.contains(address))
            .collect()
    };
    fresh.into_iter().map(decode_texture_file).collect()
}

fn decode_texture_file(address: TextureAddress) -> Result<(TextureAddress, blp::GpuImage), String> {
    let file = address.dir.join(format!("{}.blp", address.fdid));
    let io = crate::profile::span(|| "phase.asset_io.blp".to_owned());
    let bytes = fs::read(&file)
        .map_err(|error| format!("Texture {} at {}: {error}", address.fdid, file.display()))?;
    drop(io);
    let _decode = crate::profile::span(|| "phase.blp_decode.worker".to_owned());
    let image = blp::decode_gpu(&bytes)
        .map_err(|error| format!("Texture {} at {}: {error}", address.fdid, file.display()))?;
    Ok((address, image))
}

/// Main thread: retain the worker's namespace, never replace it with a caller's root.
pub(crate) fn publish_decoded_texture(
    address: TextureAddress,
    image: blp::GpuImage,
) -> Result<(), String> {
    insert_shared_texture(address.fdid, &address.dir, image)?;
    PUBLISHED_TEXTURES
        .lock()
        .expect("published textures")
        .insert(address);
    Ok(())
}

pub(crate) fn insert_decoded_textures(textures: DecodedTextures) -> Result<(), String> {
    for (address, image) in textures {
        publish_decoded_texture(address, image)?;
    }
    Ok(())
}

pub(crate) fn cache_model_files(
    resolver: &CascListfileResolver,
    data_root: &Path,
    model_fdid: u32,
) -> Result<PathBuf, String> {
    let models = data_root.join("models");
    let model_path = models.join(format!("{model_fdid}.m2"));
    let path = cache_required(resolver, model_fdid, &model_path)?;
    let bytes = fs::read(&path).map_err(|error| format!("{}: {error}", path.display()))?;
    let references = m2::parse_asset_references(&bytes)
        .map_err(|error| format!("{}: {error}", path.display()))?;
    let skin_fdid = references
        .skin_fdids
        .first()
        .copied()
        .ok_or_else(|| format!("{}: no primary SFID", path.display()))?;
    cache_required(
        resolver,
        skin_fdid,
        &models.join(format!("{model_fdid}00.skin")),
    )?;
    let skeleton = match references.skeleton_fdid {
        Some(skeleton_fdid) => {
            let skel_path = cache_required(
                resolver,
                skeleton_fdid,
                &models.join(format!("{model_fdid}.skel")),
            )?;
            Some(
                fs::read(&skel_path)
                    .map_err(|error| format!("{}: {error}", skel_path.display()))?,
            )
        }
        None => None,
    };
    let anim_fdids = m2::external_anim_fdids(&bytes, skeleton.as_deref())
        .map_err(|error| format!("{}: {error}", path.display()))?;
    // A `.anim` that can't be extracted leaves its sequence without keyframes (read_model).
    for fdid in anim_fdids {
        let destination = models.join(format!("{fdid}.anim"));
        if resolver.ensure_cached(fdid, &destination)?.is_none() {
            godot_error!(
                "{}: .anim FDID {fdid} not extractable to {}",
                path.display(),
                destination.display()
            );
        }
    }
    Ok(path)
}

pub(super) fn cache_required(
    resolver: &CascListfileResolver,
    fdid: u32,
    destination: &Path,
) -> Result<PathBuf, String> {
    resolver.ensure_cached_checked(fdid, destination)
}

/// Cache the local-CASC textures an already parsed model's batches and particles
/// reference; returns their FDIDs.
pub(crate) fn cache_model_textures(
    resolver: &CascListfileResolver,
    data_root: &Path,
    skin_fdids: &[u32],
    parsed: &m2::Model,
) -> Result<BTreeSet<u32>, String> {
    let textures = creature_texture_fdids(resolver, parsed, skin_fdids)?;
    for &fdid in &textures {
        let path = data_root.join("textures").join(format!("{fdid}.blp"));
        // Local-CASC misses remain optional; shipped misses fail the worker load.
        resolver.ensure_cached(fdid, &path)?;
    }
    Ok(textures)
}

fn creature_texture_fdids(
    resolver: &CascListfileResolver,
    model: &m2::Model,
    slots: &[u32],
) -> Result<BTreeSet<u32>, String> {
    // The same batches the native material binds, zero-opacity ones included.
    let batches =
        m2::resolve_render_batches(model, slots, true, |fdid| resolver.resolve_path(fdid))?;
    let mut textures = BTreeSet::from_iter(slots.iter().copied().filter(|fdid| *fdid != 0));
    for batch in batches {
        textures.extend(batch.texture_fdid);
        textures.extend(batch.texture_2_fdid);
        textures.extend(batch.extra_texture_fdids);
        textures.extend(batch.overlays.into_iter().map(|overlay| overlay.fdid));
    }
    textures.extend(crate::particles::ModelParticles::texture_fdids(model));
    Ok(textures)
}

#[cfg(test)]
#[path = "creature_isolation_tests.rs"]
mod isolation_tests;

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    fn cached_data_root() -> PathBuf {
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../data")
    }

    #[test]
    fn acquires_cached_creature_skin_from_sfid_not_creature_texture_slots() {
        let data_root = cached_data_root();
        let resolver = local_resolver(&data_root);
        let display = CreatureDisplay {
            source_product: game_engine_core::asset_product::AssetProduct::Retail,
            model_fdid: 126278,
            skin_fdids: [126280, 0, 0, 0],
            scale_milli: 1000,
        };
        let path = cache_model_files(&resolver, &data_root, display.model_fdid).unwrap();
        assert_eq!(path, data_root.join("models/126278.m2"));
        let model = std::fs::read(&path).unwrap();
        let skin = std::fs::read(data_root.join("models/12627800.skin")).unwrap();
        assert!(
            !game_engine_core::m2::parse_model(&model, &skin)
                .unwrap()
                .submeshes
                .is_empty()
        );
    }

    #[test]
    fn collects_authored_batch_textures_and_explicit_creature_slots() {
        let data_root = cached_data_root();
        let resolver = local_resolver(&data_root);
        let model = std::fs::read(data_root.join("models/126278.m2")).unwrap();
        let skin = std::fs::read(data_root.join("models/12627800.skin")).unwrap();
        let parsed = m2::parse_model(&model, &skin).unwrap();
        let textures = creature_texture_fdids(&resolver, &parsed, &[987654321, 0, 0]).unwrap();
        assert!(textures.contains(&987654321));
        assert!(textures.contains(&126280));
        assert!(!textures.contains(&0));
    }

    #[test]
    fn collects_textures_of_batches_whose_opacity_starts_at_zero() {
        // Instance portal 197012 fades its glowball.blp batch in from zero opacity; the
        // native material still binds that batch's texture.
        let data_root = cached_data_root();
        let resolver = local_resolver(&data_root);
        let model = std::fs::read(data_root.join("models/197012.m2")).unwrap();
        let skin = std::fs::read(data_root.join("models/19701200.skin")).unwrap();
        let parsed = m2::parse_model(&model, &skin).unwrap();
        let textures = creature_texture_fdids(&resolver, &parsed, &[0; 3]).unwrap();
        assert!(textures.contains(&1068808), "{textures:?}");
    }

    #[test]
    fn acquires_cached_external_skeleton_from_skid() {
        let data_root = cached_data_root();
        let resolver = local_resolver(&data_root);
        let display = CreatureDisplay {
            source_product: game_engine_core::asset_product::AssetProduct::Retail,
            model_fdid: 1011653,
            skin_fdids: [0; 4],
            scale_milli: 1000,
        };
        let path = cache_model_files(&resolver, &data_root, display.model_fdid).unwrap();
        let model = std::fs::read(path).unwrap();
        let skin = std::fs::read(data_root.join("models/101165300.skin")).unwrap();
        let skel = std::fs::read(data_root.join("models/1011653.skel")).unwrap();
        assert!(
            !game_engine_core::m2::parse_model_with_skeleton(&model, &skin, Some(&skel), |_| None)
                .unwrap()
                .bones
                .is_empty()
        );
    }
}
