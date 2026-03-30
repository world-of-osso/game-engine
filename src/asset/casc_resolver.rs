//! Auto-extract missing assets from local WoW CASC storage.
//!
//! Reads directly from the local WoW installation at [`WOW_DATA_PATH`].
//! On first use, parses `.build.info` and the build config to find root/encoding
//! keys, loads cached resolution files, and lazily initializes archive indices
//! only when an actual FDID extraction is needed.

use std::path::{Path, PathBuf};
use std::sync::{Mutex, OnceLock};

use cascette_client_storage::Installation;
use cascette_client_storage::resolver::ContentResolver;
use tokio::runtime::Handle as TokioHandle;

const WOW_DATA_PATH: &str = "/syncthing/World of Warcraft/Data";

static CASC: OnceLock<Option<CascState>> = OnceLock::new();

struct CascState {
    install: Installation,
    resolver: ContentResolver,
    /// Archive indices are expensive to load; defer until first FDID extraction.
    initialized: Mutex<InitState>,
}

enum InitState {
    Uninitialized,
    Initialized,
    Failed(String),
}

impl CascState {
    /// Ensure archive indices are loaded (slow, ~30s first time).
    fn ensure_initialized(&self) -> Result<(), String> {
        let mut init = self.initialized.lock().unwrap();
        match &*init {
            InitState::Initialized => return Ok(()),
            InitState::Failed(err) => return Err(err.clone()),
            InitState::Uninitialized => {}
        }
        match run_async(self.install.initialize()).map_err(|e| format!("CASC init: {e}")) {
            Ok(()) => {
                *init = InitState::Initialized;
                Ok(())
            }
            Err(err) => {
                *init = InitState::Failed(err.clone());
                Err(err)
            }
        }
    }
}

/// Ensure a BLP texture exists at `data/textures/{fdid}.blp`.
pub fn ensure_texture(fdid: u32) -> Option<PathBuf> {
    ensure_file(fdid, "textures", "blp")
}

/// Force CASC archive indices to initialize on the current thread.
pub fn warm_up() -> Result<(), String> {
    let casc = get_casc()?;
    casc.ensure_initialized()
}

/// Ensure an M2 model exists at `data/models/{fdid}.m2`.
pub fn ensure_model(fdid: u32) -> Option<PathBuf> {
    ensure_file(fdid, "models", "m2")
}

fn ensure_file(fdid: u32, dir: &str, ext: &str) -> Option<PathBuf> {
    let path = crate::paths::shared_data_path(dir).join(format!("{fdid}.{ext}"));
    if path.exists() {
        return Some(path);
    }
    extract_fdid_to_path(fdid, &path).ok()
}

/// Ensure a CASC asset exists at the requested output path.
pub fn ensure_file_at_path(fdid: u32, out_path: &Path) -> Option<PathBuf> {
    let shared_path = crate::paths::remap_to_shared_data_path(out_path);
    if shared_path.exists() {
        return Some(shared_path);
    }
    extract_fdid_to_path(fdid, &shared_path).ok()
}

fn extract_fdid_to_path(fdid: u32, out_path: &Path) -> Result<PathBuf, String> {
    let casc = get_casc()?;
    casc.ensure_initialized()?;

    let content_key = casc
        .resolver
        .resolve_file_data_id(fdid)
        .ok_or_else(|| format!("CASC resolve FDID {fdid}: missing content key in root"))?;
    let encoding_key = casc
        .resolver
        .resolve_content_key(&content_key)
        .ok_or_else(|| {
            format!("CASC resolve FDID {fdid}: missing encoding key for content {content_key}")
        })?;
    let data = run_async(casc.install.read_file_by_encoding_key(&encoding_key))
        .map_err(|e| format!("CASC read FDID {fdid} via encoding key {encoding_key}: {e}"))?;

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
    let resolver = ContentResolver::new();

    let casc_dir = crate::paths::shared_data_path("casc");
    load_cached_resolution_files(&resolver, &casc_dir)?;

    eprintln!("CASC resolver initialized from {WOW_DATA_PATH}");
    Ok(CascState {
        install,
        resolver,
        initialized: Mutex::new(InitState::Uninitialized),
    })
}

/// Load root+encoding from disk cache. Fails if cache doesn't exist yet.
fn load_cached_resolution_files(resolver: &ContentResolver, cache: &Path) -> Result<(), String> {
    let root_path = cache.join("root.bin");
    let enc_path = cache.join("encoding.bin");

    let root_data = std::fs::read(&root_path).map_err(|e| {
        format!(
            "{}: {e} (run `casc-init` binary first)",
            root_path.display()
        )
    })?;
    resolver
        .load_root_file(&root_data)
        .map_err(|e| format!("resolver load root: {e}"))?;

    let enc_data = std::fs::read(&enc_path)
        .map_err(|e| format!("{}: {e} (run `casc-init` binary first)", enc_path.display()))?;
    resolver
        .load_encoding_file(&enc_data)
        .map_err(|e| format!("resolver load encoding: {e}"))?;

    Ok(())
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
