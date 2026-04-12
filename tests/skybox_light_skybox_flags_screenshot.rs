use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Child, Command};
use std::thread;
use std::time::{Duration, Instant};

use webp::Decoder;

const TIMEOUT: Duration = Duration::from_secs(90);
const BLEND_ON_LIGHT_SKYBOX_ID: u32 = 653;
const MIN_BLEND_ON_LOWER_HALF_LUMINANCE_DELTA: f64 = 15.0;

#[test]
#[ignore = "requires windowing/GPU access and is run manually for skybox regression coverage"]
fn skybox_debug_blend_flags_change_default_vs_verify_output() {
    let workdir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let temp_root = temp_dir("skybox-light-skybox-flags");

    let blend_on_default =
        capture_skybox_debug(&workdir, &temp_root, BLEND_ON_LIGHT_SKYBOX_ID, false);
    let blend_on_verify =
        capture_skybox_debug(&workdir, &temp_root, BLEND_ON_LIGHT_SKYBOX_ID, true);

    let blend_on_default = decode_webp(&blend_on_default);
    let blend_on_verify = decode_webp(&blend_on_verify);

    assert_matching_layout(
        &blend_on_default,
        &blend_on_verify,
        BLEND_ON_LIGHT_SKYBOX_ID,
    );

    let blend_on_default_lower_half_luminance = lower_half_mean_luminance(&blend_on_default);
    let blend_on_verify_lower_half_luminance = lower_half_mean_luminance(&blend_on_verify);
    let lower_half_luminance_delta =
        blend_on_default_lower_half_luminance - blend_on_verify_lower_half_luminance;

    assert!(
        lower_half_luminance_delta >= MIN_BLEND_ON_LOWER_HALF_LUMINANCE_DELTA,
        "default vs verify should diverge for blend-on LightSkyboxID {BLEND_ON_LIGHT_SKYBOX_ID}; lower-half luminance delta {lower_half_luminance_delta:.3} < {MIN_BLEND_ON_LOWER_HALF_LUMINANCE_DELTA:.3} (default={blend_on_default_lower_half_luminance:.3}, verify={blend_on_verify_lower_half_luminance:.3})"
    );
}

fn capture_skybox_debug(
    workdir: &Path,
    temp_root: &Path,
    light_skybox_id: u32,
    verify_mode: bool,
) -> PathBuf {
    let mode = if verify_mode { "verify" } else { "default" };
    let output_path = temp_root.join(format!("skybox-{light_skybox_id}-{mode}.webp"));
    let status = wait_for_child(
        spawn_capture_command(
            workdir,
            temp_root,
            light_skybox_id,
            verify_mode,
            &output_path,
        )
        .expect("spawn skybox screenshot capture"),
    )
    .expect("skybox screenshot capture should finish successfully");
    assert!(
        status.success(),
        "skybox screenshot capture failed for LightSkyboxID {light_skybox_id} ({mode}) with {status}"
    );
    output_path
}

fn spawn_capture_command(
    workdir: &Path,
    temp_root: &Path,
    light_skybox_id: u32,
    verify_mode: bool,
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
        .arg("--screen")
        .arg("skyboxdebug")
        .arg("--light-skybox-id")
        .arg(light_skybox_id.to_string());
    if verify_mode {
        command.arg("--skybox-verify");
    }
    command.arg("screenshot").arg(output_path);
    command
        .spawn()
        .map_err(|err| format!("failed to spawn skybox screenshot command: {err}"))
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
                "screenshot command timed out after {}s",
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

fn assert_matching_layout(left: &DecodedImage, right: &DecodedImage, light_skybox_id: u32) {
    assert_eq!(
        left.width, right.width,
        "width mismatch for {light_skybox_id}"
    );
    assert_eq!(
        left.height, right.height,
        "height mismatch for {light_skybox_id}"
    );
    assert_eq!(
        left.channels, right.channels,
        "channel mismatch for {light_skybox_id}"
    );
}

fn lower_half_mean_luminance(image: &DecodedImage) -> f64 {
    let start_row = image.height as usize / 2;
    let row_bytes = image.width as usize * image.channels;
    let mut total = 0.0;
    let mut count = 0usize;
    for row in start_row..image.height as usize {
        let row_start = row * row_bytes;
        let row_end = row_start + row_bytes;
        let pixels = &image.pixels[row_start..row_end];
        for pixel in pixels.chunks_exact(image.channels) {
            let red = pixel[0] as f64;
            let green = pixel[1] as f64;
            let blue = pixel[2] as f64;
            total += 0.2126 * red + 0.7152 * green + 0.0722 * blue;
            count += 1;
        }
    }
    total / count as f64
}
