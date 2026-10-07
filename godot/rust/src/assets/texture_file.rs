//! Existing BLP file-read and missing-FDID receipt boundary, without engine allocation.
use std::{fs, path::Path};

pub(super) fn read_texture_file(
    fdid: u32,
    dir: &Path,
    record_missing: impl FnOnce(u32),
) -> Result<Option<Vec<u8>>, String> {
    match fs::read(dir.join(format!("{fdid}.blp"))) {
        Ok(bytes) => Ok(Some(bytes)),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
            record_missing(fdid);
            Ok(None)
        }
        Err(error) => Err(format!("Cannot read texture {fdid}: {error}")),
    }
}
