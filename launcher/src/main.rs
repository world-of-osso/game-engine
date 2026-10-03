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
        None => pinned_godot(root)?,
    };
    validate_godot(&godot)?;
    let (args, build_host) = route_arguments(env::args_os().skip(1))?;

    let status = build_native_extension(root, build_host.as_deref())?;
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

fn route_arguments(
    args: impl IntoIterator<Item = OsString>,
) -> Result<(Vec<OsString>, Option<OsString>), String> {
    let mut args = args.into_iter();
    let mut engine = Vec::new();
    let mut client = Vec::new();
    let mut has_separator = false;
    let mut build_host = None;

    while let Some(arg) = args.next() {
        if arg == "--" {
            has_separator = true;
            client.extend(args);
            break;
        }
        if arg == "--build-host" {
            let value = consume_option_value(&mut args, &arg)?;
            build_host = Some(validate_build_host(value)?);
        } else if arg == "--skybox-verify" {
            client.push(arg);
        } else if is_client_option(&arg) {
            let value = consume_option_value(&mut args, &arg)?;
            client.extend([arg, value]);
        } else {
            engine.push(arg);
        }
    }

    if has_separator || !client.is_empty() {
        engine.push("--".into());
        engine.extend(client);
    }
    Ok((engine, build_host))
}

fn consume_option_value(
    args: &mut impl Iterator<Item = OsString>,
    option: &OsStr,
) -> Result<OsString, String> {
    args.next()
        .filter(|value| !value.as_bytes().starts_with(b"-"))
        .ok_or_else(|| format!("{} requires a value", option.to_string_lossy()))
}

fn validate_build_host(value: OsString) -> Result<OsString, String> {
    if matches!(value.to_str(), Some("desktop" | "local")) {
        Ok(value)
    } else {
        Err(format!(
            "invalid --build-host {:?}; expected desktop or local",
            value
        ))
    }
}

fn is_client_option(argument: &OsStr) -> bool {
    matches!(
        argument.to_str(),
        Some(
            "--screen"
                | "--state"
                | "--server"
                | "--char"
                | "--run-js-ui-script"
                | "--skybox-fdid"
                | "--light-skybox-id"
                | "--skybox-time-ms"
        )
    )
}

fn build_native_extension(root: &Path, build_host: Option<&OsStr>) -> Result<ExitStatus, String> {
    let mut command = Command::new("python3");
    command
        .current_dir(root)
        .arg(root.join("scripts/depot-build.py"))
        .arg("--root")
        .arg(root);
    if let Some(host) = build_host {
        command.arg("--build-host").arg(host);
    }
    command
        .status()
        .map_err(|error| format!("cannot run native build: {error}"))
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

const GODOT_VERSION: &str = "4.7.2-pr123946-pr123546";
const GODOT_BUILD_SCRIPT: &str = "scripts/godot/build-patched-godot.sh";
/// Official 4.7.2 plus upstream godotengine/godot#123946 and #123546, as built by `GODOT_BUILD_SCRIPT`.
const GODOT_SHA512: &str = include_str!("../../scripts/godot/godot-4.7.2-pr123946-pr123546.sha512");

/// Pinned patched Godot under the user cache, checksum-verified on every launch.
fn pinned_godot(root: &Path) -> Result<PathBuf, String> {
    let executable = godot_cache_dir()?
        .join(GODOT_VERSION)
        .join(format!("godot-{GODOT_VERSION}"));
    verify_pinned_godot(&executable, GODOT_SHA512.trim()).map_err(|error| {
        format!(
            "{error}; build Godot {GODOT_VERSION} with {}",
            root.join(GODOT_BUILD_SCRIPT).display()
        )
    })?;
    eprintln!(
        "game-engine-launcher: Godot {GODOT_VERSION} {}",
        executable.display()
    );
    Ok(executable)
}

fn godot_cache_dir() -> Result<PathBuf, String> {
    let cache = match env::var_os("XDG_CACHE_HOME").filter(|value| !value.is_empty()) {
        Some(path) => PathBuf::from(path),
        None => PathBuf::from(env::var_os("HOME").ok_or("HOME is unset")?).join(".cache"),
    };
    Ok(cache.join("game-engine/godot"))
}

fn verify_pinned_godot(executable: &Path, expected: &str) -> Result<(), String> {
    if !executable.is_file() {
        return Err(format!("Godot {} is missing", executable.display()));
    }
    let output = Command::new("sha512sum")
        .arg(executable)
        .output()
        .map_err(|error| format!("cannot run sha512sum: {error}"))?;
    if !output.status.success() {
        return Err(format!("sha512sum failed on {}", executable.display()));
    }
    let actual = String::from_utf8_lossy(&output.stdout);
    let actual = actual.split_whitespace().next().unwrap_or("");
    if actual != expected {
        return Err(format!(
            "Godot {} has SHA-512 {actual}, expected {expected}",
            executable.display()
        ));
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

#[cfg(test)]
mod tests {
    use super::*;

    /// SHA-512 of "abc" (FIPS 180-2 example).
    const ABC_SHA512: &str = "ddaf35a193617abacc417349ae20413112e6fa4e89a97ea20a9eeee64b55d39a2192992a274fc1a836ba3c23a3feebbd454d4423643ce80e2a9ac94fa54ca49f";

    fn file_containing_abc(name: &str) -> PathBuf {
        let path = env::temp_dir().join(format!("game-engine-launcher-{}-{name}", process::id()));
        fs::write(&path, "abc").unwrap();
        path
    }

    #[test]
    fn pinned_hash_is_accepted() {
        let path = file_containing_abc("accepted");
        let result = verify_pinned_godot(&path, ABC_SHA512);
        fs::remove_file(&path).unwrap();
        assert_eq!(result, Ok(()));
    }

    #[test]
    fn other_hash_is_refused() {
        let path = file_containing_abc("refused");
        let expected = ABC_SHA512.replace('d', "e");
        let result = verify_pinned_godot(&path, &expected);
        fs::remove_file(&path).unwrap();
        let error = result.unwrap_err();
        assert!(
            error.contains(&format!("SHA-512 {ABC_SHA512}, expected {expected}")),
            "{error}"
        );
    }
}
