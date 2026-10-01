//! Native diagnostics. Only wire values cross the socket thread boundary.
//!
//! MAIN calls `start` in ready, `poll` on the main thread each process frame,
//! and drops this value in exit_tree, before Godot singleton teardown.

mod dev;
mod export;
mod tree;
mod ui_tree;
mod world;

use std::{
    cell::RefCell,
    path::PathBuf,
    rc::Rc,
    sync::mpsc,
    thread::{self, JoinHandle},
    time::Instant,
};

use game_engine_network::ipc_wire::{PerformanceSnapshot, Request, Response};
use godot::{
    classes::{Engine, Node, RenderingServer, Viewport},
    prelude::*,
};
use peercred_ipc::{Connection, Server};
use tokio::{sync::oneshot, task::JoinSet};

pub(crate) fn dump_tree(client: &Gd<Node>, filter: Option<&str>) -> Response {
    tree::dump_tree(client, filter)
}

pub(crate) fn dump_ui_tree(client: &Gd<Node>, filter: Option<&str>) -> Response {
    ui_tree::dump_mounted_ui(client, filter)
}

pub(crate) use export::export_scene;

struct Command {
    request: Request,
    respond: oneshot::Sender<Response>,
}

type ScreenshotReplies = Rc<RefCell<Vec<oneshot::Sender<Response>>>>;

/// The client's own requests (dev tooling over its live state); a request it does not
/// serve comes back.
pub(crate) type ClientRequests<'a> = dyn FnMut(Request) -> Result<Response, Request> + 'a;

/// Own-PID listener plus main-thread diagnostics dispatch. Never move to a worker.
pub(crate) struct NativeIpc {
    commands: mpsc::Receiver<Command>,
    stop: Option<oneshot::Sender<()>>,
    worker: Option<JoinHandle<()>>,
    screenshots: ScreenshotReplies,
    post_draw: Option<Callable>,
    previous_frame: Option<Instant>,
    frame_time_ms: Option<f64>,
}

impl NativeIpc {
    /// Returns only after binding /tmp/game-engine-<this process PID>.sock.
    pub(crate) fn start() -> Result<Self, String> {
        let path = PathBuf::from(format!("/tmp/game-engine-{}.sock", std::process::id()));
        let (send, commands) = mpsc::channel();
        let (ready_send, ready_receive) = mpsc::sync_channel(1);
        let (stop, stopped) = oneshot::channel();
        let worker = thread::Builder::new()
            .name("native-ipc".into())
            .spawn(move || run_socket_worker(path, send, ready_send, stopped))
            .map_err(|error| format!("native IPC: spawn: {error}"))?;
        let ready = ready_receive
            .recv()
            .map_err(|error| format!("native IPC: startup channel: {error}"))
            .and_then(|result| result);
        if let Err(error) = ready {
            if worker.join().is_err() {
                eprintln!("native IPC: worker panicked during startup");
            }
            return Err(error);
        }
        Ok(Self {
            commands,
            stop: Some(stop),
            worker: Some(worker),
            screenshots: Rc::new(RefCell::new(Vec::new())),
            post_draw: None,
            previous_frame: None,
            frame_time_ms: None,
        })
    }

    /// Call once per process frame on Godot's main thread with the mounted client.
    pub(crate) fn poll(
        &mut self,
        client: &Gd<Node>,
        requests: &mut ClientRequests,
    ) -> Result<(), String> {
        self.measure_frame();
        loop {
            match self.commands.try_recv() {
                Ok(command) => self.dispatch(client, requests, command),
                Err(mpsc::TryRecvError::Empty) => return Ok(()),
                Err(mpsc::TryRecvError::Disconnected) => {
                    return Err("native IPC: socket worker stopped".into());
                }
            }
        }
    }

    fn measure_frame(&mut self) {
        let now = Instant::now();
        self.frame_time_ms = self
            .previous_frame
            .replace(now)
            .map(|previous| now.duration_since(previous).as_secs_f64() * 1000.0)
            .filter(|value| value.is_finite() && *value > 0.0);
    }

    fn dispatch(&mut self, client: &Gd<Node>, requests: &mut ClientRequests, command: Command) {
        let response = match command.request {
            Request::Ping => Response::Pong,
            Request::DumpTree { filter } => tree::dump_tree(client, filter.as_deref()),
            Request::DumpUiTree { filter } => ui_tree::dump_mounted_ui(client, filter.as_deref()),
            // The original scene dispatcher ignores this filter.
            Request::DumpScene { filter: _ } => tree::dump_scene(client),
            Request::Performance => self.performance(client),
            Request::Screenshot => {
                self.queue_screenshot(client, command.respond);
                return;
            }
            request => requests(request).unwrap_or_else(|request| {
                Response::Error(format!("native IPC: unported request {request:?}"))
            }),
        };
        reply(command.respond, response);
    }

    fn performance(&self, client: &Gd<Node>) -> Response {
        let fps = Engine::singleton().get_frames_per_second();
        let focused = client.get_window().is_some_and(|window| window.has_focus());
        Response::Performance(PerformanceSnapshot {
            fps: (fps.is_finite() && fps > 0.0).then_some(fps),
            frame_time_ms: self.frame_time_ms,
            focused,
        })
    }

    fn queue_screenshot(&mut self, client: &Gd<Node>, respond: oneshot::Sender<Response>) {
        if self.post_draw.is_none() {
            let Some(viewport) = client.get_viewport() else {
                reply(
                    respond,
                    Response::Error("screenshot: client has no viewport".into()),
                );
                return;
            };
            let replies = self.screenshots.clone();
            let callback = Callable::from_fn("native-ipc-frame-post-draw", move |_| {
                capture_pending(&viewport, &replies);
            });
            let error = RenderingServer::singleton().connect("frame_post_draw", &callback);
            if error != godot::global::Error::OK {
                reply(
                    respond,
                    Response::Error(format!("screenshot: frame_post_draw connection: {error:?}")),
                );
                return;
            }
            self.post_draw = Some(callback);
        }
        self.screenshots.borrow_mut().push(respond);
    }
}

impl Drop for NativeIpc {
    fn drop(&mut self) {
        if let Some(callback) = self.post_draw.take() {
            let mut server = RenderingServer::singleton();
            if server.is_connected("frame_post_draw", &callback) {
                server.disconnect("frame_post_draw", &callback);
            }
        }
        for respond in self.screenshots.borrow_mut().drain(..) {
            reply(
                respond,
                Response::Error("native IPC: client exited before capture".into()),
            );
        }
        if let Some(stop) = self.stop.take() {
            // A closed receiver means the worker already exited.
            let _ = stop.send(());
        }
        if let Some(worker) = self.worker.take() {
            if worker.join().is_err() {
                eprintln!("native IPC: worker panicked during shutdown");
            }
        }
    }
}

fn capture_pending(viewport: &Gd<Viewport>, replies: &ScreenshotReplies) {
    let pending = std::mem::take(&mut *replies.borrow_mut());
    if pending.is_empty() {
        return;
    }
    let image = encode_viewport(viewport);
    for respond in pending {
        let response = match &image {
            Ok(bytes) => Response::Screenshot(bytes.clone()),
            Err(error) => Response::Error(error.clone()),
        };
        reply(respond, response);
    }
}

fn encode_viewport(viewport: &Gd<Viewport>) -> Result<Vec<u8>, String> {
    if !viewport.is_instance_valid() {
        return Err("screenshot: viewport was freed".into());
    }
    let texture = viewport
        .get_texture()
        .ok_or("screenshot: viewport has no texture")?;
    let image = texture
        .get_image()
        .ok_or("screenshot: viewport texture has no image")?;
    if image.is_empty() {
        return Err("screenshot: viewport image is empty".into());
    }
    // Match the original public screenshot encoder's lossy quality of 65%.
    let bytes = image
        .save_webp_to_buffer_ex()
        .lossy(true)
        .quality(0.65)
        .done()
        .to_vec();
    if bytes.is_empty() {
        return Err("screenshot: Godot WebP encoding failed".into());
    }
    Ok(bytes)
}

fn reply(respond: oneshot::Sender<Response>, response: Response) {
    if respond.send(response).is_err() {
        eprintln!("native IPC: response receiver closed");
    }
}

struct SocketGuard(PathBuf);

impl Drop for SocketGuard {
    fn drop(&mut self) {
        match std::fs::remove_file(&self.0) {
            Ok(()) => {}
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
            Err(error) => eprintln!("native IPC: remove {}: {error}", self.0.display()),
        }
    }
}

fn run_socket_worker(
    path: PathBuf,
    commands: mpsc::Sender<Command>,
    ready: mpsc::SyncSender<Result<(), String>>,
    stopped: oneshot::Receiver<()>,
) {
    let runtime = match tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
    {
        Ok(runtime) => runtime,
        Err(error) => {
            let _ = ready.send(Err(format!("native IPC: runtime: {error}")));
            return;
        }
    };
    runtime.block_on(async move {
        // peercred-ipc replaces only this exact own-PID path and retains legacy 0666 mode.
        // No process-wide signal handlers, stale socket scans or caller authorization.
        // Also removes a partially bound own-PID socket if setting its mode fails.
        let _socket = SocketGuard(path.clone());
        let server = match Server::bind(&path) {
            Ok(server) => server,
            Err(error) => {
                let _ = ready.send(Err(format!("native IPC: bind {}: {error}", path.display())));
                return;
            }
        };
        if ready.send(Ok(())).is_err() {
            eprintln!("native IPC: startup receiver closed");
            return;
        }
        serve(server, commands, stopped).await;
    });
}

async fn serve(
    server: Server,
    commands: mpsc::Sender<Command>,
    mut stopped: oneshot::Receiver<()>,
) {
    let mut connections = JoinSet::new();
    loop {
        tokio::select! {
            _ = &mut stopped => break,
            accepted = server.accept() => {
                match accepted {
                    Ok((connection, _caller)) => {
                        connections.spawn(handle_connection(connection, commands.clone()));
                    }
                    Err(error) => {
                        eprintln!("native IPC: accept: {error}");
                        break;
                    }
                }
            }
            result = connections.join_next(), if !connections.is_empty() => {
                if let Some(Err(error)) = result {
                    eprintln!("native IPC: connection task: {error}");
                }
            }
        }
    }
    connections.abort_all();
    while let Some(result) = connections.join_next().await {
        if let Err(error) = result {
            if !error.is_cancelled() {
                eprintln!("native IPC: connection shutdown: {error}");
            }
        }
    }
}

async fn handle_connection(mut connection: Connection, commands: mpsc::Sender<Command>) {
    let request = match connection.read::<Request>().await {
        Ok(request) => request,
        Err(error) => {
            eprintln!("native IPC: read: {error}");
            return;
        }
    };
    let (respond, response) = oneshot::channel();
    let reply = if commands.send(Command { request, respond }).is_err() {
        Response::Error("native IPC: main-thread command channel closed".into())
    } else {
        // Preserve the original engine's no-response-timeout contract, without blocking Tokio.
        response
            .await
            .unwrap_or_else(|_| Response::Error("internal: channel closed".into()))
    };
    if let Err(error) = connection.write(&reply).await {
        eprintln!("native IPC: write: {error}");
    }
}
