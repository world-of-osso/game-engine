//! Existing BLP file-read and missing-FDID receipt boundary, without engine allocation.
use std::{fs, path::Path};

/// The bytes of texture `fdid` at `path`, or `None` after `record_missing` when the
/// file is absent.
pub(super) fn read_texture_file(
    fdid: u32,
    path: &Path,
    record_missing: impl FnOnce(u32),
) -> Result<Option<Vec<u8>>, String> {
    match fs::read(path) {
        Ok(bytes) => Ok(Some(bytes)),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
            record_missing(fdid);
            Ok(None)
        }
        Err(error) => Err(format!("Cannot read texture {fdid}: {error}")),
    }
}
