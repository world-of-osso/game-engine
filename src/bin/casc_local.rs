//! Extract files from local WoW CASC storage by FileDataID.
//!
//! Usage:
//!   cargo run --bin casc-local -- <fdid> [fdid2 ...] [-o output_dir]

use binrw::BinRead;
use cascette_client_storage::Installation;
use cascette_client_storage::index::IndexManager;
use cascette_client_storage::storage::ArchiveManager;
use cascette_crypto::EncodingKey;
use cascette_crypto::TactKeyStore;
use cascette_formats::blte::BlteFile;
use game_engine::listfile;
use game_engine::paths;
use osso_asset_resolver::casc_cache::CascResolutionCache;
use std::path::{Path, PathBuf};

const WOW_PATH: &str = "/syncthing/World of Warcraft";
const LOCAL_CASC_HEADER_SIZE: usize = 30;

#[tokio::main]
async fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let (fdids, output_dir) = parse_args(&args);
    if fdids.is_empty() {
        eprintln!("Usage: casc-local <fdid> [fdid2 ...] [-o output_dir]");
        std::process::exit(1);
    }

    let data_root = PathBuf::from(WOW_PATH).join("Data");
    let install = open_and_initialize(&data_root).await;
    let cache_dir = paths::shared_data_path("casc");
    let cache =
        CascResolutionCache::open(&cache_dir).expect("failed to open CASC resolution cache");

    let (mut ok, mut fail) = (0u32, 0u32);
    for fdid in &fdids {
        match extract_fdid(&install, &cache, *fdid, &output_dir).await {
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
    (fdids, resolve_output_dir(&output_dir))
}

async fn open_and_initialize(data_root: &Path) -> Installation {
    eprintln!("Opening local CASC: {}", data_root.display());
    let install =
        Installation::open(data_root.to_path_buf()).expect("failed to open CASC installation");
    install.initialize().await.expect("failed to initialize");
    install
}

async fn extract_fdid(
    install: &Installation,
    cache: &CascResolutionCache,
    fdid: u32,
    output_dir: &Path,
) -> Result<PathBuf, String> {
    let filename = resolve_filename(fdid);
    let out_path = output_dir.join(&filename);
    if out_path.exists() {
        return Ok(out_path);
    }

    let (_, ek_bytes) = cache
        .resolve_fdid(fdid)
        .ok_or_else(|| format!("FDID {fdid}: missing resolution entry"))?;
    let encoding_key = EncodingKey::from_bytes(ek_bytes);
    let data = match install.read_file_by_encoding_key(&encoding_key).await {
        Ok(data) => data,
        Err(primary_err) => read_file_by_encoding_key_with_keys(&encoding_key)
            .await
            .map_err(|fallback_err| format!("FDID {fdid}: {primary_err}; key-aware local archive fallback also failed: {fallback_err}"))?,
    };

    std::fs::create_dir_all(output_dir).map_err(|e| format!("mkdir: {e}"))?;
    std::fs::write(&out_path, &data).map_err(|e| format!("write: {e}"))?;
    Ok(out_path)
}

async fn read_file_by_encoding_key_with_keys(
    encoding_key: &EncodingKey,
) -> Result<Vec<u8>, String> {
    let data_dir = PathBuf::from(WOW_PATH).join("Data").join("data");
    let mut indices = IndexManager::new(&data_dir);
    indices
        .load_all()
        .await
        .map_err(|e| format!("load CASC indices: {e}"))?;
    let mut archives = ArchiveManager::new(&data_dir);
    archives
        .open_all()
        .await
        .map_err(|e| format!("open CASC archives: {e}"))?;
    let keys = load_tact_keys();

    let index_entry = indices
        .lookup(encoding_key)
        .ok_or_else(|| format!("missing archive location for encoding key {encoding_key}"))?;
    decode_archive_entry_with_keys(&archives, &keys, &index_entry)
}

fn load_tact_keys() -> TactKeyStore {
    let mut keys = TactKeyStore::new();
    let key_path = paths::shared_data_path("tactkeys/WoW.txt");
    if let Ok(content) = std::fs::read_to_string(&key_path) {
        let loaded = keys.load_from_txt(&content);
        if loaded > 0 {
            eprintln!(
                "Loaded {loaded} external TACT keys from {}",
                key_path.display()
            );
        }
    }
    keys
}

fn resolve_output_dir(path: &Path) -> PathBuf {
    paths::remap_to_shared_data_path(path)
}

fn decode_archive_entry_with_keys(
    archives: &ArchiveManager,
    keys: &TactKeyStore,
    index_entry: &cascette_client_storage::index::IndexEntry,
) -> Result<Vec<u8>, String> {
    let raw_blte = archives
        .read_raw(
            index_entry.archive_id(),
            index_entry.archive_offset(),
            index_entry.size,
        )
        .map_err(|e| format!("read raw BLTE archive entry: {e}"))?;
    let blte_bytes = if raw_blte.len() >= LOCAL_CASC_HEADER_SIZE + 4
        && &raw_blte[LOCAL_CASC_HEADER_SIZE..LOCAL_CASC_HEADER_SIZE + 4] == b"BLTE"
    {
        &raw_blte[LOCAL_CASC_HEADER_SIZE..]
    } else {
        raw_blte.as_slice()
    };
    let blte = BlteFile::read_options(
        &mut std::io::Cursor::new(blte_bytes),
        binrw::Endian::Big,
        (),
    )
    .map_err(|e| format!("parse BLTE container: {e}"))?;
    blte.decompress_with_keys(keys)
        .map_err(|e| format!("decrypt/decompress BLTE container: {e}"))
}

fn resolve_filename(fdid: u32) -> String {
    let ext = resolve_extension(fdid);
    format!("{fdid}.{ext}")
}

fn resolve_extension(fdid: u32) -> String {
    if let Some(path) = listfile::lookup_fdid(fdid)
        && let Some(ext) = extension_from_listfile_path(path)
    {
        return ext;
    }
    "dat".to_string()
}

fn extension_from_listfile_path(path: &str) -> Option<String> {
    path.rsplit('.').next().map(|ext| ext.to_ascii_lowercase())
}

#[cfg(test)]
mod tests {
    use super::{extension_from_listfile_path, resolve_output_dir};
    use std::path::{Path, PathBuf};

    #[test]
    fn resolves_extension_from_listfile_path_case_insensitively() {
        assert_eq!(
            extension_from_listfile_path("World/Maps/Test/Tile_1_2.ADT"),
            Some("adt".to_string())
        );
    }

    #[test]
    fn resolves_extension_from_multi_dot_listfile_path() {
        assert_eq!(
            extension_from_listfile_path("spells/test.texture.BLP"),
            Some("blp".to_string())
        );
    }

    #[test]
    fn remaps_data_output_dir_to_shared_root() {
        unsafe {
            std::env::set_var(
                "GAME_ENGINE_SHARED_DATA_DIR",
                "/tmp/game-engine-shared-data",
            );
        }
        assert_eq!(
            resolve_output_dir(Path::new("data/textures")),
            PathBuf::from("/tmp/game-engine-shared-data/textures")
        );
        unsafe {
            std::env::remove_var("GAME_ENGINE_SHARED_DATA_DIR");
        }
    }
}
