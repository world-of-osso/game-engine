use std::{
    env,
    ffi::OsString,
    fs,
    os::unix::{ffi::OsStrExt, fs::PermissionsExt, process::CommandExt},
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
    let args = route_arguments(env::args_os().skip(1))?;

    let status = build_native_extension(root)?;
    if !status.success() {
        return Ok(exit_code(status));
    }
    exec_godot(root, &godot, args)
}

fn route_arguments(args: impl IntoIterator<Item = OsString>) -> Result<Vec<OsString>, String> {
    let mut args = args.into_iter();
    let mut engine = Vec::new();
    let mut client = Vec::new();
    let mut has_separator = false;

    while let Some(arg) = args.next() {
        if arg == "--" {
            has_separator = true;
            client.extend(args);
            break;
        }
        if ["--screen", "--state", "--server", "--char"].contains(&arg.to_str().unwrap_or("")) {
            let value = args
                .next()
                .filter(|value| !value.as_bytes().starts_with(b"-"));
            let value =
                value.ok_or_else(|| format!("{} requires a value", arg.to_string_lossy()))?;
            client.push(arg);
            client.push(value);
        } else {
            engine.push(arg);
        }
    }

    if has_separator || !client.is_empty() {
        engine.push("--".into());
        engine.extend(client);
    }
    Ok(engine)
}

fn build_native_extension(root: &Path) -> Result<ExitStatus, String> {
    let cargo = env::var_os("CARGO").ok_or("CARGO is unset; launch through cargo run")?;
    Command::new(cargo)
        .current_dir(root)
        .env("CARGO_TARGET_DIR", root.join("target"))
        .arg("build")
        .arg("--manifest-path")
        .arg(root.join("godot/Cargo.toml"))
        .args(["-p", "game-engine-godot", "--lib"])
        .status()
        .map_err(|error| format!("cannot run Cargo native build: {error}"))
}

fn exec_godot(root: &Path, godot: &Path, args: Vec<OsString>) -> Result<i32, String> {
    let dependencies = root.join("target/debug/deps");
    let mut library_path = dependencies.into_os_string();
    if let Some(existing) = env::var_os("LD_LIBRARY_PATH") {
        library_path.push(":");
        library_path.push(existing);
    }
    let error = Command::new(godot)
        .arg("--path")
        .arg(root.join("godot"))
        .args(args)
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
