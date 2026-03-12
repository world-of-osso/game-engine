//! Extract files from local WoW CASC storage by FileDataID.
//!
//! Usage:
//!   cargo run --bin casc-local -- <fdid> [fdid2 ...] [-o output_dir]

use cascette_client_storage::Installation;
use std::path::{Path, PathBuf};

const WOW_PATH: &str = "/syncthing/World of Warcraft";
const CACHE_DIR: &str = "/home/osso/.cache/casc-resolver";
const LISTFILE_PATH: &str = "data/community-listfile.csv";

#[tokio::main]
async fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let (fdids, output_dir) = parse_args(&args);
    if fdids.is_empty() {
        eprintln!("Usage: casc-local <fdid> [fdid2 ...] [-o output_dir]");
        std::process::exit(1);
    }

    let data_root = PathBuf::from(WOW_PATH).join("Data");
    let build_key = read_active_build_key(&data_root);
    let install = open_and_initialize(&data_root).await;
    load_cached_resolution(&install, &build_key);

    let (mut ok, mut fail) = (0u32, 0u32);
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

async fn open_and_initialize(data_root: &Path) -> Installation {
    eprintln!("Opening local CASC: {}", data_root.display());
    let install =
        Installation::open(data_root.to_path_buf()).expect("failed to open CASC installation");
    install.initialize().await.expect("failed to initialize");
    install
}

fn load_cached_resolution(install: &Installation, build_key: &str) {
    let cache = PathBuf::from(CACHE_DIR).join(build_key);
    let root_data = std::fs::read(cache.join("root.bin"))
        .unwrap_or_else(|_| panic!("Missing root.bin, run `casc-init` first"));
    install
        .load_root_file(&root_data)
        .expect("failed to load root");
    eprintln!("Loaded root ({:.1} MB)", root_data.len() as f64 / 1e6);

    let enc_data = std::fs::read(cache.join("encoding.bin"))
        .unwrap_or_else(|_| panic!("Missing encoding.bin, run `casc-init` first"));
    install
        .load_encoding_file(&enc_data)
        .expect("failed to load encoding");
    eprintln!("Loaded encoding ({:.1} MB)", enc_data.len() as f64 / 1e6);
}

fn read_active_build_key(data_root: &Path) -> String {
    let info_path = data_root.parent().unwrap().join(".build.info");
    let content = std::fs::read_to_string(&info_path).expect("read .build.info");
    let mut lines = content.lines();
    let header = lines.next().expect("empty .build.info");
    let cols: Vec<&str> = header.split('|').collect();
    let key_idx = cols
        .iter()
        .position(|c| c.starts_with("Build Key"))
        .expect("no Build Key column");
    let prod_idx = cols.iter().position(|c| c.starts_with("Product"));

    for line in lines {
        let vals: Vec<&str> = line.split('|').collect();
        let is_wow = prod_idx
            .and_then(|i| vals.get(i))
            .is_some_and(|p| *p == "wow");
        if is_wow {
            return vals[key_idx].to_string();
        }
    }
    panic!("no wow product in .build.info");
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

    let encoding_key = install
        .resolver()
        .resolve_fdid_to_encoding(fdid)
        .ok_or_else(|| format!("FDID {fdid} not found in root/encoding"))?;

    let data = install
        .read_file_by_encoding_key(&encoding_key)
        .await
        .map_err(|e| format!("{e}"))?;

    std::fs::create_dir_all(output_dir).map_err(|e| format!("mkdir: {e}"))?;
    std::fs::write(&out_path, &data).map_err(|e| format!("write: {e}"))?;
    Ok(out_path)
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
