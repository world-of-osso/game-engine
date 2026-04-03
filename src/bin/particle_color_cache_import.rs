use std::path::PathBuf;
use std::process::Command;

const PARTICLE_COLOR_WAGO_URL: &str = "https://wago.tools/db2/ParticleColor/csv";

fn ensure_wago_csv(dest: &std::path::Path) -> Result<(), String> {
    if dest.exists() {
        return Ok(());
    }
    if let Some(parent) = dest.parent() {
        std::fs::create_dir_all(parent)
            .map_err(|err| format!("create {}: {err}", parent.display()))?;
    }
    let status = Command::new("curl")
        .args([
            "--fail",
            "--silent",
            "--show-error",
            "--location",
            PARTICLE_COLOR_WAGO_URL,
            "--output",
        ])
        .arg(dest)
        .status()
        .map_err(|err| format!("spawn curl for {PARTICLE_COLOR_WAGO_URL}: {err}"))?;
    if !status.success() {
        return Err(format!(
            "curl failed downloading {PARTICLE_COLOR_WAGO_URL} to {} with status {status}",
            dest.display()
        ));
    }
    Ok(())
}

fn source_path(fetch_wago: bool) -> Result<PathBuf, String> {
    if fetch_wago {
        let shared = game_engine::paths::shared_data_path("ParticleColor.csv");
        ensure_wago_csv(&shared)?;
        return Ok(shared);
    }
    Ok(game_engine::particle_color_cache::particle_color_csv_path())
}

fn main() {
    let fetch_wago = std::env::args().skip(1).any(|arg| arg == "--fetch-wago");
    let source_path = match source_path(fetch_wago) {
        Ok(path) => path,
        Err(err) => {
            eprintln!("Failed to prepare particle color CSV: {err}");
            std::process::exit(1);
        }
    };
    match game_engine::particle_color_cache::import_particle_color_cache_from_source(&source_path) {
        Ok(path) => println!("Imported particle color cache at {}", path.display()),
        Err(err) => {
            eprintln!("Failed to import particle color cache: {err}");
            std::process::exit(1);
        }
    }
}
