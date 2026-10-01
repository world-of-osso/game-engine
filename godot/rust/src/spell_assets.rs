//! Spell kit models and sound files, loaded off the main thread: workers extract them
//! from local CASC, parse the M2 and decode its BLP textures; the main thread turns
//! arrivals into textures within a per-frame budget. Nothing waits: a kit whose asset
//! is still loading starts it on arrival (see `spell_effects`).

use std::collections::{HashMap, VecDeque};
use std::path::{Path, PathBuf};
use std::rc::Rc;
use std::sync::Arc;
use std::time::{Duration, Instant};

use game_engine_core::asset_loader::{AssetLoader, Priority};
use game_engine_core::spell_visual::{KitSound, SpellVisualCatalog, VisualEvent};
use game_engine_core::{blp, m2};
use godot::prelude::*;
use osso_asset_resolver::CascListfileResolver;

use crate::assets::creature::{cache_model_files, cache_model_textures, local_resolver};
use crate::assets::material::insert_shared_texture;
use crate::assets::read_model_file;
use crate::particles::ModelParticles;

/// Two workers: one long extraction (a cold CASC read) does not hold up the next asset.
const WORKERS: usize = 2;
/// Main-thread time per frame for turning arrived models into textures; at least one
/// model is finished each frame.
const FRAME_BUDGET: Duration = Duration::from_millis(4);

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub(crate) enum SpellAsset {
    Model(u32),
    Sound(u32),
}

/// A worker's result.
pub(crate) enum Loaded {
    Model(ModelFiles),
    /// Ogg Vorbis bytes.
    Sound(Vec<u8>),
}

/// A kit model extracted, parsed and with its textures decoded.
pub(crate) struct ModelFiles {
    path: PathBuf,
    model: m2::Model,
    textures: Vec<(u32, blp::GpuImage)>,
}

/// A parsed kit model, reused by every kit that attaches it.
pub(crate) struct EffectModel {
    pub path: GString,
    pub model: m2::Model,
    pub particles: Option<Rc<ModelParticles>>,
}

pub(crate) struct SpellAssets {
    loader: AssetLoader<SpellAsset, Loaded>,
    texture_dir: PathBuf,
    /// Arrived models waiting for their main-thread textures.
    arrived: VecDeque<(u32, ModelFiles)>,
    models: HashMap<u32, Result<Rc<EffectModel>, String>>,
    sounds: HashMap<u32, Result<Rc<[u8]>, String>>,
}

impl SpellAssets {
    pub fn new(data_root: PathBuf) -> Self {
        let resolver = Arc::new(local_resolver(&data_root));
        let texture_dir = data_root.join("textures");
        let loader =
            AssetLoader::new(
                "spell-assets",
                WORKERS,
                move |&asset: &SpellAsset| match asset {
                    SpellAsset::Model(fdid) => load_model(&resolver, &data_root, fdid)
                        .map(Loaded::Model)
                        .map_err(|error| format!("Spell effect model {fdid}: {error}")),
                    SpellAsset::Sound(fdid) => {
                        load_sound(&resolver, &data_root, fdid).map(Loaded::Sound)
                    }
                },
            );
        Self {
            loader,
            texture_dir,
            arrived: VecDeque::new(),
            models: HashMap::new(),
            sounds: HashMap::new(),
        }
    }

    /// Start loading `asset` unless it was requested before; a prefetch still queued
    /// moves ahead when it is needed `Now`.
    pub fn request(&mut self, asset: SpellAsset, priority: Priority) {
        self.loader.request(asset, priority);
    }

    /// Kit model `fdid`: `None` while it loads, else loaded or its error.
    pub fn model(&self, fdid: u32) -> Option<Result<Rc<EffectModel>, String>> {
        self.models.get(&fdid).cloned()
    }

    /// Sound file `fdid`'s Ogg bytes: `None` while it loads, else loaded or its error.
    pub fn sound(&self, fdid: u32) -> Option<Result<Rc<[u8]>, String>> {
        self.sounds.get(&fdid).cloned()
    }

    /// Assets requested and not yet usable.
    pub fn pending(&self) -> usize {
        self.loader.loading() + self.arrived.len()
    }

    /// Collect finished loads (each failure is logged once) and finish arrived models
    /// within the frame budget.
    pub fn update(&mut self) {
        for (asset, loaded) in self.loader.poll() {
            match (asset, loaded) {
                (SpellAsset::Model(fdid), Ok(Loaded::Model(files))) => {
                    self.arrived.push_back((fdid, files));
                }
                (SpellAsset::Sound(fdid), Ok(Loaded::Sound(bytes))) => {
                    self.sounds.insert(fdid, Ok(bytes.into()));
                }
                (SpellAsset::Model(fdid), Err(error)) => {
                    godot_error!("{error}");
                    self.models.insert(fdid, Err(error));
                }
                (SpellAsset::Sound(fdid), Err(error)) => {
                    godot_error!("{error}");
                    self.sounds.insert(fdid, Err(error));
                }
                (asset, Ok(_)) => unreachable!("{asset:?} loaded as another asset kind"),
            }
        }
        let started = Instant::now();
        while let Some((fdid, files)) = self.arrived.pop_front() {
            let model = self.finish_model(fdid, files);
            if let Err(error) = &model {
                godot_error!("{error}");
            }
            self.models.insert(fdid, model);
            if started.elapsed() >= FRAME_BUDGET {
                break;
            }
        }
    }

    /// Main thread: the model's textures and particles.
    fn finish_model(&self, fdid: u32, files: ModelFiles) -> Result<Rc<EffectModel>, String> {
        for (texture, image) in files.textures {
            insert_shared_texture(texture, &self.texture_dir, image)
                .map_err(|error| format!("Spell effect model {fdid}: {error}"))?;
        }
        let particles = ModelParticles::from_model(fdid, &files.model);
        Ok(Rc::new(EffectModel {
            path: GString::from(files.path.to_string_lossy().as_ref()),
            model: files.model,
            particles,
        }))
    }
}

/// The events whose kits play (`SpellEffects`, `auras`).
const PLAYED_EVENTS: [VisualEvent; 6] = [
    VisualEvent::PrecastStart,
    VisualEvent::ChannelStart,
    VisualEvent::Cast,
    VisualEvent::Impact,
    VisualEvent::AuraStart,
    VisualEvent::AuraEnd,
];

/// Every model and sound file visual `visual`'s played kits and missile use.
pub(crate) fn kit_assets(catalog: &SpellVisualCatalog, visual: u32) -> Vec<SpellAsset> {
    let kits = PLAYED_EVENTS
        .iter()
        .flat_map(|&event| catalog.kits(visual, event));
    let mut assets = Vec::new();
    for kit in kits {
        assets.extend(
            kit.models
                .iter()
                .map(|model| SpellAsset::Model(model.model_fdid)),
        );
        assets.extend(kit.sounds.iter().flat_map(sound_assets));
    }
    if let Some(missile) = catalog.missile(visual) {
        assets.push(SpellAsset::Model(missile.model_fdid));
        assets.extend(missile.sound.iter().flat_map(sound_assets));
    }
    assets
}

/// The files `sound` can pick (frequency above 0).
fn sound_assets(sound: &KitSound) -> impl Iterator<Item = SpellAsset> + '_ {
    sound
        .files
        .iter()
        .filter(|file| file.frequency > 0)
        .map(|file| SpellAsset::Sound(file.fdid))
}

/// Worker: extract model `fdid` and its companions and textures, parse it and decode
/// the textures it has (a texture missing from local CASC stays the material loader's
/// reported missing FDID).
fn load_model(
    resolver: &CascListfileResolver,
    data_root: &Path,
    fdid: u32,
) -> Result<ModelFiles, String> {
    let path = cache_model_files(resolver, data_root, fdid)?;
    let model = read_model_file(&path)?;
    let textures = cache_model_textures(resolver, data_root, &[0; 3], &model)?
        .into_iter()
        .filter_map(|texture| {
            let file = data_root.join("textures").join(format!("{texture}.blp"));
            let bytes = std::fs::read(&file).ok()?;
            Some(
                blp::decode_gpu(&bytes)
                    .map(|image| (texture, image))
                    .map_err(|error| format!("Texture {texture}: {error}")),
            )
        })
        .collect::<Result<Vec<_>, String>>()?;
    Ok(ModelFiles {
        path,
        model,
        textures,
    })
}

/// Worker: local-CASC Ogg `fdid`, cached at `data/sounds/spells/{fdid}.ogg`.
fn load_sound(
    resolver: &CascListfileResolver,
    data_root: &Path,
    fdid: u32,
) -> Result<Vec<u8>, String> {
    let destination = data_root.join("sounds/spells").join(format!("{fdid}.ogg"));
    let path = resolver
        .ensure_cached(fdid, &destination)
        .ok_or_else(|| format!("Spell sound {fdid}: not in local CASC"))?;
    let bytes =
        std::fs::read(&path).map_err(|error| format!("Spell sound {}: {error}", path.display()))?;
    if !bytes.starts_with(b"OggS") {
        return Err(format!("Spell sound {}: not Ogg data", path.display()));
    }
    Ok(bytes)
}
