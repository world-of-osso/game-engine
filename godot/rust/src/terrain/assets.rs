//! Local-CASC map and split-ADT reads for the native world host.

use std::path::{Path, PathBuf};
use std::{cell::RefCell, collections::BTreeMap, fs, sync::Arc};

use game_engine_core::{adt, wdt};
use osso_asset_resolver::{AssetResolverConfig, CascListfileResolver};

use super::textures::{TerrainLayerTextures, TerrainTextureCache};
use crate::lighting::assets::LightingCatalog;
use crate::wmo::placement::PlacedWmo;

pub(crate) struct NativeTerrainAssets {
    resolver: CascListfileResolver,
    terrain_dir: PathBuf,
    data_root: PathBuf,
    textures: RefCell<TerrainTextureCache>,
    lighting: RefCell<Option<Arc<LightingCatalog>>>,
}

pub(crate) struct NativeMapWdt {
    pub path: PathBuf,
    pub flags: wdt::MphdFlags,
    pub global_wmo: Option<PlacedWmo>,
    pub lighting: Arc<LightingCatalog>,
}

pub(crate) struct NativeTerrainTile {
    pub root_path: PathBuf,
    pub tex_path: Option<PathBuf>,
    pub obj_path: Option<PathBuf>,
    pub root: adt::Root,
    pub tex: Option<adt::AdtTexData>,
    pub obj: Option<adt::AdtObjData>,
    pub textures: BTreeMap<u32, TerrainLayerTextures>,
}

impl NativeTerrainAssets {
    pub fn new(data_root: PathBuf, cache_root: PathBuf) -> Self {
        let config = AssetResolverConfig::new()
            .with_data_root(&data_root)
            .with_shared_data_root(&data_root)
            .with_cache_root(cache_root);
        Self {
            resolver: CascListfileResolver::new(config),
            terrain_dir: data_root.join("terrain"),
            data_root,
            textures: RefCell::new(TerrainTextureCache::default()),
            lighting: RefCell::new(None),
        }
    }

    pub fn read_map_wdt(&self, map: &str) -> Result<NativeMapWdt, String> {
        let wow_path = format!("world/maps/{map}/{map}.wdt");
        let (path, bytes) = self.read_declared_file(&wow_path, "wdt")?;
        let flags = wdt::parse_wdt_mphd_flags(&bytes)
            .map_err(|error| format!("{}: {error}", path.display()))?;
        let global_wmo = wdt::parse_wdt_global_wmo(&bytes)
            .map_err(|error| format!("{}: {error}", path.display()))?
            .map(|placement| {
                let asset = crate::wmo::assets::read_placement(
                    &self.resolver,
                    &self.data_root,
                    &placement,
                )?;
                Ok::<_, String>(PlacedWmo::new(placement, asset))
            })
            .transpose()?;
        Ok(NativeMapWdt {
            path,
            flags,
            global_wmo,
            lighting: self.read_lighting_catalog()?,
        })
    }

    fn read_lighting_catalog(&self) -> Result<Arc<LightingCatalog>, String> {
        if let Some(catalog) = self.lighting.borrow().as_ref() {
            return Ok(Arc::clone(catalog));
        }
        let catalog = Arc::new(LightingCatalog::read(&self.data_root)?);
        *self.lighting.borrow_mut() = Some(Arc::clone(&catalog));
        Ok(catalog)
    }

    pub fn read_tile(
        &self,
        map: &str,
        tile_y: u32,
        tile_x: u32,
    ) -> Result<NativeTerrainTile, String> {
        let wdt = self.read_map_wdt(map)?;
        let stem = format!("world/maps/{map}/{map}_{tile_y}_{tile_x}");
        let (root_path, root_bytes) = self.read_declared_file(&format!("{stem}.adt"), "adt")?;
        let tex_file = self.read_optional_companion(&format!("{stem}_tex0.adt"))?;
        let obj_file = self.read_optional_companion(&format!("{stem}_obj0.adt"))?;
        let root = adt::parse_root_for_tile(
            &root_bytes,
            tile_y,
            tile_x,
            tex_file.as_ref().map(|(_, bytes)| bytes.as_slice()),
        )
        .map_err(|error| format!("{}: {error}", root_path.display()))?;
        let tex = tex_file
            .as_ref()
            .map(|(path, bytes)| {
                adt::parse_tex(bytes, wdt.flags, &root)
                    .map_err(|error| format!("{}: {error}", path.display()))
            })
            .transpose()?;
        let obj = obj_file
            .as_ref()
            .map(|(path, bytes)| {
                adt::parse_obj(bytes).map_err(|error| format!("{}: {error}", path.display()))
            })
            .transpose()?;
        let textures = match &tex {
            Some(tex) => {
                self.textures
                    .borrow_mut()
                    .load_for_tile(&self.resolver, &self.data_root, tex)?
            }
            None => BTreeMap::new(),
        };
        Ok(NativeTerrainTile {
            root_path,
            tex_path: tex_file.map(|(path, _)| path),
            obj_path: obj_file.map(|(path, _)| path),
            root,
            tex,
            obj,
            textures,
        })
    }

    fn read_optional_companion(
        &self,
        wow_path: &str,
    ) -> Result<Option<(PathBuf, Vec<u8>)>, String> {
        if self.resolver.lookup_path(wow_path).is_none() {
            return Ok(None);
        }
        self.read_declared_file(wow_path, "adt").map(Some)
    }

    fn read_declared_file(
        &self,
        wow_path: &str,
        extension: &str,
    ) -> Result<(PathBuf, Vec<u8>), String> {
        let fdid = self
            .resolver
            .lookup_path(wow_path)
            .ok_or_else(|| format!("{wow_path} not in listfile"))?;
        let cache_path = self.terrain_dir.join(format!("{fdid}.{extension}"));
        let path = self
            .resolver
            .ensure_cached(fdid, &cache_path)
            .ok_or_else(|| {
                format!(
                    "Failed to cache local CASC {wow_path} (FDID {fdid}) at {}",
                    cache_path.display()
                )
            })?;
        let bytes = read_bytes(&path)?;
        Ok((path, bytes))
    }
}

fn read_bytes(path: &Path) -> Result<Vec<u8>, String> {
    fs::read(path).map_err(|error| format!("{}: {error}", path.display()))
}

/// Repository `data/` plus the user's local-CASC resolver cache.
#[cfg(test)]
pub(crate) fn cached_assets() -> NativeTerrainAssets {
    let cache_root = std::env::var_os("XDG_CACHE_HOME")
        .map(PathBuf::from)
        .or_else(|| std::env::var_os("HOME").map(|home| PathBuf::from(home).join(".cache")))
        .expect("cache location")
        .join("asset-resolver");
    NativeTerrainAssets::new(test_data_root(), cache_root)
}

#[cfg(test)]
pub(crate) fn test_data_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../data")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reads_cached_map_flags_and_global_wmo_placement() {
        let assets = cached_assets();
        let azeroth = assets.read_map_wdt("azeroth").expect("cached Azeroth WDT");
        assert_eq!(azeroth.path.file_name().unwrap(), "775971.wdt");
        assert!(azeroth.global_wmo.is_none());

        let stockade = assets
            .read_map_wdt("stormwindjail")
            .expect("cached Stockade WDT");
        assert_eq!(stockade.path.file_name().unwrap(), "791060.wdt");
        let wmo = stockade.global_wmo.expect("loaded global WMO payload");
        assert_eq!(wmo.placement.fdid, Some(108_631));
        assert_eq!(wmo.placement.position, [0.0, 0.0, 0.0]);
        assert_eq!(stockade.flags.raw & 1, 1);
        let asset = wmo.asset;
        assert_eq!(asset.root_fdid, 108_631);
        assert_eq!(asset.root.n_groups, 27);
        assert_eq!(asset.groups.len(), 27);
        assert!(asset.groups.iter().any(|group| !group.batches.is_empty()));
    }

    #[test]
    fn reads_cached_tile_with_authored_shadows_textures_and_objects() {
        let tile = cached_assets()
            .read_tile("azeroth", 32, 48)
            .expect("cached complete tile");
        assert_eq!(tile.root_path.file_name().unwrap(), "778027.adt");
        assert_eq!(
            tile.tex_path.as_ref().unwrap().file_name().unwrap(),
            "778030.adt"
        );
        assert_eq!(
            tile.obj_path.as_ref().unwrap().file_name().unwrap(),
            "778028.adt"
        );
        assert_eq!(tile.root.chunks.len(), 256);
        assert!(
            tile.tex
                .as_ref()
                .is_some_and(|tex| !tex.chunk_layers.is_empty())
        );
        assert!(tile.obj.is_some());
    }

    #[test]
    fn missing_map_and_tile_are_errors_not_empty_assets() {
        let assets = cached_assets();
        let map_error = assets
            .read_map_wdt("map_that_does_not_exist_999")
            .err()
            .unwrap();
        assert!(
            map_error.contains("map_that_does_not_exist_999.wdt"),
            "{map_error}"
        );
        assert!(map_error.contains("not in listfile"), "{map_error}");

        let tile_error = assets.read_tile("azeroth", 64, 64).err().unwrap();
        assert!(tile_error.contains("azeroth_64_64.adt"), "{tile_error}");
        assert!(tile_error.contains("not in listfile"), "{tile_error}");
    }
}
