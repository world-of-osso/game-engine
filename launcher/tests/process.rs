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
        let cargo = directory.join("cargo");
        let godot = directory.join("godot");
        for binary in [&cargo, &godot] {
            fs::write(binary, FAKE_PROCESS).unwrap();
            fs::set_permissions(binary, fs::Permissions::from_mode(0o755)).unwrap();
        }
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
sys.exit(int(os.environ.get('FAKE_' + role.upper() + '_STATUS', '0')))
"#;

fn hex(value: impl AsRef<std::ffi::OsStr>) -> String {
    value
        .as_ref()
        .as_bytes()
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect()
}

fn root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .to_path_buf()
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
    let root = root();
    assert_eq!(cargo[0], "cargo");
    assert_eq!(cargo[1], hex(&root));
    assert_eq!(cargo[2], hex(root.join("target")));
    assert_eq!(
        cargo[4],
        [
            "build",
            "--manifest-path",
            root.join("godot/Cargo.toml").to_str().unwrap(),
            "-p",
            "game-engine-godot",
            "--lib",
        ]
        .map(hex)
        .join(",")
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
    let godot_project = root().join("godot");
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
    let godot_project = root().join("godot");
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
            hex(root().join("godot")),
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
    let output = Command::new(env!("CARGO_BIN_EXE_game-engine-launcher"))
        .env("CARGO", &fixture.cargo)
        .env("GODOT_BIN", &fixture.godot)
        .env("FAKE_LOG", fixture.directory.join("log"))
        .env("FAKE_CARGO_STATUS", "37")
        .output()
        .unwrap();
    assert_eq!(output.status.code(), Some(37));
    assert_eq!(fixture.log().lines().count(), 1);
    assert!(fixture.log().starts_with("cargo\t"));
}

#[test]
fn godot_exit_status_is_preserved() {
    let fixture = Fixture::new();
    let output = Command::new(env!("CARGO_BIN_EXE_game-engine-launcher"))
        .env("CARGO", &fixture.cargo)
        .env("GODOT_BIN", &fixture.godot)
        .env("FAKE_LOG", fixture.directory.join("log"))
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
fn missing_cargo_context_fails_before_godot() {
    let fixture = Fixture::new();
    let output = Command::new(env!("CARGO_BIN_EXE_game-engine-launcher"))
        .env_remove("CARGO")
        .env("GODOT_BIN", &fixture.godot)
        .env("FAKE_LOG", fixture.directory.join("log"))
        .output()
        .unwrap();
    assert!(!output.status.success());
    assert!(String::from_utf8_lossy(&output.stderr).contains("CARGO"));
    assert!(fixture.log().is_empty());
}
