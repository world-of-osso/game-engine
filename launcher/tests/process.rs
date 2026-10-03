use std::{
    env, fs,
    os::unix::{
        ffi::{OsStrExt, OsStringExt},
        fs::PermissionsExt,
    },
    path::{Path, PathBuf},
    process::{Command, Output},
    sync::atomic::{AtomicU64, Ordering},
};

static NEXT_ID: AtomicU64 = AtomicU64::new(0);

struct Fixture {
    directory: PathBuf,
    cargo: PathBuf,
    godot: PathBuf,
}

impl Fixture {
    fn new() -> Self {
        let directory = env::temp_dir().join(format!(
            "game-engine-launcher-test-{}-{}",
            std::process::id(),
            NEXT_ID.fetch_add(1, Ordering::Relaxed)
        ));
        fs::create_dir(&directory).unwrap();
        // An already-imported project, so launches skip the one-time import.
        fs::create_dir_all(directory.join("godot/.godot")).unwrap();
        fs::write(directory.join("godot/.godot/extension_list.cfg"), "").unwrap();
        fs::create_dir(directory.join("bin")).unwrap();
        let cargo = directory.join("bin/cargo");
        let godot = directory.join("bin/godot");
        for binary in [&cargo, &godot] {
            fs::write(binary, FAKE_PROCESS).unwrap();
            fs::set_permissions(binary, fs::Permissions::from_mode(0o755)).unwrap();
        }
        fs::create_dir(directory.join("scripts")).unwrap();
        fs::write(directory.join("scripts/depot-build.py"), FAKE_PROCESS).unwrap();
        Self {
            directory,
            cargo,
            godot,
        }
    }

    fn launch(&self, args: &[&str]) -> Output {
        Command::new(env!("CARGO_BIN_EXE_game-engine-launcher"))
            .args(args)
            .current_dir(&self.directory)
            .env("CARGO", &self.cargo)
            .env("GODOT_BIN", &self.godot)
            .env("FAKE_LOG", self.directory.join("log"))
            .env("GAME_ENGINE_ROOT", &self.directory)
            .env("LD_LIBRARY_PATH", "/existing/libraries")
            .output()
            .unwrap()
    }

    fn launch_os(&self, args: &[&std::ffi::OsStr]) -> Output {
        Command::new(env!("CARGO_BIN_EXE_game-engine-launcher"))
            .args(args)
            .current_dir(&self.directory)
            .env("CARGO", &self.cargo)
            .env("GODOT_BIN", &self.godot)
            .env("FAKE_LOG", self.directory.join("log"))
            .env("GAME_ENGINE_ROOT", &self.directory)
            .output()
            .unwrap()
    }

    fn godot_args(&self) -> Vec<String> {
        self.log()
            .lines()
            .nth(1)
            .unwrap()
            .split('\t')
            .nth(4)
            .unwrap()
            .split(',')
            .map(str::to_owned)
            .collect()
    }

    fn log(&self) -> String {
        fs::read_to_string(self.directory.join("log")).unwrap_or_default()
    }
}

impl Drop for Fixture {
    fn drop(&mut self) {
        fs::remove_dir_all(&self.directory).unwrap();
    }
}

const FAKE_PROCESS: &str = r#"#!/usr/bin/env python3
import os
import sys

role = os.path.basename(sys.argv[0])
def hex_bytes(value):
    return os.fsencode(value).hex()
with open(os.environ['FAKE_LOG'], 'a', encoding='ascii') as log:
    print(role, hex_bytes(os.getcwd()),
          hex_bytes(os.environ.get('CARGO_TARGET_DIR', '')),
          hex_bytes(os.environ.get('LD_LIBRARY_PATH', '')),
          ','.join(hex_bytes(arg) for arg in sys.argv[1:]),
          sep='\t', file=log)
suffix = '_IMPORT_STATUS' if '--import' in sys.argv else '_STATUS'
sys.exit(int(os.environ.get('FAKE_' + role.upper() + suffix, '0')))
"#;

fn hex(value: impl AsRef<std::ffi::OsStr>) -> String {
    value
        .as_ref()
        .as_bytes()
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect()
}

#[test]
fn successful_build_then_launch_forwards_arguments_and_environment() {
    let fixture = Fixture::new();
    let output = fixture.launch(&["--editor", "a b", "--", "profile.gd"]);
    assert!(output.status.success(), "{output:?}");
    let records: Vec<_> = fixture.log().lines().map(str::to_owned).collect();
    assert_eq!(records.len(), 2, "{records:?}");
    let cargo = records[0].split('\t').collect::<Vec<_>>();
    let godot = records[1].split('\t').collect::<Vec<_>>();
    let root = &fixture.directory;
    assert_eq!(cargo[0], "depot-build.py");
    assert_eq!(cargo[1], hex(root));
    assert_eq!(
        cargo[4],
        ["--root", root.to_str().unwrap()].map(hex).join(",")
    );
    assert_eq!(godot[0], "godot");
    assert_eq!(godot[1], hex(&fixture.directory));
    assert_eq!(
        godot[3],
        hex(format!(
            "{}:/existing/libraries",
            root.join("target/debug/deps").display()
        ))
    );
    assert_eq!(
        godot[4],
        [
            "--path",
            root.join("godot").to_str().unwrap(),
            "--editor",
            "a b",
            "--",
            "profile.gd"
        ]
        .map(hex)
        .join(",")
    );
}

#[test]
fn build_host_is_consumed_and_forwarded_only_to_helper() {
    for host in ["desktop", "local"] {
        let fixture = Fixture::new();
        let output = fixture.launch(&[
            "--headless",
            "--screen",
            "login",
            "--build-host",
            host,
            "--verbose",
        ]);
        assert!(output.status.success(), "{output:?}");
        let log = fixture.log();
        let records: Vec<_> = log.lines().collect();
        assert_eq!(records.len(), 2, "{log}");
        assert_eq!(
            records[0].split('\t').nth(4).unwrap(),
            [
                hex("--root"),
                hex(&fixture.directory),
                hex("--build-host"),
                hex(host),
            ]
            .join(",")
        );
        assert_eq!(
            fixture.godot_args(),
            [
                hex("--path"),
                hex(fixture.directory.join("godot")),
                hex("--headless"),
                hex("--verbose"),
                hex("--"),
                hex("--screen"),
                hex("login"),
            ]
        );
    }
}

#[test]
fn build_host_omission_leaves_default_selection_to_helper() {
    let fixture = Fixture::new();
    let output = fixture.launch(&[]);
    assert!(output.status.success(), "{output:?}");
    let log = fixture.log();
    assert_eq!(
        log.lines().next().unwrap().split('\t').nth(4).unwrap(),
        [hex("--root"), hex(&fixture.directory)].join(",")
    );
}

#[test]
fn build_host_after_separator_remains_a_client_argument() {
    let fixture = Fixture::new();
    let output = fixture.launch(&["--", "--build-host", "client-owned"]);
    assert!(output.status.success(), "{output:?}");
    let log = fixture.log();
    assert_eq!(
        log.lines().next().unwrap().split('\t').nth(4).unwrap(),
        [hex("--root"), hex(&fixture.directory)].join(",")
    );
    assert_eq!(
        fixture.godot_args(),
        [
            hex("--path"),
            hex(fixture.directory.join("godot")),
            hex("--"),
            hex("--build-host"),
            hex("client-owned"),
        ]
    );
}

#[test]
fn build_host_missing_value_fails_before_any_process() {
    for args in [
        vec!["--build-host"],
        vec!["--build-host", "--verbose"],
        vec!["--build-host", "--"],
    ] {
        let fixture = Fixture::new();
        let output = fixture.launch(&args);
        assert_eq!(output.status.code(), Some(1), "{output:?}");
        let stderr = String::from_utf8_lossy(&output.stderr);
        assert!(stderr.contains("--build-host requires a value"), "{stderr}");
        assert!(fixture.log().is_empty());
    }
}

#[test]
fn build_host_invalid_value_fails_before_any_process() {
    for host in ["depot", "Desktop", "", "remote"] {
        let fixture = Fixture::new();
        let output = fixture.launch(&["--build-host", host]);
        assert_eq!(output.status.code(), Some(1), "{output:?}");
        let stderr = String::from_utf8_lossy(&output.stderr);
        assert!(stderr.contains("invalid --build-host"), "{stderr}");
        assert!(stderr.contains("desktop or local"), "{stderr}");
        assert!(fixture.log().is_empty());
    }
}

#[test]
fn js_script_flag_reaches_godot_as_a_client_argument() {
    let fixture = Fixture::new();
    let output = fixture.launch(&[
        "--headless",
        "--screen",
        "login",
        "--run-js-ui-script",
        "debug/login.js",
    ]);
    assert!(output.status.success(), "{output:?}");
    assert_eq!(
        fixture.godot_args(),
        [
            "--path",
            fixture.directory.join("godot").to_str().unwrap(),
            "--headless",
            "--",
            "--screen",
            "login",
            "--run-js-ui-script",
            "debug/login.js",
        ]
        .map(hex)
    );
}

#[test]
fn interspersed_client_pairs_follow_native_engine_arguments() {
    let fixture = Fixture::new();
    let output = fixture.launch(&[
        "--headless",
        "--screen",
        "charselect",
        "--rendering-method",
        "gl_compatibility",
        "--server",
        "127.0.0.1:5000",
        "--state",
        "login",
        "--verbose",
        "--char",
        "Alice",
    ]);
    assert!(output.status.success(), "{output:?}");
    let godot_project = fixture.directory.join("godot");
    let expected = [
        "--path",
        godot_project.to_str().unwrap(),
        "--headless",
        "--rendering-method",
        "gl_compatibility",
        "--verbose",
        "--",
        "--screen",
        "charselect",
        "--server",
        "127.0.0.1:5000",
        "--state",
        "login",
        "--char",
        "Alice",
    ];
    assert_eq!(fixture.godot_args(), expected.map(hex));
}

#[test]
fn skybox_original_value_options_follow_engine_arguments_in_client_order() {
    for (asset_flag, asset_id) in [("--skybox-fdid", "120191"), ("--light-skybox-id", "42")] {
        let fixture = Fixture::new();
        let output = fixture.launch(&[
            "--headless",
            asset_flag,
            asset_id,
            "--rendering-method",
            "gl_compatibility",
            "--screen",
            "skyboxdebug",
            "--skybox-time-ms",
            "43200000",
            "--verbose",
            "--skybox-verify",
        ]);
        assert!(output.status.success(), "{output:?}");
        assert_eq!(
            fixture.godot_args(),
            [
                "--path",
                fixture.directory.join("godot").to_str().unwrap(),
                "--headless",
                "--rendering-method",
                "gl_compatibility",
                "--verbose",
                "--",
                asset_flag,
                asset_id,
                "--screen",
                "skyboxdebug",
                "--skybox-time-ms",
                "43200000",
                "--skybox-verify",
            ]
            .map(hex),
            "{asset_flag} must reach native client, preserving engine argument order"
        );
    }
}

#[test]
fn skybox_verify_is_valueless_and_does_not_consume_next_engine_argument() {
    let fixture = Fixture::new();
    let output = fixture.launch(&[
        "--skybox-verify",
        "--verbose",
        "--screen",
        "skyboxdebug",
        "--headless",
    ]);
    assert!(output.status.success(), "{output:?}");
    assert_eq!(
        fixture.godot_args(),
        [
            "--path",
            fixture.directory.join("godot").to_str().unwrap(),
            "--verbose",
            "--headless",
            "--",
            "--skybox-verify",
            "--screen",
            "skyboxdebug",
        ]
        .map(hex)
    );
}

#[test]
fn skybox_explicit_separator_preserves_original_options_without_duplication() {
    let fixture = Fixture::new();
    let output = fixture.launch(&[
        "--screen",
        "skyboxdebug",
        "--skybox-time-ms",
        "0",
        "--verbose",
        "--",
        "--light-skybox-id",
        "4294967295",
        "--skybox-verify",
    ]);
    assert!(output.status.success(), "{output:?}");
    assert_eq!(
        fixture.godot_args(),
        [
            "--path",
            fixture.directory.join("godot").to_str().unwrap(),
            "--verbose",
            "--",
            "--screen",
            "skyboxdebug",
            "--skybox-time-ms",
            "0",
            "--light-skybox-id",
            "4294967295",
            "--skybox-verify",
        ]
        .map(hex)
    );
}

#[test]
fn particle_debug_screen_reaches_godot_client_arguments() {
    let fixture = Fixture::new();
    let output = fixture.launch(&["--screen", "particledebug"]);
    assert!(output.status.success(), "{output:?}");
    let godot_project = fixture.directory.join("godot");
    let expected = [
        "--path",
        godot_project.to_str().unwrap(),
        "--",
        "--screen",
        "particledebug",
    ];
    assert_eq!(fixture.godot_args(), expected.map(hex));
}

#[test]
fn explicit_separator_keeps_remainder_in_client_order_without_duplication() {
    let fixture = Fixture::new();
    let output = fixture.launch(&[
        "--screen",
        "charselect",
        "--verbose",
        "--",
        "--state",
        "login",
        "--custom",
        "value",
    ]);
    assert!(output.status.success(), "{output:?}");
    let godot_project = fixture.directory.join("godot");
    let expected = [
        "--path",
        godot_project.to_str().unwrap(),
        "--verbose",
        "--",
        "--screen",
        "charselect",
        "--state",
        "login",
        "--custom",
        "value",
    ];
    assert_eq!(fixture.godot_args(), expected.map(hex));
}

#[test]
fn missing_client_values_fail_before_native_build() {
    for (flag, trailing) in [
        ("--screen", None),
        ("--state", Some("--verbose")),
        ("--server", Some("--char")),
        ("--char", Some("--")),
    ] {
        let fixture = Fixture::new();
        let mut args = vec![flag];
        if let Some(trailing) = trailing {
            args.push(trailing);
        }
        let output = fixture.launch(&args);
        assert_eq!(output.status.code(), Some(1));
        let stderr = String::from_utf8_lossy(&output.stderr);
        assert!(
            stderr.contains(&format!("{flag} requires a value")),
            "{stderr}"
        );
        assert!(fixture.log().is_empty(), "{flag} built native code");
    }
}

#[test]
fn non_utf8_native_and_client_values_survive_routing() {
    let fixture = Fixture::new();
    let native = std::ffi::OsString::from_vec(vec![b'-', 0xff, b'x']);
    let client = std::ffi::OsString::from_vec(vec![b'A', 0xff, b'b']);
    let output = fixture.launch_os(&[
        std::ffi::OsStr::new("--screen"),
        &client,
        &native,
        std::ffi::OsStr::new("--verbose"),
    ]);
    assert!(output.status.success(), "{output:?}");
    assert_eq!(
        fixture.godot_args(),
        [
            hex("--path"),
            hex(fixture.directory.join("godot")),
            hex(&native),
            hex("--verbose"),
            hex("--"),
            hex("--screen"),
            hex(&client),
        ]
    );
}

#[test]
fn build_failure_preserves_status_and_prevents_godot() {
    let fixture = Fixture::new();
    fs::remove_dir_all(fixture.directory.join("godot/.godot")).unwrap();
    let output = Command::new(env!("CARGO_BIN_EXE_game-engine-launcher"))
        .args(["--build-host", "local"])
        .env("CARGO", &fixture.cargo)
        .env("GODOT_BIN", &fixture.godot)
        .env("FAKE_LOG", fixture.directory.join("log"))
        .env("GAME_ENGINE_ROOT", &fixture.directory)
        .env("FAKE_DEPOT-BUILD.PY_STATUS", "37")
        .output()
        .unwrap();
    assert_eq!(output.status.code(), Some(37));
    let log = fixture.log();
    assert_eq!(log.lines().count(), 1);
    assert!(log.starts_with("depot-build.py\t"));
    assert_eq!(
        log.lines().next().unwrap().split('\t').nth(4).unwrap(),
        [
            hex("--root"),
            hex(&fixture.directory),
            hex("--build-host"),
            hex("local"),
        ]
        .join(",")
    );
}

#[test]
fn godot_exit_status_is_preserved() {
    let fixture = Fixture::new();
    let output = Command::new(env!("CARGO_BIN_EXE_game-engine-launcher"))
        .env("CARGO", &fixture.cargo)
        .env("GODOT_BIN", &fixture.godot)
        .env("FAKE_LOG", fixture.directory.join("log"))
        .env("GAME_ENGINE_ROOT", &fixture.directory)
        .env("FAKE_GODOT_STATUS", "43")
        .output()
        .unwrap();
    assert_eq!(output.status.code(), Some(43));
    assert_eq!(fixture.log().lines().count(), 2);
}

#[test]
fn missing_godot_fails_before_build() {
    let fixture = Fixture::new();
    let missing = fixture.directory.join("missing-godot");
    let output = Command::new(env!("CARGO_BIN_EXE_game-engine-launcher"))
        .env("CARGO", &fixture.cargo)
        .env("GODOT_BIN", &missing)
        .env("FAKE_LOG", fixture.directory.join("log"))
        .env("GAME_ENGINE_ROOT", &fixture.directory)
        .output()
        .unwrap();
    assert!(!output.status.success());
    assert!(String::from_utf8_lossy(&output.stderr).contains("GODOT_BIN"));
    assert!(String::from_utf8_lossy(&output.stderr).contains("missing-godot"));
    assert!(fixture.log().is_empty());
}

#[test]
fn non_utf8_argument_reaches_godot_unchanged() {
    let fixture = Fixture::new();
    let argument = std::ffi::OsString::from_vec(vec![b'-', 0xff, b'x']);
    let output = Command::new(env!("CARGO_BIN_EXE_game-engine-launcher"))
        .arg(&argument)
        .env("CARGO", &fixture.cargo)
        .env("GODOT_BIN", &fixture.godot)
        .env("FAKE_LOG", fixture.directory.join("log"))
        .env("GAME_ENGINE_ROOT", &fixture.directory)
        .output()
        .unwrap();
    assert!(output.status.success(), "{output:?}");
    assert!(
        fixture
            .log()
            .lines()
            .nth(1)
            .unwrap()
            .ends_with(&hex(argument))
    );
}

#[test]
fn non_executable_godot_fails_before_build() {
    let fixture = Fixture::new();
    fs::set_permissions(&fixture.godot, fs::Permissions::from_mode(0o644)).unwrap();
    let output = fixture.launch(&[]);
    assert!(!output.status.success());
    assert!(String::from_utf8_lossy(&output.stderr).contains("not an executable file"));
    assert!(fixture.log().is_empty());
}

#[test]
fn launches_without_local_cargo_context() {
    let fixture = Fixture::new();
    let output = Command::new(env!("CARGO_BIN_EXE_game-engine-launcher"))
        .env_remove("CARGO")
        .env("GODOT_BIN", &fixture.godot)
        .env("FAKE_LOG", fixture.directory.join("log"))
        .env("GAME_ENGINE_ROOT", &fixture.directory)
        .output()
        .unwrap();
    assert!(output.status.success(), "{output:?}");
    assert_eq!(fixture.log().lines().count(), 2);
    assert!(fixture.log().starts_with("depot-build.py\t"));
}

impl Fixture {
    /// Launch without GODOT_BIN, resolving the pinned Godot from `cache`.
    fn launch_with_cache(&self, cache: &Path) -> Output {
        Command::new(env!("CARGO_BIN_EXE_game-engine-launcher"))
            .env("CARGO", &self.cargo)
            .env_remove("GODOT_BIN")
            .env("XDG_CACHE_HOME", cache)
            .env("FAKE_LOG", self.directory.join("log"))
            .env("GAME_ENGINE_ROOT", &self.directory)
            .output()
            .unwrap()
    }
}

#[test]
fn missing_pinned_godot_names_build_script_and_prevents_build() {
    let fixture = Fixture::new();
    let output = fixture.launch_with_cache(&fixture.directory.join("cache"));
    assert!(!output.status.success());
    let stderr = String::from_utf8_lossy(&output.stderr);
    let pinned = "cache/game-engine/godot/4.7.2-pr123946-pr123546/godot-4.7.2-pr123946-pr123546 is missing";
    assert!(stderr.contains(pinned), "{stderr}");
    assert!(
        stderr.contains("scripts/godot/build-patched-godot.sh"),
        "{stderr}"
    );
    assert!(fixture.log().is_empty(), "nothing may run");
}

#[test]
fn unpinned_godot_is_refused_before_build() {
    let fixture = Fixture::new();
    let cache = fixture.directory.join("cache");
    let version = cache.join("game-engine/godot/4.7.2-pr123946-pr123546");
    fs::create_dir_all(&version).unwrap();
    // Any binary other than the pinned build is refused, even in the pinned slot.
    fs::copy(&fixture.godot, version.join("godot-4.7.2-pr123946-pr123546")).unwrap();
    let output = fixture.launch_with_cache(&cache);
    assert!(!output.status.success());
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("SHA-512"), "{stderr}");
    assert!(
        stderr.contains("scripts/godot/build-patched-godot.sh"),
        "{stderr}"
    );
    assert!(fixture.log().is_empty(), "nothing may run");
}

#[test]
fn fresh_checkout_imports_once_after_build_before_launch() {
    let fixture = Fixture::new();
    fs::remove_dir_all(fixture.directory.join("godot/.godot")).unwrap();
    let output = fixture.launch(&["--verbose"]);
    assert!(output.status.success(), "{output:?}");
    let records: Vec<Vec<String>> = fixture
        .log()
        .lines()
        .map(|line| line.split('\t').map(str::to_owned).collect())
        .collect();
    let roles: Vec<_> = records.iter().map(|record| record[0].as_str()).collect();
    assert_eq!(roles, ["depot-build.py", "godot", "godot"]);
    let project = fixture.directory.join("godot");
    assert_eq!(
        records[1][4],
        [
            hex("--headless"),
            hex("--import"),
            hex("--path"),
            hex(&project)
        ]
        .join(",")
    );
    assert_eq!(
        records[2][4],
        [hex("--path"), hex(&project), hex("--verbose")].join(",")
    );
}

#[test]
fn failed_import_preserves_status_and_prevents_launch() {
    let fixture = Fixture::new();
    fs::remove_dir_all(fixture.directory.join("godot/.godot")).unwrap();
    let output = Command::new(env!("CARGO_BIN_EXE_game-engine-launcher"))
        .env("CARGO", &fixture.cargo)
        .env("GODOT_BIN", &fixture.godot)
        .env("FAKE_LOG", fixture.directory.join("log"))
        .env("GAME_ENGINE_ROOT", &fixture.directory)
        .env("FAKE_GODOT_IMPORT_STATUS", "41")
        .output()
        .unwrap();
    assert_eq!(output.status.code(), Some(41));
    assert_eq!(
        fixture.log().lines().count(),
        2,
        "Godot must not launch after a failed import"
    );
}
