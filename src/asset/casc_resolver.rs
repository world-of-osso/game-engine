//! Auto-extract missing assets from local WoW CASC storage.
//!
//! Provides [`ensure_texture`], [`ensure_model`], and [`ensure_terrain`] which check
//! the `data/` directory for a file and extract it from CASC if missing.
//! Extraction runs via `tokio::task::block_in_place` so it works from both sync
//! and async contexts without blocking the Bevy render loop when called from
//! background threads (e.g. terrain streaming).

use std::path::{Path, PathBuf};
use std::sync::OnceLock;

use cascette_client_storage::Installation;
use tokio::runtime::Handle as TokioHandle;

const WOW_DATA_PATH: &str = "/syncthing/World of Warcraft/Data";
const CACHE_DIR: &str = "/home/osso/.cache/casc-extract";

/// Lazily initialized CASC installation + resolver data.
static CASC: OnceLock<Option<CascState>> = OnceLock::new();

struct CascState {
    install: Installation,
}

/// Ensure a BLP texture exists at `data/textures/{fdid}.blp`.
/// Returns the path if the file exists or was successfully extracted.
pub fn ensure_texture(fdid: u32) -> Option<PathBuf> {
    ensure_file(fdid, "data/textures", "blp")
}


/// Check if `{dir}/{fdid}.{ext}` exists; if not, extract from CASC.
fn ensure_file(fdid: u32, dir: &str, ext: &str) -> Option<PathBuf> {
    let path = PathBuf::from(dir).join(format!("{fdid}.{ext}"));
    if path.exists() {
        return Some(path);
    }
    extract_fdid(fdid, dir).ok()
}

/// Extract a single FDID from CASC into the given output directory.
fn extract_fdid(fdid: u32, output_dir: &str) -> Result<PathBuf, String> {
    let casc = get_casc()?;
    let encoding_key = casc
        .install
        .resolver()
        .resolve_fdid_to_encoding(fdid)
        .ok_or_else(|| format!("FDID {fdid} not in root/encoding"))?;

    let data = run_async(casc.install.read_file_by_encoding_key(&encoding_key))
        .map_err(|e| format!("CASC read {fdid}: {e}"))?;

    let dir = Path::new(output_dir);
    std::fs::create_dir_all(dir).map_err(|e| format!("mkdir {output_dir}: {e}"))?;

    let filename = resolve_filename(fdid);
    let out_path = dir.join(&filename);
    std::fs::write(&out_path, &data).map_err(|e| format!("write {}: {e}", out_path.display()))?;
    eprintln!("CASC: extracted FDID {fdid} -> {}", out_path.display());
    Ok(out_path)
}

/// Get or initialize the CASC state. Returns Err if CASC is unavailable.
fn get_casc() -> Result<&'static CascState, String> {
    CASC.get_or_init(|| init_casc().ok())
        .as_ref()
        .ok_or_else(|| "CASC not available".to_string())
}

/// Initialize CASC: open local installation, load root + encoding files.
fn init_casc() -> Result<CascState, String> {
    let data_root = PathBuf::from(WOW_DATA_PATH);
    if !data_root.exists() {
        return Err(format!("WoW data not found at {}", data_root.display()));
    }

    let install = Installation::open(data_root).map_err(|e| format!("CASC open: {e}"))?;
    run_async(install.initialize()).map_err(|e| format!("CASC init: {e}"))?;

    load_resolution_files(&install)?;
    eprintln!("CASC resolver initialized from {WOW_DATA_PATH}");
    Ok(CascState { install })
}

/// Load cached root + encoding files from casc-extract cache.
fn load_resolution_files(install: &Installation) -> Result<(), String> {
    let build_id = read_build_id()?;
    let cache = PathBuf::from(CACHE_DIR).join(format!("wow-{build_id}"));

    let root_data = std::fs::read(cache.join("root.bin"))
        .map_err(|e| format!("root.bin: {e} (run `casc-extract init` first)"))?;
    install
        .load_root_file(&root_data)
        .map_err(|e| format!("load root: {e}"))?;

    let enc_data = std::fs::read(cache.join("encoding.bin"))
        .map_err(|e| format!("encoding.bin: {e} (run `casc-extract init` first)"))?;
    install
        .load_encoding_file(&enc_data)
        .map_err(|e| format!("load encoding: {e}"))?;

    Ok(())
}

fn read_build_id() -> Result<String, String> {
    let path = PathBuf::from(CACHE_DIR).join("build-id.txt");
    std::fs::read_to_string(&path)
        .map(|s| s.trim().to_string())
        .map_err(|e| format!("{}: {e}", path.display()))
}

/// Resolve FDID to a filename using the listfile, falling back to `.dat`.
fn resolve_filename(fdid: u32) -> String {
    let listfile = PathBuf::from(CACHE_DIR).join("listfile.csv");
    let ext = resolve_extension(fdid, &listfile);
    format!("{fdid}.{ext}")
}

fn resolve_extension(fdid: u32, listfile: &Path) -> String {
    if let Ok(content) = std::fs::read_to_string(listfile) {
        let prefix = format!("{fdid};");
        for line in content.lines() {
            if let Some(path) = line.strip_prefix(&prefix) {
                if let Some(ext) = path.rsplit('.').next() {
                    return ext.to_lowercase();
                }
            }
        }
    }
    "dat".to_string()
}

/// Run an async future on the current tokio runtime, or create a temporary one.
fn run_async<F: std::future::Future>(fut: F) -> F::Output {
    if let Ok(handle) = TokioHandle::try_current() {
        tokio::task::block_in_place(|| handle.block_on(fut))
    } else {
        // No tokio runtime — create a temporary one (startup path).
        tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .expect("failed to create tokio runtime")
            .block_on(fut)
    }
}
