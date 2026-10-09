//! Worker-side authored texture file boundary, independent of Godot allocation.
use std::path::Path;

pub(super) fn read_cached_file(
    file: &Path,
    fdid: Option<u32>,
    extract: impl FnOnce(u32, &Path) -> Result<(), String>,
) -> Result<Vec<u8>, String> {
    match (std::fs::read(file), fdid) {
        (Ok(bytes), _) => Ok(bytes),
        (Err(error), Some(fdid)) if error.kind() == std::io::ErrorKind::NotFound => {
            extract(fdid, file)?;
            std::fs::read(file)
                .map_err(|error| format!("Read cached UI file {}: {error}", file.display()))
        }
        (Err(error), _) => Err(format!("Read UI file {}: {error}", file.display())),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn missing_fdid_is_cached_from_its_real_asset_before_decode() {
        let data = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../data");
        let source = data.join("textures/896467.blp");
        let directory =
            std::env::temp_dir().join(format!("ui-fdid-missing-{}", std::process::id()));
        std::fs::create_dir_all(&directory).unwrap();
        let destination = directory.join("896467.blp");
        let bytes = read_cached_file(&destination, Some(896467), |fdid, path| {
            assert_eq!(fdid, 896467);
            std::fs::copy(&source, path)
                .map(|_| ())
                .map_err(|e| e.to_string())
        })
        .unwrap();
        assert_eq!(bytes, std::fs::read(&source).unwrap());
        let image = game_engine_core::blp::decode_rgba(&bytes).unwrap();
        assert!(image.width > 0 && image.height > 0);
        assert!(!image.pixels.is_empty());
        std::fs::remove_dir_all(directory).unwrap();
    }

    #[test]
    fn genuinely_unavailable_fdid_propagates_casc_receipt_not_substitute_art() {
        let directory =
            std::env::temp_dir().join(format!("ui-fdid-unavailable-{}", std::process::id()));
        std::fs::create_dir_all(&directory).unwrap();
        let destination = directory.join("999999999.blp");
        let error = read_cached_file(&destination, Some(999999999), |_, _| {
            Err("Local CASC FDID 999999999 absent".into())
        })
        .unwrap_err();
        assert!(
            error.contains("Local CASC FDID 999999999 absent"),
            "{error}"
        );
        assert!(!destination.exists());
        std::fs::remove_dir_all(directory).unwrap();
    }

    #[test]
    fn existing_cache_and_non_fdid_files_do_not_trigger_casc() {
        let data = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../data");
        let source = data.join("textures/896467.blp");
        let cached = read_cached_file(&source, Some(896467), |_, _| {
            panic!("Existing cache must not extract")
        })
        .unwrap();
        assert_eq!(cached, std::fs::read(source).unwrap());
        let absent = std::env::temp_dir().join(format!("ui-font-{}.ttf", std::process::id()));
        assert!(
            read_cached_file(&absent, None, |_, _| panic!(
                "Non-FDID art must not extract"
            ))
            .is_err()
        );
    }
}
