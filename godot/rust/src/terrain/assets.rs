//! Local-CASC map and split-ADT reads for the native world host.

use std::path::{Path, PathBuf};
use std::{
    cell::RefCell,
    collections::{BTreeMap, HashMap},
    fs,
    sync::{Arc, OnceLock},
};

use game_engine_core::{
    adt, blp,
    footstep_data::FootstepSurface,
    ground_effect_data,
    liquid_data::{
        FIRST_LIQUID_OBJECT, LiquidMaterial, MAGMA_NOISE_FDID, MAGMA_NOISE_SIZE, MapLiquidCatalog,
        TEXTURE_SLOTS,
    },
    terrain_surface_data, wdt,
    wmo_surface_data::{WmoSurfaceBounds, select_wmo_material_surface},
};
use osso_asset_resolver::{AssetResolverConfig, CascListfileResolver};
use shared::ground::{WmoCollision, WmoGroupCollision};

use super::textures::{TerrainLayerTextures, TerrainTextureCache};
use crate::lighting::assets::LightingCatalog;
use crate::wmo::placement::PlacedWmo;

fn parse_tile_root(
    path: &Path,
    bytes: &[u8],
    tile: (u32, u32),
    tex_file: &Option<(PathBuf, Vec<u8>)>,
) -> Result<adt::Root, String> {
    adt::parse_root_for_tile(
        bytes,
        tile.0,
        tile.1,
        tex_file.as_ref().map(|(_, bytes)| bytes.as_slice()),
    )
    .map_err(|error| format!("{}: {error}", path.display()))
}

fn parse_tile_textures(
    tex_file: &Option<(PathBuf, Vec<u8>)>,
    flags: wdt::MphdFlags,
    root: &adt::Root,
) -> Result<Option<adt::AdtTexData>, String> {
    tex_file
        .as_ref()
        .map(|(path, bytes)| {
            adt::parse_tex(bytes, flags, root)
                .map_err(|error| format!("{}: {error}", path.display()))
        })
        .transpose()
}

fn parse_tile_objects(
    obj_file: &Option<(PathBuf, Vec<u8>)>,
) -> Result<Option<adt::AdtObjData>, String> {
    obj_file
        .as_ref()
        .map(|(path, bytes)| {
            adt::parse_obj(bytes).map_err(|error| format!("{}: {error}", path.display()))
        })
        .transpose()
}

pub(crate) struct NativeTerrainAssets {
    resolver: CascListfileResolver,
    terrain_dir: PathBuf,
    data_root: PathBuf,
    textures: RefCell<TerrainTextureCache>,
    lighting: RefCell<Option<Arc<LightingCatalog>>>,
    surface_catalog: OnceLock<Result<SurfaceCatalog, String>>,
    liquids: OnceLock<Result<MapLiquidCatalog, String>>,
    maps: OnceLock<Result<game_engine_core::map_catalog::MapCatalog, String>>,
    /// Parsed group floors and root-wide material surface, keyed by root FDID.
    wmo_groups: RefCell<HashMap<u32, (Vec<Arc<WmoGroupCollision>>, Option<FootstepSurface>)>>,
}

type CachedFile = (PathBuf, Vec<u8>);
type TileFiles = (CachedFile, Option<CachedFile>, Option<CachedFile>);

pub(crate) struct NativeMapWdt {
    pub map_id: u32,
    pub path: PathBuf,
    pub flags: wdt::MphdFlags,
    pub tiles: wdt::WdtTiles,
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
    pub chunk_surfaces: BTreeMap<(u32, u32), FootstepSurface>,
    /// Collision of the tile's `_obj0` MODF WMOs by MODF unique id, placed as they render.
    /// Their floors are the player's ground and their faces physics walls from the moment the
    /// tile is parsed, before any WMO node spawns. A WMO spanning tiles is in each tile's list.
    pub wmo_floors: Vec<(u32, WmoCollision)>,
    pub wmo_surfaces: Vec<(WmoSurfaceBounds, FootstepSurface)>,
    /// Liquid material of each MH2O `(liquid_type, liquid_object)` on the tile.
    pub liquid_materials: BTreeMap<(u16, u16), Result<Arc<NativeLiquidMaterial>, String>>,
}

/// One decoded texture frame; a zero-FDID (procedural) frame has no image and binds black.
pub(crate) type LiquidFrame = (u32, Option<Arc<blp::RgbaImage>>);

/// A LiquidType's DB2 inputs with the decoded frames of the slots its shader samples and
/// its engine-global textures by uniform name.
pub(crate) struct NativeLiquidMaterial {
    pub params: LiquidMaterial,
    pub slots: [Vec<LiquidFrame>; TEXTURE_SLOTS],
    pub globals: Vec<(&'static str, Arc<blp::RgbaImage>)>,
}

impl NativeTerrainAssets {
    pub fn new(data_root: PathBuf) -> Self {
        let config = AssetResolverConfig::new()
            .with_data_root(&data_root)
            .with_shared_data_root(&data_root);
        Self {
            resolver: CascListfileResolver::new(config),
            terrain_dir: data_root.join("terrain"),
            data_root,
            textures: RefCell::new(TerrainTextureCache::default()),
            lighting: RefCell::new(None),
            surface_catalog: OnceLock::new(),
            liquids: OnceLock::new(),
            maps: OnceLock::new(),
            wmo_groups: RefCell::new(HashMap::new()),
        }
    }

    pub fn read_map_wdt(&self, map: &str) -> Result<NativeMapWdt, String> {
        let identity = self.map_identity(map)?;
        let (path, bytes) = self.read_fdid_file(identity.wdt_fdid, "wdt")?;
        let tiles =
            wdt::parse_wdt_tiles(&bytes).map_err(|error| format!("{}: {error}", path.display()))?;
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
                let surface = classify_wmo_surface(&asset.root, &self.resolver);
                Ok::<_, String>(PlacedWmo::new(placement, asset, surface))
            })
            .transpose()?;
        Ok(NativeMapWdt {
            map_id: identity.id,
            path,
            flags,
            tiles,
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
        let ((root_path, root_bytes), tex_file, obj_file) =
            self.read_tile_files(map, tile_y, tile_x, &wdt.tiles)?;
        let mut root = parse_tile_root(&root_path, &root_bytes, (tile_y, tile_x), &tex_file)?;
        let tex = parse_tile_textures(&tex_file, wdt.flags, &root)?;
        let obj = parse_tile_objects(&obj_file)?;
        let chunk_surfaces = self.classify_tile_surfaces(&root, tex.as_ref());
        let textures = self.load_tile_textures(tex.as_ref())?;
        let liquid_materials = self.read_liquid_materials(wdt.map_id, &mut root)?;
        let (wmo_floors, wmo_surfaces) = obj
            .as_ref()
            .map(|obj| self.read_wmo_floors(&obj.wmos, (tile_y, tile_x)))
            .unwrap_or_default();
        Ok(NativeTerrainTile {
            root_path,
            tex_path: tex_file.map(|(path, _)| path),
            obj_path: obj_file.map(|(path, _)| path),
            root,
            tex,
            obj,
            textures,
            chunk_surfaces,
            wmo_floors,
            wmo_surfaces,
            liquid_materials,
        })
    }

    fn load_tile_textures(
        &self,
        tex: Option<&adt::AdtTexData>,
    ) -> Result<BTreeMap<u32, TerrainLayerTextures>, String> {
        match tex {
            Some(tex) => {
                self.textures
                    .borrow_mut()
                    .load_for_tile(&self.resolver, &self.data_root, tex)
            }
            None => Ok(BTreeMap::new()),
        }
    }

    /// Resolves each layer's material, then reads LiquidObject vertices in its LVF.
    fn read_liquid_materials(
        &self,
        map_id: u32,
        root: &mut adt::Root,
    ) -> Result<BTreeMap<(u16, u16), Result<Arc<NativeLiquidMaterial>, String>>, String> {
        let mut materials = BTreeMap::new();
        let layers = root
            .water
            .iter_mut()
            .flat_map(|water| &mut water.chunks)
            .flat_map(|chunk| &mut chunk.layers);
        for layer in layers {
            let key = (layer.liquid_type, layer.liquid_object);
            if !materials.contains_key(&key) {
                let source = self.liquid_source(map_id);
                let material = match source.resolve(key) {
                    Ok(params) => Ok(Arc::new(source.read_textures(params)?)),
                    Err(error) => Err(error),
                };
                materials.insert(key, material);
            }
            let Some(Ok(material)) = materials.get(&key) else {
                continue;
            };
            if layer.liquid_object >= FIRST_LIQUID_OBJECT {
                let lvf = material.params.lvf;
                if let Err(error) = layer.decode_object_vertices(lvf) {
                    let error = format!(
                        "LiquidObject {} LVF {lvf} vertices: {error}",
                        layer.liquid_object
                    );
                    materials.insert(key, Err(error));
                }
            }
        }
        Ok(materials)
    }

    /// The liquid material of one MH2O `(liquid_type, liquid_object)`; texture read failures
    /// are errors of the whole tile, material resolution failures only of its layers.
    pub fn read_liquid_material(
        &self,
        map_id: u32,
        key: (u16, u16),
    ) -> Result<NativeLiquidMaterial, String> {
        self.liquid_source(map_id).read_material(key)
    }

    fn liquid_source(&self, map_id: u32) -> LiquidSource<'_> {
        LiquidSource {
            map_id,
            resolver: &self.resolver,
            data_root: &self.data_root,
            textures: &self.textures,
            catalog: &self.liquids,
        }
    }

    fn classify_tile_surfaces(
        &self,
        root: &adt::Root,
        tex: Option<&adt::AdtTexData>,
    ) -> BTreeMap<(u32, u32), FootstepSurface> {
        let Some(tex) = tex else {
            return BTreeMap::new();
        };
        let catalog = self.read_surface_catalog().ok();
        let paths: Vec<_> = tex
            .texture_fdids
            .iter()
            .map(|&fdid| self.resolver.resolve_path(fdid))
            .collect();
        root.height_grids
            .iter()
            .map(|grid| {
                let index = (grid.index_y * 16 + grid.index_x) as usize;
                let surface = tex
                    .chunk_layers
                    .get(index)
                    .map_or(FootstepSurface::Dirt, |layers| {
                        terrain_surface_data::dominant_surface_for_chunk_with_resolver(
                            tex,
                            layers,
                            |effect_id| {
                                catalog.and_then(|catalog| catalog.effect_surface(effect_id))
                            },
                            |fdid| {
                                tex.texture_fdids
                                    .iter()
                                    .position(|&id| id == fdid)
                                    .and_then(|index| paths[index].as_deref())
                            },
                        )
                    });
                ((grid.index_x, grid.index_y), surface)
            })
            .collect()
    }

    fn read_surface_catalog(&self) -> Result<&SurfaceCatalog, String> {
        self.surface_catalog
            .get_or_init(|| {
                let result = self.load_surface_catalog();
                if let Err(error) = &result {
                    eprintln!("Native terrain footstep metadata unavailable: {error}");
                }
                result
            })
            .as_ref()
            .map_err(Clone::clone)
    }

    fn load_surface_catalog(&self) -> Result<SurfaceCatalog, String> {
        let ground = self.read_local_db2(1_308_499)?;
        let sounds = self.read_local_db2(1_284_822)?;
        SurfaceCatalog::parse(&ground, &sounds)
    }

    fn read_local_db2(&self, fdid: u32) -> Result<Vec<u8>, String> {
        let cache = self
            .data_root
            .join("dbfilesclient")
            .join(format!("{fdid}.db2"));
        let path = self.resolver.ensure_cached_checked(fdid, &cache)?;
        read_bytes(&path).map_err(|error| format!("DB2 FDID {fdid}: {error}"))
    }

    /// Unreadable WMO roots have neither floors nor surface metadata.
    fn read_wmo_floors(
        &self,
        placements: &[adt::WmoPlacement],
        tile: (u32, u32),
    ) -> (
        Vec<(u32, WmoCollision)>,
        Vec<(WmoSurfaceBounds, FootstepSurface)>,
    ) {
        let mut floors = Vec::new();
        let mut surfaces = Vec::new();
        for placement in placements {
            match self.read_wmo_groups(placement) {
                Ok((groups, surface)) => {
                    floors.push((
                        placement.unique_id,
                        crate::wmo::placement::adt_wmo_collision(placement, tile, groups),
                    ));
                    if let Some(surface) = surface {
                        surfaces.push((wmo_surface_bounds(placement, false), surface));
                    }
                }
                Err(error) => eprintln!(
                    "WMO {} on tile {tile:?} has no floor or footstep surface: {error}",
                    placement.unique_id
                ),
            }
        }
        (floors, surfaces)
    }

    fn read_wmo_groups(
        &self,
        placement: &adt::WmoPlacement,
    ) -> Result<(Vec<Arc<WmoGroupCollision>>, Option<FootstepSurface>), String> {
        if let Some(groups) = placement
            .fdid
            .and_then(|fdid| self.wmo_groups.borrow().get(&fdid).cloned())
        {
            return Ok(groups);
        }
        let asset = crate::wmo::assets::read_placement(&self.resolver, &self.data_root, placement)?;
        let groups: Vec<_> = asset
            .groups
            .iter()
            .map(|group| Arc::clone(&group.collision))
            .collect();
        let result = (groups, classify_wmo_surface(&asset.root, &self.resolver));
        self.wmo_groups
            .borrow_mut()
            .insert(asset.root_fdid, result.clone());
        Ok(result)
    }

    fn map_identity(
        &self,
        directory: &str,
    ) -> Result<&game_engine_core::map_catalog::MapIdentity, String> {
        let maps = self
            .maps
            .get_or_init(|| game_engine_core::map_catalog::MapCatalog::read(&self.data_root))
            .as_ref()
            .map_err(Clone::clone)?;
        maps.by_directory(directory)
            .ok_or_else(|| format!("Map.csv: no Directory {directory}"))
    }

    fn read_tile_files(
        &self,
        map: &str,
        first: u32,
        second: u32,
        tiles: &wdt::WdtTiles,
    ) -> Result<TileFiles, String> {
        if !tiles.active.contains(&(first, second)) {
            return Err(format!(
                "Map {map} tile ({first}, {second}) is inactive in WDT MAIN"
            ));
        }
        if tiles.maid.is_some() {
            let ids = tiles
                .file_ids(first, second)
                .ok_or_else(|| format!("Map {map} active tile ({first}, {second}) missing MAID"))?;
            return Ok((
                self.read_fdid_file(ids.root, "adt")?,
                self.read_optional_fdid(ids.tex0)?,
                self.read_optional_fdid(ids.obj0)?,
            ));
        }
        let stem = format!("world/maps/{map}/{map}_{first}_{second}");
        Ok((
            self.read_declared_file(&format!("{stem}.adt"), "adt")?,
            self.read_optional_companion(&format!("{stem}_tex0.adt"))?,
            self.read_optional_companion(&format!("{stem}_obj0.adt"))?,
        ))
    }

    fn read_optional_fdid(&self, fdid: u32) -> Result<Option<CachedFile>, String> {
        if fdid == 0 {
            return Ok(None);
        }
        self.read_fdid_file(fdid, "adt").map(Some)
    }

    /// The map-level WDL FDID from the local listfile; no declared FDID means no horizon.
    /// MAID's per-tile LOD/maptexture IDs are not a map-level WDL.
    pub fn read_map_wdl(&self, map: &str) -> Result<Option<Vec<u8>>, String> {
        let path = format!("world/maps/{map}/{map}.wdl");
        if self.resolver.lookup_path(&path).is_none() {
            return Ok(None);
        }
        self.read_declared_file(&path, "wdl")
            .map(|(_, bytes)| Some(bytes))
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
        self.read_fdid_file(fdid, extension)
    }

    fn read_fdid_file(&self, fdid: u32, extension: &str) -> Result<CachedFile, String> {
        if fdid == 0 {
            return Err(format!("Required terrain {extension} has zero FileDataID"));
        }
        let cache_path = self.terrain_dir.join(format!("{fdid}.{extension}"));
        let path = self.resolver.ensure_cached_checked(fdid, &cache_path)?;
        let bytes = read_bytes(&path)?;
        Ok((path, bytes))
    }
}

struct SurfaceCatalog {
    effects: HashMap<u32, ground_effect_data::GroundEffectEntry>,
    sounds: HashMap<u8, FootstepSurface>,
}

impl SurfaceCatalog {
    fn parse(ground: &[u8], sounds: &[u8]) -> Result<Self, String> {
        let effects = ground_effect_data::parse_ground_effect_entries(ground)
            .map_err(|error| format!("GroundEffectTexture DB2 FDID 1308499: {error}"))?;
        let sounds = ground_effect_data::parse_terrain_type_sounds(sounds)
            .map_err(|error| format!("TerrainTypeSounds DB2 FDID 1284822: {error}"))?
            .into_iter()
            .filter_map(|(id, name)| {
                ground_effect_data::classify_surface_from_terrain_sound_name(&name)
                    .map(|surface| (id, surface))
            })
            .collect();
        Ok(Self { effects, sounds })
    }

    fn effect_surface(&self, effect_id: u32) -> Option<FootstepSurface> {
        let sound_id = self.effects.get(&effect_id)?.terrain_sound_id;
        self.sounds.get(&sound_id).copied()
    }
}

fn read_bytes(path: &Path) -> Result<Vec<u8>, String> {
    fs::read(path).map_err(|error| format!("{}: {error}", path.display()))
}

/// Liquid materials read through one resolver: the terrain reader's for MH2O layers,
/// the object spawner's for WMO group liquids.
pub(crate) struct LiquidSource<'a> {
    pub map_id: u32,
    pub resolver: &'a CascListfileResolver,
    pub data_root: &'a Path,
    pub textures: &'a RefCell<TerrainTextureCache>,
    pub catalog: &'a OnceLock<Result<MapLiquidCatalog, String>>,
}

impl LiquidSource<'_> {
    pub fn read_material(&self, key: (u16, u16)) -> Result<NativeLiquidMaterial, String> {
        self.read_textures(self.resolve(key)?)
    }

    fn read_textures(&self, params: LiquidMaterial) -> Result<NativeLiquidMaterial, String> {
        self.require_forever_texture_cache(&params)?;
        let mut slots: [Vec<LiquidFrame>; TEXTURE_SLOTS] = Default::default();
        for &slot in params.shader.texture_slots() {
            slots[slot] = self.read_frames(&params.texture_slots[slot])?;
        }
        let globals = params
            .shader
            .global_textures()
            .iter()
            .map(|&(name, fdid)| Ok((name, self.read_global_texture(fdid)?)))
            .collect::<Result<_, String>>()?;
        Ok(NativeLiquidMaterial {
            params,
            slots,
            globals,
        })
    }

    fn require_forever_texture_cache(&self, params: &LiquidMaterial) -> Result<(), String> {
        let catalog = self
            .catalog
            .get()
            .ok_or("Liquid catalog not initialized")?
            .as_ref()
            .map_err(Clone::clone)?;
        if !catalog.uses_forever(self.map_id) {
            return Ok(());
        }
        let sampled = params
            .shader
            .texture_slots()
            .iter()
            .flat_map(|&slot| &params.texture_slots[slot]);
        let globals = params.shader.global_textures().iter().map(|(_, fdid)| fdid);
        for &fdid in sampled.chain(globals).filter(|&&fdid| fdid != 0) {
            let extension = if fdid == MAGMA_NOISE_FDID {
                "blob"
            } else {
                "blp"
            };
            let path = self
                .data_root
                .join("textures")
                .join(format!("{fdid}.{extension}"));
            if !path.is_file() {
                return Err(format!(
                    "Forever map {} liquid texture FDID {fdid} missing at {}",
                    self.map_id,
                    path.display()
                ));
            }
        }
        Ok(())
    }

    fn resolve(&self, (liquid_type, liquid_object): (u16, u16)) -> Result<LiquidMaterial, String> {
        let catalog = self
            .catalog
            .get_or_init(|| MapLiquidCatalog::read(self.data_root))
            .as_ref()
            .map_err(Clone::clone)?;
        catalog.render_material(self.map_id, liquid_type, liquid_object)
    }

    fn read_frames(&self, fdids: &[u32]) -> Result<Vec<LiquidFrame>, String> {
        fdids
            .iter()
            .map(|&fdid| {
                let image = (fdid != 0)
                    .then(|| {
                        self.textures
                            .borrow_mut()
                            .load_image(self.resolver, self.data_root, fdid)
                    })
                    .transpose()?;
                Ok((fdid, image))
            })
            .collect()
    }

    /// The magma noise volume is raw RGBA slices, every other global a BLP.
    fn read_global_texture(&self, fdid: u32) -> Result<Arc<blp::RgbaImage>, String> {
        if fdid != MAGMA_NOISE_FDID {
            return self
                .textures
                .borrow_mut()
                .load_image(self.resolver, self.data_root, fdid);
        }
        let cache = self.data_root.join("textures").join(format!("{fdid}.blob"));
        let path = self.resolver.ensure_cached(fdid, &cache)?.ok_or_else(|| {
            format!(
                "Local CASC noise volume FDID {fdid} unavailable at {}",
                cache.display()
            )
        })?;
        let pixels = read_bytes(&path)?;
        let (width, height) = MAGMA_NOISE_SIZE;
        if pixels.len() != (width * height * 4) as usize {
            return Err(format!(
                "Noise volume FDID {fdid} has {} bytes, expected {width}x{height} RGBA",
                pixels.len()
            ));
        }
        Ok(Arc::new(blp::RgbaImage {
            pixels,
            width,
            height,
        }))
    }
}

/// Repository `data/` plus the user's local-CASC resolver cache.
#[cfg(test)]
pub(crate) fn cached_assets() -> NativeTerrainAssets {
    NativeTerrainAssets::new(test_data_root())
}

#[cfg(test)]
pub(crate) fn test_data_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../data")
}

fn classify_wmo_surface(
    root: &game_engine_core::wmo::WmoRootData,
    resolver: &CascListfileResolver,
) -> Option<FootstepSurface> {
    let paths: Vec<_> = root
        .materials
        .iter()
        .map(|mat| resolver.resolve_path(mat.texture_fdid))
        .collect();
    select_wmo_material_surface(root.materials.iter().zip(&paths).map(|(mat, path)| {
        (
            mat.ground_type != 0,
            mat.diff_color[3] > 0.0,
            mat.texture_fdid,
            path.as_deref(),
        )
    }))
}

pub(crate) fn wmo_surface_bounds(placement: &adt::WmoPlacement, global: bool) -> WmoSurfaceBounds {
    let convert: fn([f32; 3]) -> glam::Vec3 = if global {
        shared::ground::global_wmo_placement_position
    } else {
        shared::ground::placement_position
    };
    let first = convert(placement.extents_min);
    let second = convert(placement.extents_max);
    WmoSurfaceBounds {
        world_min: first.min(second).to_array(),
        world_max: first.max(second).to_array(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn zephras_missing_forever_liquid_texture_is_explicit_without_retail_extraction() {
        let data = test_data_root();
        let scratch =
            std::env::temp_dir().join(format!("native-forever-liquid-{}", std::process::id()));
        for build in ["12.1.0.69933", "1.60.1.70205"] {
            let destination = scratch.join("db2").join(build);
            fs::create_dir_all(&destination).unwrap();
            for table in [
                "Map",
                "LiquidType",
                "LiquidObject",
                "LiquidMaterial",
                "LiquidTypeXTexture",
            ] {
                let name = format!("{table}.csv");
                fs::copy(
                    data.join("db2").join(build).join(&name),
                    destination.join(name),
                )
                .unwrap();
            }
        }
        let assets = NativeTerrainAssets::new(scratch.clone());
        let error = assets
            .read_liquid_material(2991, (1251, 18420))
            .err()
            .unwrap();
        assert!(
            error.contains("Forever map 2991 liquid texture FDID"),
            "{error}"
        );
        assert!(error.contains("missing at"), "{error}");
        assert!(!scratch.join("textures").exists());
        fs::remove_dir_all(scratch).unwrap();
    }

    #[test]
    fn zephras_reads_unnamed_wdt_and_maid_tile_from_fdid_cache() {
        let assets = cached_assets();
        let map = assets.read_map_wdt("2991").unwrap();
        assert_eq!(map.path.file_name().unwrap(), "7198644.wdt");
        let tile = assets.read_tile("2991", 29, 26).unwrap();
        assert_eq!(tile.root_path.file_name().unwrap(), "7199999.adt");
        assert_eq!(tile.tex_path.unwrap().file_name().unwrap(), "7200002.adt");
        assert_eq!(tile.obj_path.unwrap().file_name().unwrap(), "7200000.adt");
        assert_eq!(tile.root.chunks.len(), 256);
        assert_eq!(tile.obj.unwrap().doodads.len(), 218);
        assert!(assets.read_map_wdl("2991").unwrap().is_none());
        assert!(
            assets
                .read_tile("2991", 0, 0)
                .err()
                .unwrap()
                .contains("inactive")
        );
    }

    #[test]
    fn zephras_maid_reader_preserves_named_companions_without_maid() {
        let assets = cached_assets();
        let tiles = wdt::WdtTiles {
            active: std::collections::BTreeSet::from([(32, 48)]),
            maid: None,
        };
        let ((root_path, bytes), tex, obj) =
            assets.read_tile_files("azeroth", 32, 48, &tiles).unwrap();
        assert_eq!(root_path.file_name().unwrap(), "778027.adt");
        assert_eq!(adt::parse_root(&bytes).unwrap().chunks.len(), 256);
        assert_eq!(tex.unwrap().0.file_name().unwrap(), "778030.adt");
        assert_eq!(obj.unwrap().0.file_name().unwrap(), "778028.adt");
    }

    #[test]
    fn zephras_declared_maid_does_not_switch_to_named_files_on_missing_root() {
        let assets = cached_assets();
        let tiles = wdt::WdtTiles {
            active: std::collections::BTreeSet::from([(32, 48)]),
            maid: Some(std::collections::BTreeMap::from([(
                (32, 48),
                wdt::TileFileIds::default(),
            )])),
        };
        let error = assets
            .read_tile_files("azeroth", 32, 48, &tiles)
            .err()
            .unwrap();
        assert!(error.contains("zero FileDataID"), "{error}");
    }

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
    fn authored_global_and_streamed_modf_extents_use_their_world_coordinate_conventions() {
        let data = test_data_root().join("terrain");
        let wdt_bytes = fs::read(data.join("791060.wdt")).unwrap();
        let global = wdt::parse_wdt_global_wmo(&wdt_bytes).unwrap().unwrap();
        let bounds = wmo_surface_bounds(&global, true);
        // Stockade WDT extents are near the global origin, not ADT map center.
        assert!((bounds.world_min[0] + 8.136_324).abs() < 0.01);
        assert!((bounds.world_max[2] - 150.329_83).abs() < 0.01);

        let obj_bytes = fs::read(data.join("azeroth_32_48_obj0.adt")).unwrap();
        let obj = adt::parse_obj(&obj_bytes).unwrap();
        let placement = obj.wmos.first().expect("authored MODF");
        let bounds = wmo_surface_bounds(placement, false);
        // Azeroth 32_48 MODF extents use absolute map coordinates.
        assert!((bounds.world_min[0] + 8879.101).abs() < 0.01);
        assert!((bounds.world_max[2] - 230.155_6).abs() < 0.01);
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
    fn local_ground_effect_rows_resolve_authored_surfaces() {
        use game_engine_core::footstep_data::FootstepSurface;

        let assets = cached_assets();
        let catalog = assets.read_surface_catalog().expect("local DB2 catalog");
        assert_eq!(
            catalog.effect_surface(141_671),
            Some(FootstepSurface::Metal)
        );
        assert_eq!(
            catalog.effect_surface(163_441),
            Some(FootstepSurface::Grass)
        );
        assert_eq!(catalog.effect_surface(999_999_999), None);
    }

    #[test]
    fn chunk_classification_prefers_effect_then_texture_and_retains_dirt() {
        use game_engine_core::asset::adt_format::adt_tex::{
            ChunkTexLayers, MclyFlags, TextureLayer,
        };

        let assets = cached_assets();
        let root_path = test_data_root().join("terrain/azeroth_32_48.adt");
        let root = adt::parse_root_for_tile(&fs::read(root_path).unwrap(), 32, 48, None).unwrap();
        let layer = |effect_id| TextureLayer {
            texture_index: 0,
            flags: MclyFlags::default(),
            effect_id,
            material_id: 0,
            alpha_map: None,
        };
        let tex = adt::AdtTexData {
            map_flags: wdt::MphdFlags::default(),
            texture_amplifier: None,
            texture_fdids: vec![126_746], // listfile: dm_grass.blp
            height_texture_fdids: vec![],
            texture_flags: vec![],
            texture_params: vec![],
            chunk_layers: (0..33)
                .map(|index| ChunkTexLayers {
                    layers: vec![layer(match index {
                        0 => 141_671,
                        32 => 999_999_999,
                        _ => 0,
                    })],
                })
                .collect(),
        };
        let surfaces = assets.classify_tile_surfaces(&root, Some(&tex));
        assert_eq!(surfaces.get(&(0, 0)), Some(&FootstepSurface::Metal));
        assert_eq!(surfaces.get(&(0, 1)), Some(&FootstepSurface::Grass));
        assert_eq!(surfaces.get(&(0, 2)), Some(&FootstepSurface::Grass));
        assert_eq!(surfaces.get(&(0, 3)), Some(&FootstepSurface::Dirt));
        assert!(assets.classify_tile_surfaces(&root, None).is_empty());

        let mut missing = tex;
        missing.texture_fdids = vec![999_999_999];
        assert_eq!(
            assets
                .classify_tile_surfaces(&root, Some(&missing))
                .get(&(0, 1)),
            Some(&FootstepSurface::Dirt)
        );
    }

    #[test]
    fn failed_optional_catalog_keeps_texture_classification() {
        use game_engine_core::asset::adt_format::adt_tex::{
            ChunkTexLayers, MclyFlags, TextureLayer,
        };

        let assets = cached_assets();
        assets
            .surface_catalog
            .set(Err("bad local DB2".into()))
            .ok()
            .unwrap();
        let root_path = test_data_root().join("terrain/azeroth_32_48.adt");
        let root = adt::parse_root_for_tile(&fs::read(root_path).unwrap(), 32, 48, None).unwrap();
        let tex = adt::AdtTexData {
            map_flags: wdt::MphdFlags::default(),
            texture_amplifier: None,
            texture_fdids: vec![126_746],
            height_texture_fdids: vec![],
            texture_flags: vec![],
            texture_params: vec![],
            chunk_layers: vec![ChunkTexLayers {
                layers: vec![TextureLayer {
                    texture_index: 0,
                    flags: MclyFlags::default(),
                    effect_id: 141_671,
                    material_id: 0,
                    alpha_map: None,
                }],
            }],
        };
        assert_eq!(
            assets
                .classify_tile_surfaces(&root, Some(&tex))
                .get(&(0, 0)),
            Some(&FootstepSurface::Grass)
        );
        assert_eq!(
            assets.read_surface_catalog().err().as_deref(),
            Some("bad local DB2")
        );
    }

    #[test]
    fn invalid_local_db2_reports_fdid_and_parse_error() {
        let error = SurfaceCatalog::parse(b"invalid", b"invalid")
            .err()
            .expect("invalid DB2 must fail");
        assert!(error.contains("1308499"), "{error}");
        assert!(error.contains("WDC5"), "{error}");
    }

    #[test]
    fn missing_map_and_tile_are_errors_not_empty_assets() {
        let assets = cached_assets();
        let map_error = assets
            .read_map_wdt("map_that_does_not_exist_999")
            .err()
            .unwrap();
        assert!(
            map_error.contains("no Directory map_that_does_not_exist_999"),
            "{map_error}"
        );

        let tile_error = assets.read_tile("azeroth", 64, 64).err().unwrap();
        assert!(
            tile_error.contains("azeroth tile (64, 64) is inactive in WDT"),
            "{tile_error}"
        );
    }
}
