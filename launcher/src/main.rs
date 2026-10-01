use std::{
    env,
    ffi::{OsStr, OsString},
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
    let root = match env::var_os("GAME_ENGINE_ROOT") {
        Some(path) => PathBuf::from(path),
        None => Path::new(env!("CARGO_MANIFEST_DIR"))
            .parent()
            .ok_or("launcher manifest has no checkout parent")?
            .to_path_buf(),
    };
    let root = root.as_path();
    let godot = match env::var_os("GODOT_BIN") {
        Some(path) => PathBuf::from(path),
        None => ensure_cached_godot()?,
    };
    validate_godot(&godot)?;
    let args = route_arguments(env::args_os().skip(1))?;

    let status = build_native_extension(root)?;
    if !status.success() {
        return Ok(exit_code(status));
    }
    let status = import_project_once(root, &godot)?;
    if !status.success() {
        return Ok(exit_code(status));
    }
    exec_godot(root, &godot, args)
}

/// A fresh checkout has no `godot/.godot` cache; without it Godot cannot load the
/// native extension classes, so import once before the first launch.
fn import_project_once(root: &Path, godot: &Path) -> Result<ExitStatus, String> {
    let project = root.join("godot");
    if project.join(".godot/extension_list.cfg").is_file() {
        return Ok(ExitStatus::default());
    }
    eprintln!(
        "game-engine-launcher: importing Godot project {}",
        project.display()
    );
    Command::new(godot)
        .args(["--headless", "--import", "--path"])
        .arg(&project)
        .env("LD_LIBRARY_PATH", native_library_path(root))
        .status()
        .map_err(|error| format!("cannot run Godot import: {error}"))
}

fn native_library_path(root: &Path) -> OsString {
    let mut library_path = root.join("target/debug/deps").into_os_string();
    if let Some(existing) = env::var_os("LD_LIBRARY_PATH") {
        library_path.push(":");
        library_path.push(existing);
    }
    library_path
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
        if is_client_option(&arg) {
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

fn is_client_option(argument: &OsStr) -> bool {
    matches!(
        argument.to_str(),
        Some("--screen" | "--state" | "--server" | "--char" | "--run-js-ui-script")
    )
}

fn build_native_extension(root: &Path) -> Result<ExitStatus, String> {
    Command::new("python3")
        .current_dir(root)
        .arg(root.join("scripts/depot-build.py"))
        .arg("--root")
        .arg(root)
        .status()
        .map_err(|error| format!("cannot run Depot native build: {error}"))
}

fn exec_godot(root: &Path, godot: &Path, args: Vec<OsString>) -> Result<i32, String> {
    let error = Command::new(godot)
        .arg("--path")
        .arg(root.join("godot"))
        .args(args)
        .env("LD_LIBRARY_PATH", native_library_path(root))
        .exec();
    Err(format!("cannot launch Godot {}: {error}", godot.display()))
}

const GODOT_VERSION: &str = "4.7.2";
const GODOT_EXECUTABLE: &str = "Godot_v4.7.2-stable_linux.x86_64";
const GODOT_ZIP_URL: &str = "https://github.com/godotengine/godot/releases/download/4.7.2-stable/Godot_v4.7.2-stable_linux.x86_64.zip";
/// From the release's SHA512-SUMS.txt.
const GODOT_ZIP_SHA512: &str = "9aa00f7a605200940bce3027a567b782f49bd8e940dd06ae9e987bd65aee1b1467edd56ed84fcdcbdd44354bf613bdbb4e5d2913e925850368e150c59ed54c65";

/// Pinned Godot under the user cache, downloaded and checksum-verified on first use.
fn ensure_cached_godot() -> Result<PathBuf, String> {
    let version_dir = godot_cache_dir()?.join(GODOT_VERSION);
    let executable = version_dir.join(GODOT_EXECUTABLE);
    if executable.is_file() {
        return Ok(executable);
    }
    eprintln!(
        "game-engine-launcher: downloading Godot {GODOT_VERSION} to {}",
        version_dir.display()
    );
    let staging = version_dir.with_extension(format!("partial-{}", process::id()));
    let installed = install_godot(&staging).and_then(|()| {
        fs::rename(&staging, &version_dir)
            .map_err(|error| format!("cannot install {}: {error}", version_dir.display()))
    });
    if staging.exists() {
        fs::remove_dir_all(&staging)
            .map_err(|error| format!("cannot remove {}: {error}", staging.display()))?;
    }
    installed?;
    Ok(executable)
}

fn godot_cache_dir() -> Result<PathBuf, String> {
    let cache = match env::var_os("XDG_CACHE_HOME").filter(|value| !value.is_empty()) {
        Some(path) => PathBuf::from(path),
        None => PathBuf::from(env::var_os("HOME").ok_or("HOME is unset")?).join(".cache"),
    };
    Ok(cache.join("game-engine/godot"))
}

fn install_godot(staging: &Path) -> Result<(), String> {
    fs::create_dir_all(staging)
        .map_err(|error| format!("cannot create {}: {error}", staging.display()))?;
    let zip = staging.join("godot.zip");
    run_tool(
        Command::new("curl")
            .args([
                "--fail",
                "--location",
                "--silent",
                "--show-error",
                "--output",
            ])
            .arg(&zip)
            .arg(GODOT_ZIP_URL),
    )?;
    verify_sha512(&zip)?;
    run_tool(
        Command::new("unzip")
            .arg("-q")
            .arg(&zip)
            .arg(GODOT_EXECUTABLE)
            .arg("-d")
            .arg(staging),
    )?;
    fs::remove_file(&zip).map_err(|error| format!("cannot remove {}: {error}", zip.display()))
}

fn verify_sha512(zip: &Path) -> Result<(), String> {
    let output = Command::new("sha512sum")
        .arg(zip)
        .output()
        .map_err(|error| format!("cannot run sha512sum: {error}"))?;
    if !output.status.success() {
        return Err(format!("sha512sum failed on {}", zip.display()));
    }
    let actual = String::from_utf8_lossy(&output.stdout);
    let actual = actual.split_whitespace().next().unwrap_or("");
    if actual != GODOT_ZIP_SHA512 {
        return Err(format!(
            "Godot download {GODOT_ZIP_URL} has SHA-512 {actual}, expected {GODOT_ZIP_SHA512}"
        ));
    }
    Ok(())
}

fn run_tool(command: &mut Command) -> Result<(), String> {
    let program = command.get_program().to_string_lossy().into_owned();
    let status = command
        .status()
        .map_err(|error| format!("cannot run {program}: {error}"))?;
    if !status.success() {
        return Err(format!("{program} exited with {status}"));
    }
    Ok(())
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
