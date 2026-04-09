use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Child, Command};
use std::thread;
use std::time::{Duration, Instant};
use webp::Decoder;

const MODEL_PATH: &str = "data/models/189796.m2";
const GOLDEN_PATH: &str = "tests/golden/model-189796.webp";
const TIMEOUT: Duration = Duration::from_secs(90);
const MAX_MEAN_DIFF: f64 = 3.0;

#[test]
#[ignore = "requires windowing/GPU access and is run via scripts/run_screenshot_regression.sh"]
fn renders_known_model_matches_golden() {
    let workdir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let temp_root = temp_dir("screenshot-regression");
    let output_path = temp_root.join("actual.webp");
    let status = wait_for_child(
        spawn_capture_command(&workdir, &temp_root, &output_path)
            .expect("failed to spawn screenshot regression command"),
    )
    .expect("screenshot regression command should finish successfully");
    assert!(status.success(), "screenshot capture failed with {status}");

    let expected = decode_webp(&workdir.join(GOLDEN_PATH));
    let actual = decode_webp(&output_path);
    assert_eq!(
        actual.width,
        expected.width,
        "screenshot width mismatch against {GOLDEN_PATH}; actual output at {}",
        output_path.display()
    );
    assert_eq!(
        actual.height,
        expected.height,
        "screenshot height mismatch against {GOLDEN_PATH}; actual output at {}",
        output_path.display()
    );
    assert_eq!(
        actual.channels,
        expected.channels,
        "screenshot channel layout mismatch against {GOLDEN_PATH}; actual output at {}",
        output_path.display()
    );
    let mean_diff = mean_abs_diff(&actual.pixels, &expected.pixels);
    assert!(
        mean_diff <= MAX_MEAN_DIFF,
        "screenshot mismatch against {GOLDEN_PATH}; mean absolute pixel diff {mean_diff:.3} exceeds {MAX_MEAN_DIFF:.3}; actual output at {}",
        output_path.display()
    );
}

fn spawn_capture_command(
    workdir: &Path,
    temp_root: &Path,
    output_path: &Path,
) -> Result<Child, String> {
    let bin = PathBuf::from(env!("CARGO_BIN_EXE_game-engine"));
    let mut command = if should_wrap_in_xvfb() {
        let mut cmd = Command::new("xvfb-run");
        cmd.arg("-a").arg(bin);
        cmd
    } else {
        Command::new(bin)
    };
    command
        .current_dir(workdir)
        .env("XDG_CONFIG_HOME", temp_root)
        .arg("--screenshot-regression")
        .arg("screenshot")
        .arg(output_path)
        .arg(MODEL_PATH);
    command
        .spawn()
        .map_err(|err| format!("failed to spawn screenshot regression binary: {err}"))
}

fn should_wrap_in_xvfb() -> bool {
    std::env::var_os("DISPLAY").is_none()
        && std::env::var_os("WAYLAND_DISPLAY").is_none()
        && Command::new("sh")
            .arg("-c")
            .arg("command -v xvfb-run >/dev/null 2>&1")
            .status()
            .map(|status| status.success())
            .unwrap_or(false)
}

fn wait_for_child(mut child: Child) -> Result<std::process::ExitStatus, String> {
    let deadline = Instant::now() + TIMEOUT;
    loop {
        if let Some(status) = child
            .try_wait()
            .map_err(|err| format!("failed to wait for screenshot child: {err}"))?
        {
            return Ok(status);
        }
        if Instant::now() >= deadline {
            let _ = child.kill();
            let _ = child.wait();
            return Err(format!(
                "screenshot regression command timed out after {}s",
                TIMEOUT.as_secs()
            ));
        }
        thread::sleep(Duration::from_millis(250));
    }
}

fn temp_dir(label: &str) -> PathBuf {
    let path = std::env::temp_dir().join(format!(
        "game-engine-{label}-{}-{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ));
    fs::create_dir_all(&path).expect("temp dir should be creatable");
    path
}

struct DecodedImage {
    width: u32,
    height: u32,
    channels: usize,
    pixels: Vec<u8>,
}

fn decode_webp(path: &Path) -> DecodedImage {
    let bytes =
        fs::read(path).unwrap_or_else(|err| panic!("failed to read {}: {err}", path.display()));
    let image = Decoder::new(&bytes)
        .decode()
        .unwrap_or_else(|| panic!("failed to decode {}", path.display()));
    DecodedImage {
        width: image.width(),
        height: image.height(),
        channels: image.layout().bytes_per_pixel() as usize,
        pixels: image.to_vec(),
    }
}

fn mean_abs_diff(actual: &[u8], expected: &[u8]) -> f64 {
    assert_eq!(
        actual.len(),
        expected.len(),
        "decoded screenshots should have the same byte length"
    );
    let total: u64 = actual
        .iter()
        .zip(expected)
        .map(|(a, b)| a.abs_diff(*b) as u64)
        .sum();
    total as f64 / actual.len() as f64
}
