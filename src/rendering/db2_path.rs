use std::path::{Path, PathBuf};

pub(crate) fn ensure_db2_path(fdid: u32, path: &Path) -> Option<PathBuf> {
    if path.exists() {
        return Some(path.to_path_buf());
    }
    crate::asset::asset_cache::file_at_path(fdid, path)
}

#[cfg(test)]
mod tests {
    use super::ensure_db2_path;
    use std::fs;

    #[test]
    fn ensure_db2_path_returns_existing_path() {
        let stamp = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .expect("system clock before unix epoch")
            .as_nanos();
        let path = std::env::temp_dir().join(format!("game_engine_db2_path_{stamp}.db2"));
        fs::write(&path, b"test").expect("create temp db2 file");
        let ensured = ensure_db2_path(0, &path);
        assert_eq!(ensured, Some(path.clone()));
        fs::remove_file(path).expect("remove temp db2 file");
    }
}
