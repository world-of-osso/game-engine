use std::collections::HashSet;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Child, Command};
use std::thread;
use std::time::{Duration, Instant};

use webp::Decoder;

const TIMEOUT: Duration = Duration::from_secs(120);
const SKYBOX_TIME_MS: &str = "0";
const EDGE_LUMA_THRESHOLD: f64 = 8.0;
const HIGHLIGHT_LUMA_THRESHOLD: f64 = 96.0;
const COSTAL_ISLAND_SKYBOX_FDID: u32 = 525_142;

#[derive(Clone, Copy)]
enum SkyboxCaptureMode {
    DefaultLookup,
    LightSkyboxId(u32),
    SkyboxFdid(u32),
}

impl SkyboxCaptureMode {
    fn append_args(self, command: &mut Command) {
        match self {
            Self::DefaultLookup => {}
            Self::LightSkyboxId(id) => {
                command.arg("--light-skybox-id").arg(id.to_string());
            }
            Self::SkyboxFdid(fdid) => {
                command.arg("--skybox-fdid").arg(fdid.to_string());
            }
        }
    }
}

struct SkyboxScreenshotCase {
    slug: &'static str,
    skybox_fdid: u32,
    description: &'static str,
    min_highlight_ratio: f64,
    min_edge_ratio: f64,
    min_quantized_color_buckets: usize,
    min_luma_stddev: f64,
}

const CASES: &[SkyboxScreenshotCase] = &[
    SkyboxScreenshotCase {
        slug: "cloud-11xp",
        skybox_fdid: 5_412_968,
        description: "11xp_cloudsky01.m2 authored stars and cloud highlights",
        min_highlight_ratio: 0.0002,
        min_edge_ratio: 0.00035,
        min_quantized_color_buckets: 20,
        min_luma_stddev: 4.0,
    },
    SkyboxScreenshotCase {
        slug: "deathskybox",
        skybox_fdid: 235_313,
        description: "deathskybox.m2 authored high-contrast sky layers",
        min_highlight_ratio: 0.25,
        min_edge_ratio: 0.002,
        min_quantized_color_buckets: 80,
        min_luma_stddev: 20.0,
    },
];

struct SkyboxLookupCase {
    slug: &'static str,
    description: &'static str,
    lookup_mode: SkyboxCaptureMode,
    expected_fdid: u32,
    min_highlight_ratio: f64,
    min_edge_ratio: f64,
    min_quantized_color_buckets: usize,
    min_luma_stddev: f64,
    max_mean_abs_rgb_diff: f64,
}

struct LookupMetricContext<'a> {
    case: &'a SkyboxLookupCase,
    output_path: &'a Path,
    metrics: &'a ImageMetrics,
    sample_label: &'a str,
}

const LOOKUP_CASES: &[SkyboxLookupCase] = &[
    SkyboxLookupCase {
        slug: "default-lookup",
        description: "default skyboxdebug lookup should render costalislandskybox.m2",
        lookup_mode: SkyboxCaptureMode::DefaultLookup,
        expected_fdid: COSTAL_ISLAND_SKYBOX_FDID,
        min_highlight_ratio: 0.00005,
        min_edge_ratio: 0.0002,
        min_quantized_color_buckets: 12,
        min_luma_stddev: 1.0,
        max_mean_abs_rgb_diff: 1.5,
    },
    SkyboxLookupCase {
        slug: "light-skybox-id-653",
        description: "forced LightSkyboxID=653 should render 11xp_cloudsky01.m2",
        lookup_mode: SkyboxCaptureMode::LightSkyboxId(653),
        expected_fdid: 5_412_968,
        min_highlight_ratio: 0.0002,
        min_edge_ratio: 0.00035,
        min_quantized_color_buckets: 20,
        min_luma_stddev: 4.0,
        max_mean_abs_rgb_diff: 1.5,
    },
];

#[test]
#[ignore = "requires windowing/GPU access and is run via scripts/run_skybox_screenshot_regression.sh"]
fn authored_skybox_models_show_structured_pixels_in_verify_mode() {
    let workdir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let temp_root = temp_dir("skybox-screenshot-regression");

    for case in CASES {
        assert_skybox_case_has_authored_pixels(case, &workdir, &temp_root);
    }
}

#[test]
#[ignore = "requires windowing/GPU access and is run via scripts/run_skybox_screenshot_regression.sh"]
fn skyboxdebug_lookup_modes_match_expected_authored_skybox_render_in_verify_mode() {
    let workdir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let temp_root = temp_dir("skybox-lookup-regression");

    for case in LOOKUP_CASES {
        assert_lookup_mode_matches_expected_skybox(case, &workdir, &temp_root);
    }
}

fn assert_skybox_case_has_authored_pixels(
    case: &SkyboxScreenshotCase,
    workdir: &Path,
    temp_root: &Path,
) {
    let output_path = capture_skybox_case_output(case, workdir, temp_root);
    let metrics = compute_image_metrics(&decode_webp(&output_path));
    assert_case_metrics(case, &output_path, &metrics);
}

fn capture_skybox_case_output(
    case: &SkyboxScreenshotCase,
    workdir: &Path,
    temp_root: &Path,
) -> PathBuf {
    let output_path = temp_root.join(format!("{}.webp", case.slug));
    let status = wait_for_child(
        spawn_capture_command(
            SkyboxCaptureMode::SkyboxFdid(case.skybox_fdid),
            workdir,
            temp_root,
            &output_path,
        )
        .expect("failed to spawn skybox screenshot regression command"),
    )
    .expect("skybox screenshot regression command should finish successfully");
    assert!(
        status.success(),
        "skybox screenshot capture failed for {} (fdid={}): {status}",
        case.slug,
        case.skybox_fdid
    );
    output_path
}

fn assert_lookup_mode_matches_expected_skybox(
    case: &SkyboxLookupCase,
    workdir: &Path,
    temp_root: &Path,
) {
    let actual_output =
        capture_lookup_mode_output(case, "lookup", case.lookup_mode, workdir, temp_root);
    let expected_output = capture_lookup_mode_output(
        case,
        "fdid",
        SkyboxCaptureMode::SkyboxFdid(case.expected_fdid),
        workdir,
        temp_root,
    );
    let actual = decode_webp(&actual_output);
    let expected = decode_webp(&expected_output);
    let actual_metrics = compute_image_metrics(&actual);
    let expected_metrics = compute_image_metrics(&expected);

    assert_lookup_metrics(case, &actual_output, &actual_metrics, "lookup-mode output");
    assert_lookup_metrics(
        case,
        &expected_output,
        &expected_metrics,
        "explicit-fdid baseline",
    );

    let mean_diff = mean_abs_rgb_diff(&actual, &expected);
    assert!(
        mean_diff <= case.max_mean_abs_rgb_diff,
        "{} did not match explicit-fdid baseline render: mean_abs_rgb_diff={:.4} > {:.4}; lookup_output={}; baseline_output={}",
        case.description,
        mean_diff,
        case.max_mean_abs_rgb_diff,
        actual_output.display(),
        expected_output.display()
    );
}

fn capture_lookup_mode_output(
    case: &SkyboxLookupCase,
    suffix: &str,
    mode: SkyboxCaptureMode,
    workdir: &Path,
    temp_root: &Path,
) -> PathBuf {
    let output_path = temp_root.join(format!("{}-{suffix}.webp", case.slug));
    let status = wait_for_child(
        spawn_capture_command(mode, workdir, temp_root, &output_path)
            .expect("failed to spawn skybox lookup screenshot regression command"),
    )
    .expect("skybox lookup screenshot regression command should finish successfully");
    assert!(
        status.success(),
        "skybox lookup screenshot capture failed for {} ({suffix}): {status}",
        case.slug
    );
    output_path
}

fn assert_case_metrics(case: &SkyboxScreenshotCase, output_path: &Path, metrics: &ImageMetrics) {
    let context = case_failure_context(case, output_path, metrics);
    assert!(
        metrics.highlight_ratio >= case.min_highlight_ratio,
        "{} did not meet authored-skybox highlight ratio floor: {:.6} < {:.6}; {}",
        case.description,
        metrics.highlight_ratio,
        case.min_highlight_ratio,
        context
    );
    assert!(
        metrics.edge_ratio >= case.min_edge_ratio,
        "{} did not meet authored-skybox edge/detail floor: {:.6} < {:.6}; {}",
        case.description,
        metrics.edge_ratio,
        case.min_edge_ratio,
        context
    );
    assert!(
        metrics.quantized_color_buckets >= case.min_quantized_color_buckets,
        "{} did not meet authored-skybox color diversity floor: {} < {}; {}",
        case.description,
        metrics.quantized_color_buckets,
        case.min_quantized_color_buckets,
        context
    );
    assert!(
        metrics.luma_stddev >= case.min_luma_stddev,
        "{} did not meet authored-skybox luma variation floor: {:.4} < {:.4}; {}",
        case.description,
        metrics.luma_stddev,
        case.min_luma_stddev,
        context
    );
}

fn assert_lookup_metrics(
    case: &SkyboxLookupCase,
    output_path: &Path,
    metrics: &ImageMetrics,
    sample_label: &str,
) {
    let context = LookupMetricContext {
        case,
        output_path,
        metrics,
        sample_label,
    };
    for (name, actual, minimum) in [
        (
            "highlight ratio",
            metrics.highlight_ratio,
            case.min_highlight_ratio,
        ),
        ("edge/detail ratio", metrics.edge_ratio, case.min_edge_ratio),
        ("luma variation", metrics.luma_stddev, case.min_luma_stddev),
    ] {
        assert_lookup_metric_floor(
            &context,
            name,
            actual >= minimum,
            format!("{actual:.6}"),
            format!("{minimum:.6}"),
        );
    }
    assert_lookup_metric_floor(
        &context,
        "color diversity",
        metrics.quantized_color_buckets >= case.min_quantized_color_buckets,
        metrics.quantized_color_buckets.to_string(),
        case.min_quantized_color_buckets.to_string(),
    );
}

fn assert_lookup_metric_floor(
    context: &LookupMetricContext<'_>,
    metric_name: &str,
    passes: bool,
    actual: String,
    minimum: String,
) {
    assert!(
        passes,
        "{} {} did not meet authored-skybox {metric_name} floor: {} < {}; output={}; metrics={metrics:?}",
        context.case.description,
        context.sample_label,
        actual,
        minimum,
        context.output_path.display(),
        metrics = context.metrics
    );
}

fn case_failure_context(
    case: &SkyboxScreenshotCase,
    output_path: &Path,
    metrics: &ImageMetrics,
) -> String {
    format!(
        "fdid={}; output={}; metrics={metrics:?}",
        case.skybox_fdid,
        output_path.display()
    )
}

fn spawn_capture_command(
    mode: SkyboxCaptureMode,
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
        .arg("--screen")
        .arg("skyboxdebug")
        .arg("--skybox-verify")
        .arg("--skybox-time-ms")
        .arg(SKYBOX_TIME_MS);
    mode.append_args(&mut command);
    command.arg("screenshot").arg(output_path);
    command
        .spawn()
        .map_err(|err| format!("failed to spawn skybox screenshot regression binary: {err}"))
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
            .map_err(|err| format!("failed to wait for skybox screenshot child: {err}"))?
        {
            return Ok(status);
        }
        if Instant::now() >= deadline {
            let _ = child.kill();
            let _ = child.wait();
            return Err(format!(
                "skybox screenshot regression command timed out after {}s",
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

#[derive(Debug)]
struct ImageMetrics {
    highlight_ratio: f64,
    edge_ratio: f64,
    quantized_color_buckets: usize,
    luma_stddev: f64,
}

fn compute_image_metrics(image: &DecodedImage) -> ImageMetrics {
    assert!(
        image.channels >= 3,
        "expected RGB or RGBA screenshot, got {} channels",
        image.channels
    );
    let width = image.width as usize;
    let height = image.height as usize;
    let totals = scan_image_pixels(image, width, height);

    finalize_image_metrics(&totals, width, height)
}

struct MetricTotals {
    highlight_pixels: usize,
    edge_pixels: usize,
    luma_sum: f64,
    luma_sq_sum: f64,
    quantized_colors: HashSet<u16>,
}

fn scan_image_pixels(image: &DecodedImage, width: usize, height: usize) -> MetricTotals {
    let mut totals = MetricTotals {
        highlight_pixels: 0,
        edge_pixels: 0,
        luma_sum: 0.0,
        luma_sq_sum: 0.0,
        quantized_colors: HashSet::new(),
    };
    for y in 0..height {
        for x in 0..width {
            accumulate_pixel_metrics(image, width, height, x, y, &mut totals);
        }
    }
    totals
}

fn accumulate_pixel_metrics(
    image: &DecodedImage,
    width: usize,
    height: usize,
    x: usize,
    y: usize,
    totals: &mut MetricTotals,
) {
    let idx = (y * width + x) * image.channels;
    let r = image.pixels[idx];
    let g = image.pixels[idx + 1];
    let b = image.pixels[idx + 2];
    let luma = srgb_luma(r, g, b);
    totals.luma_sum += luma;
    totals.luma_sq_sum += luma * luma;
    if luma > HIGHLIGHT_LUMA_THRESHOLD {
        totals.highlight_pixels += 1;
    }
    totals.quantized_colors.insert(quantized_color_key(r, g, b));
    if edge_pixel_exceeds_threshold(image, width, height, x, y, luma) {
        totals.edge_pixels += 1;
    }
}

fn edge_pixel_exceeds_threshold(
    image: &DecodedImage,
    width: usize,
    height: usize,
    x: usize,
    y: usize,
    luma: f64,
) -> bool {
    if x + 1 >= width || y + 1 >= height {
        return false;
    }
    let right_idx = (y * width + (x + 1)) * image.channels;
    let down_idx = ((y + 1) * width + x) * image.channels;
    let right_luma = srgb_luma(
        image.pixels[right_idx],
        image.pixels[right_idx + 1],
        image.pixels[right_idx + 2],
    );
    let down_luma = srgb_luma(
        image.pixels[down_idx],
        image.pixels[down_idx + 1],
        image.pixels[down_idx + 2],
    );
    (right_luma - luma).abs() > EDGE_LUMA_THRESHOLD
        || (down_luma - luma).abs() > EDGE_LUMA_THRESHOLD
}

fn finalize_image_metrics(totals: &MetricTotals, width: usize, height: usize) -> ImageMetrics {
    let total_pixels = width * height;
    let edge_sample_count = (width.saturating_sub(1)) * (height.saturating_sub(1));
    let mean = totals.luma_sum / total_pixels.max(1) as f64;
    let variance = (totals.luma_sq_sum / total_pixels.max(1) as f64) - (mean * mean);

    ImageMetrics {
        highlight_ratio: totals.highlight_pixels as f64 / total_pixels.max(1) as f64,
        edge_ratio: totals.edge_pixels as f64 / edge_sample_count.max(1) as f64,
        quantized_color_buckets: totals.quantized_colors.len(),
        luma_stddev: variance.max(0.0).sqrt(),
    }
}

fn srgb_luma(r: u8, g: u8, b: u8) -> f64 {
    0.2126 * (r as f64) + 0.7152 * (g as f64) + 0.0722 * (b as f64)
}

fn quantized_color_key(r: u8, g: u8, b: u8) -> u16 {
    let rq = (r >> 4) as u16;
    let gq = (g >> 4) as u16;
    let bq = (b >> 4) as u16;
    (rq << 8) | (gq << 4) | bq
}

fn mean_abs_rgb_diff(left: &DecodedImage, right: &DecodedImage) -> f64 {
    assert_eq!(
        (left.width, left.height),
        (right.width, right.height),
        "image dimensions should match for comparison"
    );
    assert!(
        left.channels >= 3 && right.channels >= 3,
        "expected RGB or RGBA screenshots for comparison"
    );
    let mut total_diff = 0.0;
    let pixel_count = (left.width as usize) * (left.height as usize);
    for pixel_index in 0..pixel_count {
        let left_idx = pixel_index * left.channels;
        let right_idx = pixel_index * right.channels;
        for channel in 0..3 {
            total_diff += (left.pixels[left_idx + channel] as f64
                - right.pixels[right_idx + channel] as f64)
                .abs();
        }
    }
    total_diff / ((pixel_count * 3).max(1) as f64)
}
