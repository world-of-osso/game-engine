use cascette_client_storage::Installation;
use cascette_client_storage::resolver::ContentResolver;
use cascette_crypto::{ContentKey, EncodingKey};
use std::path::{Path, PathBuf};

const WOW_PATH: &str = "/syncthing/World of Warcraft";
const OUT_DIR: &str = "data/casc";

#[tokio::main]
async fn main() {
    if let Err(err) = run().await {
        eprintln!("{err}");
        std::process::exit(1);
    }
}

async fn run() -> Result<(), String> {
    let data_root = casc_data_root();
    let refresh = resolve_refresh_targets(&data_root)?;
    let install = open_local_installation(&data_root).await?;
    let encoding_data = read_encoding_data(&install, &refresh.encoding_ekey).await?;
    let root_data = read_root_data(&install, &encoding_data, &refresh.root_ckey).await?;
    write_refresh_outputs(&encoding_data, &root_data)?;
    eprintln!("Refreshed {OUT_DIR}/encoding.bin and {OUT_DIR}/root.bin from local CASC");
    Ok(())
}

struct CascRefreshTargets {
    root_ckey: ContentKey,
    encoding_ekey: EncodingKey,
}

fn casc_data_root() -> PathBuf {
    PathBuf::from(WOW_PATH).join("Data")
}

fn resolve_refresh_targets(data_root: &Path) -> Result<CascRefreshTargets, String> {
    let build_key = read_active_build_key(&PathBuf::from(WOW_PATH).join(".build.info"))?;
    let build_cfg = read_build_config(&data_root.join("config"), &build_key)?;
    Ok(CascRefreshTargets {
        root_ckey: parse_single_key(&build_cfg, "root")?,
        encoding_ekey: parse_second_key(&build_cfg, "encoding")?,
    })
}

async fn open_local_installation(data_root: &Path) -> Result<Installation, String> {
    eprintln!("Opening local CASC: {}", data_root.display());
    let install = Installation::open(data_root.to_path_buf())
        .map_err(|e| format!("failed to open installation: {e}"))?;
    install
        .initialize()
        .await
        .map_err(|e| format!("failed to initialize installation: {e}"))?;
    Ok(install)
}

async fn read_encoding_data(
    install: &Installation,
    encoding_ekey: &EncodingKey,
) -> Result<Vec<u8>, String> {
    let encoding_data = install
        .read_file_by_encoding_key(encoding_ekey)
        .await
        .map_err(|e| format!("failed to read encoding by local encoding key: {e}"))?;
    eprintln!(
        "Read encoding.bin from local archives: {} bytes",
        encoding_data.len()
    );
    Ok(encoding_data)
}

async fn read_root_data(
    install: &Installation,
    encoding_data: &[u8],
    root_ckey: &ContentKey,
) -> Result<Vec<u8>, String> {
    let resolver = load_encoding_resolver(encoding_data)?;
    let root_ekey = resolver
        .resolve_content_key(root_ckey)
        .ok_or_else(|| format!("root content key {root_ckey} not found in fresh encoding file"))?;
    let root_data = install
        .read_file_by_encoding_key(&root_ekey)
        .await
        .map_err(|e| format!("failed to read root by resolved encoding key {root_ekey}: {e}"))?;
    eprintln!(
        "Read root.bin from local archives: {} bytes",
        root_data.len()
    );
    Ok(root_data)
}

fn load_encoding_resolver(encoding_data: &[u8]) -> Result<ContentResolver, String> {
    let resolver = ContentResolver::new();
    resolver
        .load_encoding_file(encoding_data)
        .map_err(|e| format!("failed to load fresh encoding file: {e}"))?;
    Ok(resolver)
}

fn write_refresh_outputs(encoding_data: &[u8], root_data: &[u8]) -> Result<(), String> {
    let out_dir = PathBuf::from(OUT_DIR);
    std::fs::create_dir_all(&out_dir).map_err(|e| format!("mkdir {}: {e}", out_dir.display()))?;
    std::fs::write(out_dir.join("encoding.bin"), encoding_data)
        .map_err(|e| format!("write encoding.bin: {e}"))?;
    std::fs::write(out_dir.join("root.bin"), root_data)
        .map_err(|e| format!("write root.bin: {e}"))?;
    Ok(())
}

fn read_active_build_key(build_info_path: &Path) -> Result<String, String> {
    let content = std::fs::read_to_string(build_info_path)
        .map_err(|e| format!("read {}: {e}", build_info_path.display()))?;
    let mut lines = content.lines();
    let header = lines.next().ok_or("empty .build.info")?;
    let cols: Vec<&str> = header.split('|').collect();
    let key_idx = cols
        .iter()
        .position(|c| c.starts_with("Build Key"))
        .ok_or("no Build Key column in .build.info")?;
    let prod_idx = cols
        .iter()
        .position(|c| c.starts_with("Product"))
        .ok_or("no Product column in .build.info")?;

    for line in lines {
        let vals: Vec<&str> = line.split('|').collect();
        if vals.get(prod_idx).copied() == Some("wow") {
            return vals
                .get(key_idx)
                .map(|s| s.to_string())
                .ok_or_else(|| "missing build key value for wow".to_string());
        }
    }

    Err("no wow row in .build.info".to_string())
}

fn read_build_config(config_root: &Path, build_key: &str) -> Result<String, String> {
    if build_key.len() < 4 {
        return Err(format!("invalid build key: {build_key}"));
    }
    let cfg_path = config_root
        .join(&build_key[0..2])
        .join(&build_key[2..4])
        .join(build_key);
    std::fs::read_to_string(&cfg_path).map_err(|e| format!("read {}: {e}", cfg_path.display()))
}

fn parse_single_key(config: &str, field: &str) -> Result<ContentKey, String> {
    let raw = config_line_value(config, field)?;
    let key = raw
        .split_whitespace()
        .next()
        .ok_or_else(|| format!("missing {field} key"))?;
    ContentKey::from_hex(key).map_err(|e| format!("invalid {field} content key: {e}"))
}

fn parse_second_key(config: &str, field: &str) -> Result<EncodingKey, String> {
    let raw = config_line_value(config, field)?;
    let mut parts = raw.split_whitespace();
    let _content = parts
        .next()
        .ok_or_else(|| format!("missing {field} content key"))?;
    let encoding = parts
        .next()
        .ok_or_else(|| format!("missing {field} encoding key"))?;
    EncodingKey::from_hex(encoding).map_err(|e| format!("invalid {field} encoding key: {e}"))
}

fn config_line_value<'a>(config: &'a str, field: &str) -> Result<&'a str, String> {
    let prefix = format!("{field} = ");
    config
        .lines()
        .find_map(|line| line.strip_prefix(&prefix))
        .ok_or_else(|| format!("missing `{field}` in build config"))
}
