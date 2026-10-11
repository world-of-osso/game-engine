use std::ffi::OsStr;
use std::fs;
use std::path::PathBuf;
use std::process::{Command, Output};
use std::sync::atomic::{AtomicUsize, Ordering};

fn invoke(args: &[impl AsRef<OsStr>]) -> Output {
    Command::new(env!("CARGO_BIN_EXE_d4_triangle_count"))
        .args(args)
        .output()
        .expect("run triangle counter")
}

fn assert_error(output: &Output) -> String {
    assert!(!output.status.success());
    assert!(output.stdout.is_empty());
    let stderr = String::from_utf8(output.stderr.clone()).expect("UTF-8 error");
    assert!(!stderr.is_empty());
    stderr
}

struct FixtureDir(PathBuf);

impl FixtureDir {
    fn new() -> Self {
        static NEXT_ID: AtomicUsize = AtomicUsize::new(0);
        let path = std::env::temp_dir().join(format!(
            "d4-triangle-cli-{}-{}",
            std::process::id(),
            NEXT_ID.fetch_add(1, Ordering::Relaxed)
        ));
        fs::create_dir(&path).expect("create fixture directory");
        Self(path)
    }
}

impl Drop for FixtureDir {
    fn drop(&mut self) {
        fs::remove_dir_all(&self.0).expect("remove fixture directory");
    }
}

#[test]
fn help_prints_usage_and_succeeds() {
    let output = invoke(&["--help"]);
    assert!(output.status.success());
    assert!(output.stderr.is_empty());
    let stdout = String::from_utf8(output.stdout).expect("UTF-8 help");
    assert!(stdout.contains("Usage: d4_triangle_count META PAYLOAD"));
}

#[test]
fn rejects_missing_and_extra_arguments() {
    for args in [vec![], vec!["meta.app"], vec!["a", "b", "c"]] {
        let stderr = assert_error(&invoke(&args));
        assert!(stderr.contains("META PAYLOAD"));
    }
}

#[test]
fn rejects_options_instead_of_paths() {
    for args in [
        vec!["--unknown"],
        vec!["--unknown", "payload.app"],
        vec!["meta.app", "--help"],
        vec!["--help", "payload.app"],
    ] {
        assert_error(&invoke(&args));
    }
}

#[test]
fn missing_meta_error_identifies_input_path() {
    let dir = FixtureDir::new();
    let meta = dir.0.join("missing-meta.app");
    let payload = dir.0.join("payload.app");
    fs::write(&payload, []).expect("write payload");
    let stderr = assert_error(&invoke(&[&meta, &payload]));
    assert!(stderr.contains(meta.to_str().expect("UTF-8 fixture path")));
}

#[test]
fn missing_payload_error_identifies_input_path() {
    let dir = FixtureDir::new();
    let meta = dir.0.join("meta.app");
    let payload = dir.0.join("missing-payload.app");
    fs::write(&meta, []).expect("write meta");
    let stderr = assert_error(&invoke(&[&meta, &payload]));
    assert!(stderr.contains(payload.to_str().expect("UTF-8 fixture path")));
}
