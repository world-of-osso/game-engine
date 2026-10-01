//! Local-CASC terrain texture reads, independent of Godot resource creation.

use std::{
    collections::{BTreeMap, HashMap},
    fs,
    path::Path,
    sync::Arc,
};

use game_engine_core::{adt, blp};
use osso_asset_resolver::CascListfileResolver;

/// One decoded image may be shared by every tile and layer referencing its FDID.
pub(crate) struct TerrainTexture {
    pub fdid: u32,
    pub image: Arc<blp::RgbaImage>,
}

pub(crate) struct TerrainLayerTextures {
    pub diffuse_fdid: u32,
    pub diffuse: Arc<blp::RgbaImage>,
    pub height: Option<TerrainTexture>,
}

#[derive(Default)]
pub(crate) struct TerrainTextureCache {
    decoded: HashMap<u32, Arc<blp::RgbaImage>>,
}

impl TerrainTextureCache {
    /// Returns only indices actually referenced by MCNK layers, keyed by MCLY texture index.
    pub fn load_for_tile(
        &mut self,
        resolver: &CascListfileResolver,
        data_root: &Path,
        tex: &adt::AdtTexData,
    ) -> Result<BTreeMap<u32, TerrainLayerTextures>, String> {
        let mut layers = BTreeMap::new();
        for index in tex
            .chunk_layers
            .iter()
            .flat_map(|chunk| &chunk.layers)
            .map(|layer| layer.texture_index)
        {
            if layers.contains_key(&index) {
                continue;
            }
            let authored = *tex
                .texture_fdids
                .get(index as usize)
                .ok_or_else(|| format!("MCLY texture index {index} has no MDID entry"))?;
            let diffuse_fdid = resolve_diffuse_fdid(resolver, authored)?;
            let diffuse = self.load_image(resolver, data_root, diffuse_fdid)?;
            let height = tex
                .height_texture_fdids
                .get(index as usize)
                .copied()
                .filter(|&fdid| fdid != 0)
                .map(|fdid| {
                    self.load_image(resolver, data_root, fdid)
                        .map(|image| TerrainTexture { fdid, image })
                })
                .transpose()?;
            layers.insert(
                index,
                TerrainLayerTextures {
                    diffuse_fdid,
                    diffuse,
                    height,
                },
            );
        }
        Ok(layers)
    }

    pub fn load_image(
        &mut self,
        resolver: &CascListfileResolver,
        data_root: &Path,
        fdid: u32,
    ) -> Result<Arc<blp::RgbaImage>, String> {
        if let Some(image) = self.decoded.get(&fdid) {
            return Ok(Arc::clone(image));
        }
        let destination = data_root.join("textures").join(format!("{fdid}.blp"));
        let path = resolver.ensure_cached(fdid, &destination).ok_or_else(|| {
            format!(
                "Local CASC texture FDID {fdid} unavailable at {}",
                destination.display()
            )
        })?;
        let bytes =
            fs::read(&path).map_err(|error| format!("FDID {fdid} {}: {error}", path.display()))?;
        let image = blp::decode_rgba(&bytes)
            .map_err(|error| format!("FDID {fdid} {}: {error}", path.display()))?;
        let image = Arc::new(image);
        self.decoded.insert(fdid, Arc::clone(&image));
        Ok(image)
    }
}

fn resolve_diffuse_fdid(resolver: &CascListfileResolver, authored: u32) -> Result<u32, String> {
    let path = resolver
        .resolve_path(authored)
        .ok_or_else(|| format!("MDID FDID {authored} not in local listfile"))?;
    let Some(base) = path
        .strip_suffix("_s.blp")
        .or_else(|| path.strip_suffix("_S.blp"))
    else {
        return Ok(authored);
    };
    let diffuse_path = format!("{base}.blp");
    resolver.lookup_path(&diffuse_path).ok_or_else(|| {
        format!("MDID FDID {authored} requires diffuse {diffuse_path}, absent from local listfile")
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::terrain::assets::NativeTerrainAssets;
    use game_engine_core::adt;
    use osso_asset_resolver::{AssetResolverConfig, CascListfileResolver};
    use std::{path::PathBuf, sync::Arc};

    fn fixture() -> (PathBuf, CascListfileResolver, adt::AdtTexData) {
        let data_root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../data");
        let assets = NativeTerrainAssets::new(data_root.clone());
        let tile = assets.read_tile("azeroth", 32, 48).expect("cached tile");
        let resolver = CascListfileResolver::new(
            AssetResolverConfig::new()
                .with_data_root(&data_root)
                .with_shared_data_root(&data_root),
        );
        (data_root, resolver, tile.tex.expect("texture companion"))
    }

    #[test]
    fn loads_only_referenced_authored_diffuse_and_reuses_pixels_across_tile_loads() {
        let (data_root, resolver, mut tex) = fixture();
        // This tile's MDID starts at 186770 (_s); the named diffuse is 186769.
        // MHID is all zero: no authored height images.
        let mut cache = TerrainTextureCache::default();
        let all = cache
            .load_for_tile(&resolver, &data_root, &tex)
            .expect("tile textures");
        let first = all.get(&0).expect("referenced first layer");
        assert_eq!(first.diffuse_fdid, 186769);
        assert_eq!((first.diffuse.width, first.diffuse.height), (256, 256));
        assert_eq!(first.diffuse.pixels.len(), 256 * 256 * 4);
        assert!(first.height.is_none());
        assert_eq!(all.len(), 10);

        let again = cache
            .load_for_tile(&resolver, &data_root, &tex)
            .expect("second tile");
        assert!(Arc::ptr_eq(
            &first.diffuse,
            &again.get(&0).expect("same texture index").diffuse
        ));
        tex.chunk_layers.truncate(1);
        let subset = cache
            .load_for_tile(&resolver, &data_root, &tex)
            .expect("one chunk");
        assert_eq!(subset.keys().copied().collect::<Vec<_>>(), vec![1, 2]);
    }

    #[test]
    fn mdid_already_pointing_to_diffuse_preserves_authored_fdid() {
        let (data_root, resolver, mut tex) = fixture();
        tex.chunk_layers.truncate(1);
        let index = tex.chunk_layers[0].layers[0].texture_index as usize;
        tex.texture_fdids[index] = 186769;
        let layers = TerrainTextureCache::default()
            .load_for_tile(&resolver, &data_root, &tex)
            .expect("direct diffuse MDID");
        assert_eq!(layers.get(&(index as u32)).unwrap().diffuse_fdid, 186769);
    }

    #[test]
    fn missing_mdid_reference_is_an_error_not_an_empty_layer() {
        let (data_root, resolver, mut tex) = fixture();
        tex.chunk_layers[0].layers[0].texture_index = tex.texture_fdids.len() as u32;
        let error = TerrainTextureCache::default()
            .load_for_tile(&resolver, &data_root, &tex)
            .err()
            .expect("invalid index");
        assert!(error.contains("texture index"), "{error}");
    }

    #[test]
    fn unresolved_mdid_fdid_does_not_guess_previous_id() {
        let (data_root, resolver, mut tex) = fixture();
        let index = tex.chunk_layers[0].layers[0].texture_index as usize;
        tex.texture_fdids[index] = 999_999_999;
        let error = TerrainTextureCache::default()
            .load_for_tile(&resolver, &data_root, &tex)
            .err()
            .expect("missing FDID");
        assert!(error.contains("999999999"), "{error}");
    }

    #[test]
    fn zero_height_is_absent_but_referenced_height_is_decoded() {
        let (data_root, resolver, mut tex) = fixture();
        tex.chunk_layers.truncate(1);
        let index = tex.chunk_layers[0].layers[0].texture_index as usize;
        tex.height_texture_fdids[index] = 878974;
        let layers = TerrainTextureCache::default()
            .load_for_tile(&resolver, &data_root, &tex)
            .expect("authored height texture");
        let height = layers
            .get(&(index as u32))
            .unwrap()
            .height
            .as_ref()
            .unwrap();
        assert_eq!(height.fdid, 878974);
        assert_eq!(
            height.image.pixels.len(),
            (height.image.width * height.image.height * 4) as usize
        );
    }

    #[test]
    fn invalid_cached_blp_reports_fdid_and_decode_error() {
        let (data_root, resolver, _) = fixture();
        let isolated = data_root
            .join("cache")
            .join(format!("terrain-texture-test-{}", std::process::id()));
        let texture_dir = isolated.join("textures");
        std::fs::create_dir_all(&texture_dir).expect("isolated cache directory");
        std::fs::write(texture_dir.join("186769.blp"), b"not a BLP")
            .expect("corrupt cache fixture");
        let error = TerrainTextureCache::default()
            .load_image(&resolver, &isolated, 186769)
            .err()
            .expect("invalid cached BLP");
        std::fs::remove_dir_all(&isolated).expect("remove isolated cache fixture");
        assert!(error.contains("186769"), "{error}");
        assert!(error.contains("Failed to load BLP"), "{error}");
    }

    #[test]
    fn missing_referenced_height_is_not_treated_as_zero_height() {
        let (data_root, resolver, mut tex) = fixture();
        tex.chunk_layers.truncate(1);
        let index = tex.chunk_layers[0].layers[0].texture_index as usize;
        tex.height_texture_fdids[index] = 999_999_999;
        let error = TerrainTextureCache::default()
            .load_for_tile(&resolver, &data_root, &tex)
            .err()
            .expect("missing height texture");
        assert!(error.contains("999999999"), "{error}");
    }
}
