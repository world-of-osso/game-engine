use std::path::PathBuf;

use clap::{Parser, Subcommand};
use peercred_ipc::Client;
use game_engine::ipc::{Request, Response, socket_glob};

#[derive(Parser)]
#[command(name = "game-engine-cli", about = "Control a running game-engine instance")]
struct Cli {
    /// Unix socket path (auto-discovered if omitted)
    #[arg(short, long)]
    socket: Option<PathBuf>,

    #[command(subcommand)]
    command: Cmd,
}

#[derive(Subcommand)]
enum Cmd {
    /// Check if the engine is running
    Ping,
    /// Capture a screenshot and save to file
    Screenshot {
        /// Output file path (default: screenshot.webp)
        #[arg(default_value = "screenshot.webp")]
        output: PathBuf,
    },
    /// Dump the entity hierarchy
    DumpTree {
        /// Filter by entity name (case-insensitive substring)
        #[arg(short, long)]
        filter: Option<String>,
    },
    /// Dump the UI frame hierarchy
    DumpUiTree {
        /// Filter by frame name (case-insensitive substring)
        #[arg(short, long)]
        filter: Option<String>,
    },
}

fn find_socket() -> Result<PathBuf, String> {
    let pattern = socket_glob();
    let mut sockets: Vec<PathBuf> = glob::glob(&pattern)
        .map_err(|e| format!("bad glob: {e}"))?
        .filter_map(Result::ok)
        .collect();

    match sockets.len() {
        0 => Err("no running game-engine instance found".into()),
        1 => Ok(sockets.remove(0)),
        n => Err(format!("{n} instances found, specify --socket")),
    }
}

fn main() {
    let cli = Cli::parse();
    let socket = match cli.socket {
        Some(s) => s,
        None => match find_socket() {
            Ok(s) => s,
            Err(e) => {
                eprintln!("error: {e}");
                std::process::exit(1);
            }
        },
    };

    let result = match cli.command {
        Cmd::Ping => handle_ping(&socket),
        Cmd::Screenshot { output } => handle_screenshot(&socket, &output),
        Cmd::DumpTree { filter } => handle_dump_tree(&socket, filter),
        Cmd::DumpUiTree { filter } => handle_dump_ui_tree(&socket, filter),
    };

    if let Err(e) = result {
        eprintln!("error: {e}");
        std::process::exit(1);
    }
}

fn handle_ping(socket: &PathBuf) -> Result<(), String> {
    let resp: Response =
        Client::call(socket, &Request::Ping).map_err(|e| format!("{e}"))?;

    match resp {
        Response::Pong => {
            println!("pong");
            Ok(())
        }
        other => Err(format!("unexpected response: {other:?}")),
    }
}

fn handle_dump_tree(socket: &PathBuf, filter: Option<String>) -> Result<(), String> {
    let resp: Response =
        Client::call(socket, &Request::DumpTree { filter }).map_err(|e| format!("{e}"))?;
    match resp {
        Response::Tree(tree) => {
            println!("{tree}");
            Ok(())
        }
        Response::Error(msg) => Err(msg),
        other => Err(format!("unexpected response: {other:?}")),
    }
}

fn handle_dump_ui_tree(socket: &PathBuf, filter: Option<String>) -> Result<(), String> {
    let resp: Response =
        Client::call(socket, &Request::DumpUiTree { filter }).map_err(|e| format!("{e}"))?;
    match resp {
        Response::Tree(tree) => {
            println!("{tree}");
            Ok(())
        }
        Response::Error(msg) => Err(msg),
        other => Err(format!("unexpected response: {other:?}")),
    }
}

fn handle_screenshot(socket: &PathBuf, output: &PathBuf) -> Result<(), String> {
    let resp: Response =
        Client::call(socket, &Request::Screenshot).map_err(|e| format!("{e}"))?;

    match resp {
        Response::Screenshot(data) => {
            std::fs::write(output, &data)
                .map_err(|e| format!("failed to write {}: {e}", output.display()))?;
            println!("saved {} ({} bytes)", output.display(), data.len());
            Ok(())
        }
        Response::Error(msg) => Err(msg),
        other => Err(format!("unexpected response: {other:?}")),
    }
}
