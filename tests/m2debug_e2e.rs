use std::collections::HashSet;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Child, Command, Stdio};
use std::thread;
use std::time::{Duration, Instant};

use game_engine::ipc::{Request, Response};
use peercred_ipc::Client;
use webp::Decoder;

const SOCKET_WAIT_TIMEOUT: Duration = Duration::from_secs(30);
const SCENE_READY_TIMEOUT: Duration = Duration::from_secs(30);
const SCREENSHOT_SETTLE_DELAY: Duration = Duration::from_secs(2);
const OVERLAY_EXCLUSION_WIDTH: u32 = 240;
const OVERLAY_EXCLUSION_HEIGHT: u32 = 120;
const MIN_VISIBLE_PIXEL_RATIO: f64 = 0.01;
const VISIBLE_PIXEL_THRESHOLD: u8 = 32;

#[test]
#[ignore = "requires windowing/GPU access; run with: cargo test --test m2debug_e2e -- --ignored"]
fn m2debug_ipc_screenshot_is_not_effectively_black() {
    let engine_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let temp_root = temp_dir("m2debug-e2e");
    let output_path = temp_root.join("m2debug-ipc.webp");
    let existing_sockets = list_engine_sockets();
    let mut child =
        spawn_m2debug_client(&engine_dir, &temp_root).expect("failed to spawn m2debug client");

    let socket = wait_for_new_socket(&existing_sockets, &mut child)
        .expect("m2debug IPC socket should appear");
    let scene = wait_for_m2debug_scene(&socket, &mut child)
        .expect("m2debug scene should become queryable over IPC");
    thread::sleep(SCREENSHOT_SETTLE_DELAY);

    let screenshot = request_screenshot(&socket).expect("IPC screenshot should succeed");
    fs::write(&output_path, &screenshot)
        .unwrap_or_else(|err| panic!("failed to write {}: {err}", output_path.display()));
    stop_child(&mut child);
    let stderr_tail = child_stderr_tail(&mut child);

    let image = decode_webp_bytes(&screenshot);
    let visible_ratio = visible_pixel_ratio_outside_overlay(&image);
    assert!(
        visible_ratio >= MIN_VISIBLE_PIXEL_RATIO,
        "expected m2debug live IPC screenshot to contain visible 3D content; \
visible pixel ratio {visible_ratio:.6} < {MIN_VISIBLE_PIXEL_RATIO:.6}; screenshot at {}\nscene:\n{scene}\nstderr:\n{stderr_tail}",
        output_path.display()
    );
}

fn spawn_m2debug_client(workdir: &Path, temp_root: &Path) -> Result<Child, String> {
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
        .env("SKIP_EULA", "1")
        .args(["--screen", "m2debug"])
        .stdout(Stdio::null())
        .stderr(Stdio::piped());
    command
        .spawn()
        .map_err(|err| format!("failed to spawn m2debug client: {err}"))
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

fn list_engine_sockets() -> HashSet<PathBuf> {
    let Ok(entries) = fs::read_dir("/tmp") else {
        return HashSet::new();
    };
    entries
        .filter_map(|entry| entry.ok())
        .map(|entry| entry.path())
        .filter(|path| {
            path.file_name()
                .and_then(|name| name.to_str())
                .is_some_and(|name| name.starts_with("game-engine-") && name.ends_with(".sock"))
        })
        .collect()
}

fn wait_for_new_socket(existing: &HashSet<PathBuf>, child: &mut Child) -> Result<PathBuf, String> {
    let deadline = Instant::now() + SOCKET_WAIT_TIMEOUT;
    while Instant::now() < deadline {
        if let Some(status) = child.try_wait().map_err(|err| format!("{err}"))? {
            return Err(format!(
                "m2debug client exited before socket appeared: {status}\nstderr:\n{}",
                child_stderr_tail(child)
            ));
        }
        let current = list_engine_sockets();
        if let Some(path) = current.into_iter().find(|path| !existing.contains(path)) {
            return Ok(path);
        }
        thread::sleep(Duration::from_millis(250));
    }
    Err(format!(
        "timed out waiting for m2debug IPC socket after {:?}\nstderr:\n{}",
        SOCKET_WAIT_TIMEOUT,
        child_stderr_tail(child)
    ))
}

fn wait_for_m2debug_scene(socket: &Path, child: &mut Child) -> Result<String, String> {
    let deadline = Instant::now() + SCENE_READY_TIMEOUT;
    let mut last_scene = String::new();
    while Instant::now() < deadline {
        if let Some(status) = child.try_wait().map_err(|err| format!("{err}"))? {
            return Err(format!(
                "m2debug client exited before scene became ready: {status}\nlast scene:\n{last_scene}\nstderr:\n{}",
                child_stderr_tail(child)
            ));
        }
        match request_tree(socket, Request::DumpScene { filter: None }) {
            Ok(scene) => {
                last_scene = scene;
                if last_scene.contains("M2DebugScene")
                    && last_scene.contains("ReferenceModel")
                    && last_scene.contains("is_displayed=true")
                {
                    return Ok(last_scene);
                }
            }
            Err(_) => {}
        }
        thread::sleep(Duration::from_millis(500));
    }
    Err(format!(
        "timed out waiting for m2debug scene readiness after {:?}\nlast scene:\n{last_scene}\nstderr:\n{}",
        SCENE_READY_TIMEOUT,
        child_stderr_tail(child)
    ))
}

fn request_tree(socket: &Path, request: Request) -> Result<String, String> {
    match Client::call(socket, &request).map_err(|err| format!("{err}"))? {
        Response::Tree(tree) => Ok(tree),
        Response::Error(message) => Err(message),
        other => Err(format!("unexpected IPC response: {other:?}")),
    }
}

fn request_screenshot(socket: &Path) -> Result<Vec<u8>, String> {
    match Client::call(socket, &Request::Screenshot).map_err(|err| format!("{err}"))? {
        Response::Screenshot(bytes) => Ok(bytes),
        Response::Error(message) => Err(message),
        other => Err(format!("unexpected IPC response: {other:?}")),
    }
}

fn stop_child(child: &mut Child) {
    let _ = child.kill();
    let _ = child.wait();
}

fn child_stderr_tail(child: &mut Child) -> String {
    let stderr = child
        .stderr
        .take()
        .map(|handle| std::io::read_to_string(handle).unwrap_or_default())
        .unwrap_or_default();
    tail_with_limit(&stderr, 20_000)
}

fn tail_with_limit(text: &str, max_chars: usize) -> String {
    let total_chars = text.chars().count();
    if total_chars <= max_chars {
        return text.to_string();
    }
    text.chars()
        .skip(total_chars.saturating_sub(max_chars))
        .collect()
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

fn decode_webp_bytes(bytes: &[u8]) -> DecodedImage {
    let image = Decoder::new(bytes)
        .decode()
        .unwrap_or_else(|| panic!("failed to decode {} bytes of webp", bytes.len()));
    DecodedImage {
        width: image.width(),
        height: image.height(),
        channels: image.layout().bytes_per_pixel() as usize,
        pixels: image.to_vec(),
    }
}

fn visible_pixel_ratio_outside_overlay(image: &DecodedImage) -> f64 {
    assert!(
        image.channels >= 3,
        "expected RGB or RGBA screenshot, got {} channels",
        image.channels
    );
    let overlay_width = OVERLAY_EXCLUSION_WIDTH.min(image.width);
    let overlay_height = OVERLAY_EXCLUSION_HEIGHT.min(image.height);
    let mut visible_pixels = 0usize;
    let mut sampled_pixels = 0usize;
    for y in 0..image.height as usize {
        for x in 0..image.width as usize {
            if (x as u32) < overlay_width && (y as u32) < overlay_height {
                continue;
            }
            sampled_pixels += 1;
            let idx = (y * image.width as usize + x) * image.channels;
            let rgb_max = image.pixels[idx..idx + 3]
                .iter()
                .copied()
                .max()
                .unwrap_or_default();
            if rgb_max > VISIBLE_PIXEL_THRESHOLD {
                visible_pixels += 1;
            }
        }
    }
    visible_pixels as f64 / sampled_pixels.max(1) as f64
}
