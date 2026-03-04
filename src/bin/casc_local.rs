//! Extract files from local WoW CASC storage by FileDataID.
//!
//! Uses the cached root+encoding files from casc-extract init,
//! combined with the local CASC archives from the WoW installation.
//!
//! Usage:
//!   cargo run --bin casc-local -- <fdid> [fdid2 ...] [-o output_dir]

use cascette_client_storage::Installation;
use std::path::{Path, PathBuf};

const WOW_PATH: &str = "/syncthing/World of Warcraft";
const CACHE_DIR: &str = "/home/osso/.cache/casc-extract";

#[tokio::main]
async fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let (fdids, output_dir) = parse_args(&args);
    if fdids.is_empty() {
        eprintln!("Usage: casc-local <fdid> [fdid2 ...] [-o output_dir]");
        std::process::exit(1);
    }

    let install = open_installation().await;
    load_resolution_files(&install);

    let mut ok = 0u32;
    let mut fail = 0u32;
    for fdid in &fdids {
        match extract_fdid(&install, *fdid, &output_dir).await {
            Ok(path) => {
                eprintln!("Extracted FDID {fdid} -> {}", path.display());
                ok += 1;
            }
            Err(e) => {
                eprintln!("Failed FDID {fdid}: {e}");
                fail += 1;
            }
        }
    }
    eprintln!("{ok} extracted, {fail} failed");
    if fail > 0 {
        std::process::exit(1);
    }
}

fn parse_args(args: &[String]) -> (Vec<u32>, PathBuf) {
    let mut fdids = Vec::new();
    let mut output_dir = PathBuf::from(".");
    let mut i = 0;
    while i < args.len() {
        if args[i] == "-o" && i + 1 < args.len() {
            output_dir = PathBuf::from(&args[i + 1]);
            i += 2;
        } else if let Ok(fdid) = args[i].parse::<u32>() {
            fdids.push(fdid);
            i += 1;
        } else {
            i += 1;
        }
    }
    (fdids, output_dir)
}

async fn open_installation() -> Installation {
    let data_root = PathBuf::from(WOW_PATH).join("Data");
    eprintln!("Opening local CASC: {}", data_root.display());

    let install = Installation::open(data_root).expect("failed to open CASC installation");
    install.initialize().await.expect("failed to initialize");

    let stats = install.stats().await;
    eprintln!(
        "  {} index entries, {} archives ({} bytes)",
        stats.index_entries, stats.archive_files, stats.archive_size,
    );
    install
}

fn load_resolution_files(install: &Installation) {
    let build_id = find_build_id();
    let cache = PathBuf::from(CACHE_DIR).join(format!("wow-{build_id}"));

    let root_path = cache.join("root.bin");
    let encoding_path = cache.join("encoding.bin");

    load_root(install, &root_path);
    load_encoding(install, &encoding_path);
}

fn find_build_id() -> String {
    let id_file = PathBuf::from(CACHE_DIR).join("build-id.txt");
    std::fs::read_to_string(&id_file)
        .unwrap_or_else(|_| panic!("Missing {}, run `casc-extract init` first", id_file.display()))
        .trim()
        .to_string()
}

fn load_root(install: &Installation, path: &Path) {
    let data = std::fs::read(path)
        .unwrap_or_else(|_| panic!("Missing {}, run `casc-extract init`", path.display()));
    install.load_root_file(&data).expect("failed to load root file");
    eprintln!("Loaded root file ({} bytes)", data.len());
}

fn load_encoding(install: &Installation, path: &Path) {
    let data = std::fs::read(path)
        .unwrap_or_else(|_| panic!("Missing {}, run `casc-extract init`", path.display()));
    install.load_encoding_file(&data).expect("failed to load encoding file");
    eprintln!("Loaded encoding file ({} bytes)", data.len());
}

async fn extract_fdid(
    install: &Installation,
    fdid: u32,
    output_dir: &Path,
) -> Result<PathBuf, String> {
    let filename = resolve_filename(fdid);
    let out_path = output_dir.join(&filename);

    if out_path.exists() {
        return Ok(out_path);
    }

    // Full chain: FDID → ContentKey → EncodingKey → local archive
    let encoding_key = install
        .resolver()
        .resolve_fdid_to_encoding(fdid)
        .ok_or_else(|| format!("FDID {fdid} not found in root/encoding files"))?;

    let data = install
        .read_file_by_encoding_key(&encoding_key)
        .await
        .map_err(|e| format!("{e}"))?;

    std::fs::create_dir_all(output_dir).map_err(|e| format!("mkdir: {e}"))?;
    std::fs::write(&out_path, &data).map_err(|e| format!("write: {e}"))?;
    Ok(out_path)
}

/// Resolve FDID to a filename. Uses FDID-based naming for consistency
/// with wow-engine's `data/models/{fdid}.m2` convention.
fn resolve_filename(fdid: u32) -> String {
    let listfile_path = PathBuf::from(CACHE_DIR).join("listfile.csv");
    let ext = resolve_extension(fdid, &listfile_path);
    format!("{fdid}.{ext}")
}

fn resolve_extension(fdid: u32, listfile_path: &Path) -> String {
    if let Ok(content) = std::fs::read_to_string(listfile_path) {
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
