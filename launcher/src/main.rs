use std::{
    env, fs,
    os::unix::{fs::PermissionsExt, process::CommandExt},
    path::{Path, PathBuf},
    process::{self, Command, ExitStatus},
};

fn main() {
    match launch() {
        Ok(code) => process::exit(code),
        Err(error) => {
            eprintln!("game-engine-launcher: {error}");
            process::exit(1);
        }
    }
}

fn launch() -> Result<i32, String> {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .ok_or("launcher manifest has no checkout parent")?;
    let godot = match env::var_os("GODOT_BIN") {
        Some(path) => PathBuf::from(path),
        None => root.join("data/tools/godot/4.7.2/Godot_v4.7.2-stable_linux.x86_64"),
    };
    validate_godot(&godot)?;

    let cargo = env::var_os("CARGO").ok_or("CARGO is unset; launch through cargo run")?;
    let status = Command::new(cargo)
        .current_dir(root)
        .env("CARGO_TARGET_DIR", root.join("target"))
        .arg("build")
        .arg("--manifest-path")
        .arg(root.join("godot/Cargo.toml"))
        .args(["-p", "game-engine-godot", "--lib"])
        .status()
        .map_err(|error| format!("cannot run Cargo native build: {error}"))?;
    if !status.success() {
        return Ok(exit_code(status));
    }

    let dependencies = root.join("target/debug/deps");
    let mut library_path = dependencies.into_os_string();
    if let Some(existing) = env::var_os("LD_LIBRARY_PATH") {
        library_path.push(":");
        library_path.push(existing);
    }
    let error = Command::new(&godot)
        .arg("--path")
        .arg(root.join("godot"))
        .args(env::args_os().skip(1))
        .env("LD_LIBRARY_PATH", library_path)
        .exec();
    Err(format!("cannot launch Godot {}: {error}", godot.display()))
}

fn validate_godot(path: &Path) -> Result<(), String> {
    let metadata = fs::metadata(path)
        .map_err(|error| format!("Godot executable {} (GODOT_BIN): {error}", path.display()))?;
    if !metadata.is_file() || metadata.permissions().mode() & 0o111 == 0 {
        return Err(format!(
            "Godot executable {} (GODOT_BIN) is not an executable file",
            path.display()
        ));
    }
    Ok(())
}

fn exit_code(status: ExitStatus) -> i32 {
    use std::os::unix::process::ExitStatusExt;
    match status.code() {
        Some(code) => code,
        None => match status.signal() {
            Some(signal) => 128 + signal,
            None => 1,
        },
    }
}
