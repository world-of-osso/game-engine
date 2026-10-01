//! Consume a PipeWire video node that only offers DMA-BUFs (niri's screencast streams) and write
//! constant-rate raw BGRx frames to stdout for ffmpeg.
//!
//! Usage: pw-video-cat --node <id> [--fps <n>]
//!
//! Stdout: one header line `<width> <height> <fps>\n`, then `width*height*4`-byte BGRx frames at
//! a fixed rate. Each tick repeats the latest received frame, because niri sends frames only on
//! damage. The output size is the first negotiated size; later sizes are cropped/padded into it.
//! Exits when the node goes away or stdout closes.

use std::collections::HashMap;
use std::io::Write;
use std::os::fd::FromRawFd;
use std::os::fd::RawFd;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::{Arc, Condvar, Mutex};
use std::time::{Duration, Instant};

use pipewire as pw;
use pw::spa;
use pw::spa::param::format::{FormatProperties, MediaSubtype, MediaType};
use pw::spa::param::video::{VideoFormat, VideoInfoRaw};
use pw::spa::pod::{self, ChoiceValue, Pod, Property, PropertyFlags};
use pw::spa::utils::{Choice, ChoiceEnum, ChoiceFlags, Fraction, Rectangle, SpaTypes};

/// DRM_FORMAT_MOD_LINEAR: the only layout a CPU mmap can read row by row.
const MODIFIER_LINEAR: i64 = 0;
/// DMA_BUF_IOCTL_SYNC = _IOW('b', 0, struct dma_buf_sync { u64 flags; }).
const DMA_BUF_IOCTL_SYNC: libc::c_ulong = 0x4008_6200;
const DMA_BUF_SYNC_READ: u64 = 1;
const DMA_BUF_SYNC_END: u64 = 4;
const SPA_CHUNK_FLAG_CORRUPTED: i32 = 1;

/// Set by SIGTERM/SIGINT so the writer stops between frames instead of mid-frame.
static STOP: AtomicBool = AtomicBool::new(false);

extern "C" fn request_stop(_: libc::c_int) {
    STOP.store(true, Ordering::Relaxed);
}

#[derive(Default)]
struct Frame {
    width: u32,
    height: u32,
    /// Tightly packed BGRx rows.
    pixels: Vec<u8>,
}

#[derive(Default)]
struct Shared {
    frame: Mutex<Option<Frame>>,
    received: AtomicU64,
    ready: Condvar,
}

struct Mapping {
    ptr: *mut libc::c_void,
    len: usize,
}

impl Drop for Mapping {
    fn drop(&mut self) {
        unsafe { libc::munmap(self.ptr, self.len) };
    }
}

struct StreamState {
    format: VideoInfoRaw,
    mappings: HashMap<RawFd, Mapping>,
    shared: Arc<Shared>,
}

fn main() {
    let (node, fps) = match parse_args(std::env::args().skip(1)) {
        Ok(args) => args,
        Err(message) => {
            eprintln!("pw-video-cat: {message}\nusage: pw-video-cat --node <id> [--fps <n>]");
            std::process::exit(2);
        }
    };
    if let Err(error) = run(node, fps) {
        eprintln!("pw-video-cat: {error}");
        std::process::exit(1);
    }
}

fn parse_args(mut args: impl Iterator<Item = String>) -> Result<(u32, u32), String> {
    let (mut node, mut fps) = (None, 30);
    while let Some(flag) = args.next() {
        let value = args.next().ok_or(format!("{flag} needs a value"))?;
        let number = value
            .parse()
            .map_err(|_| format!("{flag}: not a number: {value}"))?;
        match flag.as_str() {
            "--node" => node = Some(number),
            "--fps" if number > 0 => fps = number,
            _ => return Err(format!("unknown argument {flag}")),
        }
    }
    Ok((node.ok_or("--node is required")?, fps))
}

fn install_stop_handlers() {
    for signal in [libc::SIGTERM, libc::SIGINT] {
        unsafe { libc::signal(signal, request_stop as *const () as libc::sighandler_t) };
    }
}

fn run(node: u32, fps: u32) -> Result<(), pw::Error> {
    install_stop_handlers();
    pw::init();
    let mainloop = pw::main_loop::MainLoopRc::new(None)?;
    let context = pw::context::ContextRc::new(&mainloop, None)?;
    let core = context.connect_rc(None)?;
    let stream = pw::stream::StreamBox::new(
        &core,
        "pw-video-cat",
        pw::properties::properties! {
            *pw::keys::MEDIA_TYPE => "Video",
            *pw::keys::MEDIA_CATEGORY => "Capture",
            *pw::keys::MEDIA_ROLE => "Screen",
        },
    )?;
    let shared = Arc::new(Shared::default());
    let state = StreamState {
        format: VideoInfoRaw::default(),
        mappings: HashMap::new(),
        shared: shared.clone(),
    };
    let ended = shared.clone();
    let _listener = stream
        .add_local_listener_with_user_data(state)
        .state_changed(move |_, _, old, new| {
            eprintln!("pw-video-cat: stream {old:?} -> {new:?}");
            // The node went away (window closed or cast stopped): let the writer finish its frame.
            if matches!(
                new,
                pw::stream::StreamState::Error(_) | pw::stream::StreamState::Unconnected
            ) {
                STOP.store(true, Ordering::Relaxed);
                ended.ready.notify_all();
            }
        })
        .param_changed(|_, state, id, param| {
            if id == spa::param::ParamType::Format.as_raw()
                && let Some(param) = param
            {
                update_format(state, param);
            }
        })
        .process(|stream, state| {
            if let Some(mut buffer) = stream.dequeue_buffer()
                && let Some(data) = buffer.datas_mut().first_mut()
            {
                copy_frame(state, data);
            }
        })
        .register()?;

    connect_stream(&stream, node, fps)?;
    std::thread::spawn(move || {
        write_frames(&shared, fps);
        // stdout closed, stop requested, or stream ended: the process is done.
        std::process::exit(0);
    });
    mainloop.run();
    Ok(())
}

fn connect_stream(stream: &pw::stream::Stream, node: u32, fps: u32) -> Result<(), pw::Error> {
    let params_bytes = serialize_pod(format_params(fps));
    let mut params = [Pod::from_bytes(&params_bytes).expect("serialized format pod")];
    stream.connect(
        spa::utils::Direction::Input,
        Some(node),
        pw::stream::StreamFlags::AUTOCONNECT,
        &mut params,
    )
}

/// BGRx/BGRA with a LINEAR DMA-BUF modifier (niri allocates exactly what we accept).
fn format_params(fps: u32) -> pod::Object {
    let mut properties = vec![
        pod::property!(FormatProperties::MediaType, Id, MediaType::Video),
        pod::property!(FormatProperties::MediaSubtype, Id, MediaSubtype::Raw),
        pod::property!(
            FormatProperties::VideoFormat,
            Choice,
            Enum,
            Id,
            VideoFormat::BGRx,
            VideoFormat::BGRx,
            VideoFormat::BGRA
        ),
        Property {
            key: FormatProperties::VideoModifier.as_raw(),
            flags: PropertyFlags::MANDATORY,
            value: pod::Value::Choice(ChoiceValue::Long(Choice(
                ChoiceFlags::empty(),
                ChoiceEnum::Enum {
                    default: MODIFIER_LINEAR,
                    alternatives: vec![MODIFIER_LINEAR],
                },
            ))),
        },
    ];
    properties.extend(size_and_rate_properties(fps));
    pod::Object {
        type_: SpaTypes::ObjectParamFormat.as_raw(),
        id: spa::param::ParamType::EnumFormat.as_raw(),
        properties,
    }
}

/// Any size (niri fixes it to the window); frames arrive on damage, at most `fps` per second.
fn size_and_rate_properties(fps: u32) -> [Property; 3] {
    let size = |width, height| Rectangle { width, height };
    [
        pod::property!(
            FormatProperties::VideoSize,
            Choice,
            Range,
            Rectangle,
            size(1280, 720),
            size(1, 1),
            size(8192, 8192)
        ),
        pod::property!(
            FormatProperties::VideoFramerate,
            Fraction,
            Fraction { num: 0, denom: 1 }
        ),
        pod::property!(
            FormatProperties::VideoMaxFramerate,
            Choice,
            Range,
            Fraction,
            Fraction { num: fps, denom: 1 },
            Fraction { num: 1, denom: 1 },
            Fraction {
                num: 1000,
                denom: 1
            }
        ),
    ]
}

fn serialize_pod(object: pod::Object) -> Vec<u8> {
    pod::serialize::PodSerializer::serialize(
        std::io::Cursor::new(Vec::new()),
        &pod::Value::Object(object),
    )
    .expect("serialize pod")
    .0
    .into_inner()
}

fn update_format(state: &mut StreamState, param: &Pod) {
    let Ok((MediaType::Video, MediaSubtype::Raw)) = spa::param::format_utils::parse_format(param)
    else {
        return;
    };
    if let Err(error) = state.format.parse(param) {
        eprintln!("pw-video-cat: cannot parse video format: {error:?}");
        return;
    }
    let size = state.format.size();
    eprintln!(
        "pw-video-cat: format {:?} {}x{} modifier {:#x}",
        state.format.format(),
        size.width,
        size.height,
        state.format.modifier()
    );
    // Buffers are reallocated after a format change; their fds may be reused.
    state.mappings.clear();
}

fn copy_frame(state: &mut StreamState, data: &mut spa::buffer::Data) {
    let chunk = data.chunk();
    if chunk.flags().bits() & SPA_CHUNK_FLAG_CORRUPTED != 0 {
        return;
    }
    let (offset, stride) = (chunk.offset() as usize, chunk.stride() as usize);
    let size = state.format.size();
    let (width, height) = (size.width as usize, size.height as usize);
    let fd = data.fd();
    let mapping = match state.mappings.entry(fd) {
        std::collections::hash_map::Entry::Occupied(entry) => entry.into_mut(),
        std::collections::hash_map::Entry::Vacant(entry) => match map_dmabuf(fd) {
            Some(mapping) => entry.insert(mapping),
            None => return,
        },
    };
    if stride < width * 4 || offset + stride * height > mapping.len {
        eprintln!(
            "pw-video-cat: {} byte dmabuf too small for {width}x{height} stride {stride}",
            mapping.len
        );
        return;
    }

    let mut pixels = vec![0u8; width * height * 4];
    dmabuf_sync(fd, DMA_BUF_SYNC_READ);
    let base = unsafe { (mapping.ptr as *const u8).add(offset) };
    for row in 0..height {
        let source = unsafe { std::slice::from_raw_parts(base.add(row * stride), width * 4) };
        pixels[row * width * 4..(row + 1) * width * 4].copy_from_slice(source);
    }
    dmabuf_sync(fd, DMA_BUF_SYNC_READ | DMA_BUF_SYNC_END);

    let frame = Frame {
        width: size.width,
        height: size.height,
        pixels,
    };
    *state.shared.frame.lock().unwrap() = Some(frame);
    state.shared.received.fetch_add(1, Ordering::Relaxed);
    state.shared.ready.notify_all();
}

/// Maps the whole dma-buf: niri sets `maxsize` to 1 (consumers must ignore it for DMA-BUFs), so
/// the size comes from seeking the fd.
fn map_dmabuf(fd: RawFd) -> Option<Mapping> {
    let len = unsafe { libc::lseek(fd, 0, libc::SEEK_END) };
    if len <= 0 {
        eprintln!(
            "pw-video-cat: dmabuf fd {fd} size: {}",
            std::io::Error::last_os_error()
        );
        return None;
    }
    let len = len as usize;
    let ptr = unsafe {
        libc::mmap(
            std::ptr::null_mut(),
            len,
            libc::PROT_READ,
            libc::MAP_SHARED,
            fd,
            0,
        )
    };
    if ptr == libc::MAP_FAILED {
        eprintln!(
            "pw-video-cat: mmap dmabuf fd {fd}: {}",
            std::io::Error::last_os_error()
        );
        return None;
    }
    Some(Mapping { ptr, len })
}

fn dmabuf_sync(fd: RawFd, flags: u64) {
    if unsafe { libc::ioctl(fd, DMA_BUF_IOCTL_SYNC, &flags) } != 0 {
        eprintln!(
            "pw-video-cat: DMA_BUF_IOCTL_SYNC: {}",
            std::io::Error::last_os_error()
        );
    }
}

/// Header, then one frame per tick until stdout closes, a stop signal arrives, or the stream ends.
fn write_frames(shared: &Shared, fps: u32) {
    let (width, height) = {
        let mut frame = shared.frame.lock().unwrap();
        while frame.is_none() && !STOP.load(Ordering::Relaxed) {
            frame = shared.ready.wait(frame).unwrap();
        }
        let Some(frame) = frame.as_ref() else {
            eprintln!("pw-video-cat: stream ended before its first frame");
            return;
        };
        (frame.width, frame.height)
    };
    // Raw fd 1: std's stdout is line-buffered and would split frames at 0x0a bytes.
    let mut stdout = std::mem::ManuallyDrop::new(unsafe { std::fs::File::from_raw_fd(1) });
    if writeln!(stdout, "{width} {height} {fps}")
        .and_then(|()| stdout.flush())
        .is_err()
    {
        return;
    }
    let period = Duration::from_secs(1) / fps;
    let mut output = vec![0u8; width as usize * height as usize * 4];
    let start = Instant::now();
    let (mut deadline, mut written) = (start, 0u64);
    while !STOP.load(Ordering::Relaxed) {
        if let Some(frame) = shared.frame.lock().unwrap().as_ref() {
            blit(frame, width as usize, height as usize, &mut output);
        }
        if stdout.write_all(&output).is_err() {
            break;
        }
        written += 1;
        deadline += period;
        std::thread::sleep(deadline.saturating_duration_since(Instant::now()));
    }
    let elapsed = start.elapsed().as_secs_f64();
    let received = shared.received.load(Ordering::Relaxed);
    eprintln!(
        "pw-video-cat: {elapsed:.2} s, {received} frames received ({:.1}/s), {written} written ({:.1}/s)",
        received as f64 / elapsed,
        written as f64 / elapsed
    );
}

/// Copy `frame` into the fixed-size output canvas, cropping or zero-padding a resized window.
fn blit(frame: &Frame, width: usize, height: usize, output: &mut [u8]) {
    if frame.width as usize == width && frame.height as usize == height {
        output.copy_from_slice(&frame.pixels);
        return;
    }
    output.fill(0);
    let copy_width = width.min(frame.width as usize) * 4;
    for row in 0..height.min(frame.height as usize) {
        let source = row * frame.width as usize * 4;
        output[row * width * 4..][..copy_width]
            .copy_from_slice(&frame.pixels[source..source + copy_width]);
    }
}
