//! Auto-extract missing assets from local WoW CASC storage.
//!
//! Reads directly from the local WoW installation at [`WOW_DATA_PATH`].
//! On first use, parses `.build.info` and the build config to find root/encoding
//! keys, loads cached resolution files, and lazily initializes archive indices
//! only when an actual FDID extraction is needed.

use std::path::{Path, PathBuf};
use std::sync::{Mutex, OnceLock};

use cascette_client_storage::Installation;
use tokio::runtime::Handle as TokioHandle;

const WOW_DATA_PATH: &str = "/syncthing/World of Warcraft/Data";
const CASC_DATA_DIR: &str = "data/casc";
const LISTFILE_PATH: &str = "data/community-listfile.csv";

static CASC: OnceLock<Option<CascState>> = OnceLock::new();

struct CascState {
    install: Installation,
    /// Archive indices are expensive to load; defer until first FDID extraction.
    initialized: Mutex<bool>,
}

impl CascState {
    /// Ensure archive indices are loaded (slow, ~30s first time).
    fn ensure_initialized(&self) -> Result<(), String> {
        let mut init = self.initialized.lock().unwrap();
        if *init {
            return Ok(());
        }
        run_async(self.install.initialize()).map_err(|e| format!("CASC init: {e}"))?;
        *init = true;
        Ok(())
    }
}

/// Ensure a BLP texture exists at `data/textures/{fdid}.blp`.
pub fn ensure_texture(fdid: u32) -> Option<PathBuf> {
    ensure_file(fdid, "data/textures", "blp")
}

fn ensure_file(fdid: u32, dir: &str, ext: &str) -> Option<PathBuf> {
    let path = PathBuf::from(dir).join(format!("{fdid}.{ext}"));
    if path.exists() {
        return Some(path);
    }
    extract_fdid(fdid, dir).ok()
}

/// Ensure a CASC asset exists at the requested output path.
pub fn ensure_file_at_path(fdid: u32, out_path: &Path) -> Option<PathBuf> {
    if out_path.exists() {
        return Some(out_path.to_path_buf());
    }
    extract_fdid_to_path(fdid, out_path).ok()
}

fn extract_fdid(fdid: u32, output_dir: &str) -> Result<PathBuf, String> {
    let filename = resolve_filename(fdid);
    let out_path = Path::new(output_dir).join(&filename);
    extract_fdid_to_path(fdid, &out_path)
}

fn extract_fdid_to_path(fdid: u32, out_path: &Path) -> Result<PathBuf, String> {
    let casc = get_casc()?;
    casc.ensure_initialized()?;

    let encoding_key = casc
        .install
        .resolver()
        .resolve_fdid_to_encoding(fdid)
        .ok_or_else(|| format!("FDID {fdid} not in root/encoding"))?;

    let data = run_async(casc.install.read_file_by_encoding_key(&encoding_key))
        .map_err(|e| format!("CASC read {fdid}: {e}"))?;

    write_to_path(out_path, &data)?;
    eprintln!("CASC: extracted FDID {fdid} -> {}", out_path.display());
    Ok(out_path.to_path_buf())
}

fn write_to_path(out_path: &Path, data: &[u8]) -> Result<(), String> {
    let parent = out_path
        .parent()
        .ok_or_else(|| format!("missing parent for {}", out_path.display()))?;
    std::fs::create_dir_all(parent).map_err(|e| format!("mkdir {}: {e}", parent.display()))?;
    std::fs::write(out_path, data).map_err(|e| format!("write {}: {e}", out_path.display()))?;
    Ok(())
}

fn get_casc() -> Result<&'static CascState, String> {
    CASC.get_or_init(|| init_casc().ok())
        .as_ref()
        .ok_or_else(|| "CASC not available".to_string())
}

/// Initialize CASC: parse build config, load cached root+encoding.
/// Does NOT load archive indices yet (deferred to first extraction).
fn init_casc() -> Result<CascState, String> {
    let data_root = PathBuf::from(WOW_DATA_PATH);
    if !data_root.exists() {
        return Err(format!("WoW data not found at {}", data_root.display()));
    }

    let install = Installation::open(data_root).map_err(|e| format!("CASC open: {e}"))?;

    let casc_dir = PathBuf::from(CASC_DATA_DIR);
    load_cached_resolution_files(&install, &casc_dir)?;

    eprintln!("CASC resolver initialized from {WOW_DATA_PATH}");
    Ok(CascState {
        install,
        initialized: Mutex::new(false),
    })
}

/// Load root+encoding from disk cache. Fails if cache doesn't exist yet.
fn load_cached_resolution_files(install: &Installation, cache: &Path) -> Result<(), String> {
    let root_path = cache.join("root.bin");
    let enc_path = cache.join("encoding.bin");

    let root_data = std::fs::read(&root_path).map_err(|e| {
        format!(
            "{}: {e} (run `casc-init` binary first)",
            root_path.display()
        )
    })?;
    install
        .load_root_file(&root_data)
        .map_err(|e| format!("load root: {e}"))?;

    let enc_data = std::fs::read(&enc_path)
        .map_err(|e| format!("{}: {e} (run `casc-init` binary first)", enc_path.display()))?;
    install
        .load_encoding_file(&enc_data)
        .map_err(|e| format!("load encoding: {e}"))?;

    Ok(())
}

/// Resolve FDID to a filename using the local community listfile.
fn resolve_filename(fdid: u32) -> String {
    let ext = resolve_extension(fdid, Path::new(LISTFILE_PATH));
    format!("{fdid}.{ext}")
}

fn resolve_extension(fdid: u32, listfile: &Path) -> String {
    if let Ok(content) = std::fs::read_to_string(listfile) {
        let prefix = format!("{fdid};");
        for line in content.lines() {
            if let Some(path) = line.strip_prefix(&prefix)
                && let Some(ext) = path.rsplit('.').next()
            {
                return ext.to_lowercase();
            }
        }
    }
    "dat".to_string()
}

fn run_async<F: std::future::Future>(fut: F) -> F::Output {
    if let Ok(handle) = TokioHandle::try_current() {
        tokio::task::block_in_place(|| handle.block_on(fut))
    } else {
        tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .expect("failed to create tokio runtime")
            .block_on(fut)
    }
}
