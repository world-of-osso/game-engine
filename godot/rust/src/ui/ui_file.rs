//! Shipped UI file boundary: no WoW installation, CASC, or substitute art.
use std::path::Path;

pub(super) fn read_required_file(file: &Path) -> Result<Vec<u8>, String> {
    std::fs::read(file).map_err(|error| format!("Read UI file {}: {error}", file.display()))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn extracted_icon_reads_its_authentic_bytes_and_decodes_nonempty() {
        let data = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../data");
        let file = data.join("textures/135875.blp");
        let bytes = read_required_file(&file).unwrap();
        assert_eq!(bytes, std::fs::read(&file).unwrap());
        let image = game_engine_core::blp::decode_rgba(&bytes).unwrap();
        assert!(image.width > 0 && image.height > 0);
        assert!(image.pixels.chunks_exact(4).any(|pixel| pixel[3] != 0));
    }

    #[test]
    fn missing_icon_is_a_hard_shipped_file_error_without_creating_art() {
        let directory =
            std::env::temp_dir().join(format!("ui-file-missing-{}", std::process::id()));
        std::fs::create_dir_all(&directory).unwrap();
        let file = directory.join("135875.blp");
        let error = read_required_file(&file).unwrap_err();
        assert!(error.contains("Read UI file"), "{error}");
        assert!(error.contains("135875.blp"), "{error}");
        assert!(!file.exists());
        assert_eq!(std::fs::read_dir(&directory).unwrap().count(), 0);
        std::fs::remove_dir_all(directory).unwrap();
    }

    #[test]
    fn missing_non_icon_file_is_also_a_hard_file_error() {
        let file = std::env::temp_dir().join(format!("ui-font-missing-{}.ttf", std::process::id()));
        let error = read_required_file(&file).unwrap_err();
        assert!(error.contains("Read UI file"), "{error}");
        assert!(!file.exists());
    }
}
