//! Local-only creature model companions and textures for the native M2 loader.
use std::{
    collections::BTreeSet,
    fs,
    path::{Path, PathBuf},
};

use game_engine_core::{creature_display_data::CreatureDisplay, m2};
use godot::{classes::Node3D, prelude::*};
use osso_asset_resolver::{AssetResolverConfig, CascListfileResolver};

use super::{
    appearance::PreparedAppearance,
    build_model,
    equipment::{attach_each_equipment, place_equipment},
    read_model,
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

pub(crate) fn load_creature_model(
    data_root: &Path,
    display: &CreatureDisplay,
    appearance: Option<&PreparedAppearance>,
    gear: &CreatureGear,
) -> Result<(Gd<Node3D>, PackedInt32Array), String> {
    let resolver = local_resolver(data_root);
    let path = cache_model_files(&resolver, data_root, display.model_fdid)?;
    let path = GString::from(path.to_string_lossy().as_ref());
    let parsed = read_model(&path)?;
    cache_model_textures(&resolver, data_root, &display.skin_fdids, &parsed)?;
    let (mut model, missing) = build_model(&parsed, &path, &display.skin_fdids, appearance)?;
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
        attach_each_equipment(&mut model, &parsed, &resolver, data_root, &models, report)
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

pub(crate) fn local_resolver(data_root: &Path) -> CascListfileResolver {
    CascListfileResolver::new(
        AssetResolverConfig::new()
            .with_data_root(data_root)
            .with_shared_data_root(data_root),
    )
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
        if resolver.ensure_cached(fdid, &destination).is_none() {
            godot_error!(
                "{}: .anim FDID {fdid} not extractable to {}",
                path.display(),
                destination.display()
            );
        }
    }
    Ok(path)
}

fn cache_required(
    resolver: &CascListfileResolver,
    fdid: u32,
    destination: &Path,
) -> Result<PathBuf, String> {
    resolver.ensure_cached(fdid, destination).ok_or_else(|| {
        format!(
            "Failed to cache local CASC FDID {fdid} at {}",
            destination.display()
        )
    })
}

/// Cache the local-CASC textures an already parsed model's batches and particles
/// reference; returns their FDIDs.
pub(crate) fn cache_model_textures(
    resolver: &CascListfileResolver,
    data_root: &Path,
    skin_fdids: &[u32; 3],
    parsed: &m2::Model,
) -> Result<BTreeSet<u32>, String> {
    let textures = creature_texture_fdids(resolver, parsed, skin_fdids)?;
    for &fdid in &textures {
        let path = data_root.join("textures").join(format!("{fdid}.blp"));
        // Missing textures remain the native material loader's reported missing FDIDs.
        resolver.ensure_cached(fdid, &path);
    }
    Ok(textures)
}

fn creature_texture_fdids(
    resolver: &CascListfileResolver,
    model: &m2::Model,
    slots: &[u32; 3],
) -> Result<BTreeSet<u32>, String> {
    let batches =
        m2::resolve_render_batches(model, slots, false, |fdid| resolver.resolve_path(fdid))?;
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
            model_fdid: 126278,
            skin_fdids: [126280, 0, 0],
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
    fn acquires_cached_external_skeleton_from_skid() {
        let data_root = cached_data_root();
        let resolver = local_resolver(&data_root);
        let display = CreatureDisplay {
            model_fdid: 1011653,
            skin_fdids: [0; 3],
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
