//! Extract files from local WoW CASC storage by FileDataID.
//!
//! Usage:
//!   cargo run --bin casc-local -- <fdid> [fdid2 ...] [-o output_dir]

use binrw::BinRead;
use cascette_client_storage::Installation;
use cascette_client_storage::index::IndexManager;
use cascette_client_storage::resolver::ContentResolver;
use cascette_client_storage::storage::ArchiveManager;
use cascette_crypto::EncodingKey;
use cascette_crypto::TactKeyStore;
use cascette_formats::blte::BlteFile;
use std::path::{Path, PathBuf};

const WOW_PATH: &str = "/syncthing/World of Warcraft";
const CACHE_DIR: &str = "data/casc";
const LISTFILE_PATH: &str = "data/community-listfile.csv";
const LOCAL_CASC_HEADER_SIZE: usize = 30;
const EXTERNAL_TACT_KEYS_PATH: &str = "data/tactkeys/WoW.txt";

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
    let resolver = load_cached_resolution();

    let (mut ok, mut fail) = (0u32, 0u32);
    for fdid in &fdids {
        match extract_fdid(&install, &resolver, *fdid, &output_dir).await {
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

async fn open_and_initialize(data_root: &Path) -> Installation {
    eprintln!("Opening local CASC: {}", data_root.display());
    let install =
        Installation::open(data_root.to_path_buf()).expect("failed to open CASC installation");
    install.initialize().await.expect("failed to initialize");
    install
}

fn load_cached_resolution() -> ContentResolver {
    let cache = PathBuf::from(CACHE_DIR);
    let resolver = ContentResolver::new();
    let root_data = std::fs::read(cache.join("root.bin"))
        .unwrap_or_else(|_| panic!("Missing root.bin, run `casc-init` first"));
    resolver
        .load_root_file(&root_data)
        .expect("failed to load root");
    eprintln!("Loaded root ({:.1} MB)", root_data.len() as f64 / 1e6);

    let enc_data = std::fs::read(cache.join("encoding.bin"))
        .unwrap_or_else(|_| panic!("Missing encoding.bin, run `casc-init` first"));
    resolver
        .load_encoding_file(&enc_data)
        .expect("failed to load encoding");
    eprintln!("Loaded encoding ({:.1} MB)", enc_data.len() as f64 / 1e6);
    resolver
}

async fn extract_fdid(
    install: &Installation,
    resolver: &ContentResolver,
    fdid: u32,
    output_dir: &Path,
) -> Result<PathBuf, String> {
    let filename = resolve_filename(fdid);
    let out_path = output_dir.join(&filename);
    if out_path.exists() {
        return Ok(out_path);
    }

    let content_key = resolver
        .resolve_file_data_id(fdid)
        .ok_or_else(|| format!("FDID {fdid}: missing content key in root"))?;
    let encoding_key: EncodingKey = resolver
        .resolve_content_key(&content_key)
        .ok_or_else(|| format!("FDID {fdid}: missing encoding key for content {content_key}"))?;
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
    if let Ok(content) = std::fs::read_to_string(EXTERNAL_TACT_KEYS_PATH) {
        let loaded = keys.load_from_txt(&content);
        if loaded > 0 {
            eprintln!("Loaded {loaded} external TACT keys from {EXTERNAL_TACT_KEYS_PATH}");
        }
    }
    keys
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
