#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    fn cached_assets() -> NativeTerrainAssets {
        let data_root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../data");
        let cache_root = std::env::var_os("XDG_CACHE_HOME")
            .map(PathBuf::from)
            .or_else(|| std::env::var_os("HOME").map(|home| PathBuf::from(home).join(".cache")))
            .expect("cache location")
            .join("asset-resolver");
        NativeTerrainAssets::new(data_root, cache_root)
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
        let wmo = stockade.global_wmo.expect("authored global WMO");
        assert_eq!(wmo.fdid, Some(108_631));
        assert_eq!(wmo.position, [0.0, 0.0, 0.0]);
        assert_eq!(stockade.flags.raw & 1, 1);
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
        assert!(tile.wdt.global_wmo.is_none());
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
