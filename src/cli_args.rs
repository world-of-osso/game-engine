//! Command-line argument parsing for game-engine.

use std::path::PathBuf;
use std::str::FromStr;

use crate::ScreenshotRequest;
use crate::game_state;
use game_engine::game_state_enum::ScreenArg;

#[cfg(debug_assertions)]
pub const DEFAULT_SERVER_ADDR: &str = "127.0.0.1:5000";
#[cfg(not(debug_assertions))]
pub const DEFAULT_SERVER_ADDR: &str = "game.worldofosso.com:5000";

pub fn screenshot_arg_index(args: &[String]) -> Option<usize> {
    args.iter().position(|arg| arg == "screenshot")
}

pub fn parse_screenshot_args(args: &[String]) -> Option<ScreenshotRequest> {
    let screenshot_idx = screenshot_arg_index(args)?;
    let output = args
        .get(screenshot_idx + 1)
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from("screenshot.webp"));
    let has_server = args.windows(2).any(|w| w[0] == "--server");
    let has_state = args
        .windows(2)
        .any(|w| w[0] == "--state" || w[0] == "--screen");
    let frames = if has_server {
        60
    } else if has_state {
        10
    } else {
        3
    };
    Some(ScreenshotRequest {
        output,
        frames_remaining: frames,
    })
}

#[derive(Debug, PartialEq)]
pub struct ServerArg {
    pub addr: std::net::SocketAddr,
    pub hostname: String,
    pub dev: bool,
}

impl ServerArg {
    pub fn dev() -> Self {
        Self {
            addr: "127.0.0.1:5000".parse().unwrap(),
            hostname: "127.0.0.1:5000".to_string(),
            dev: true,
        }
    }

    pub fn prod() -> Self {
        default_server_arg_for("game.worldofosso.com:5000", false)
            .expect("prod server alias should resolve")
    }
}

pub fn default_server_arg() -> Result<ServerArg, String> {
    default_server_arg_for(DEFAULT_SERVER_ADDR, cfg!(debug_assertions))
}

fn default_server_arg_for(hostname: &str, dev: bool) -> Result<ServerArg, String> {
    use std::net::ToSocketAddrs;

    let addr = hostname
        .parse()
        .ok()
        .or_else(|| hostname.to_socket_addrs().ok()?.next())
        .ok_or_else(|| format!("failed to resolve default server address '{hostname}'"))?;
    Ok(ServerArg {
        addr,
        hostname: hostname.to_string(),
        dev,
    })
}

pub fn parse_server_arg(args: &[String]) -> Option<ServerArg> {
    use std::net::ToSocketAddrs;
    let w = args.windows(2).find(|w| w[0] == "--server")?;
    if w[1] == "dev" {
        return Some(ServerArg::dev());
    }
    if w[1] == "prod" {
        return Some(ServerArg::prod());
    }
    let hostname = w[1].clone();
    let addr = w[1]
        .parse()
        .ok()
        .or_else(|| w[1].to_socket_addrs().ok()?.next())?;
    Some(ServerArg {
        addr,
        hostname,
        dev: false,
    })
}

pub fn has_flag(args: &[String], flag: &str) -> bool {
    args.iter().any(|arg| arg == flag)
}

pub fn parse_state_arg(args: &[String]) -> Result<Option<game_state::GameState>, String> {
    let Some((flag, value)) = find_flag_value(args, &["--state", "--screen"])? else {
        return Ok(None);
    };

    match flag {
        "--state" => game_state::GameState::from_str(value)
            .map(Some)
            .map_err(|err| format!("invalid --state value '{value}': {err}")),
        "--screen" => ScreenArg::from_str(value)
            .map(game_state::GameState::from)
            .map(Some)
            .map_err(|err| format!("invalid --screen value '{value}': {err}")),
        _ => unreachable!("unexpected flag matched: {flag}"),
    }
}

pub fn parse_screen_arg(args: &[String]) -> Result<Option<ScreenArg>, String> {
    let Some((flag, value)) = find_flag_value(args, &["--screen"])? else {
        return Ok(None);
    };
    debug_assert_eq!(flag, "--screen");
    ScreenArg::from_str(value)
        .map(Some)
        .map_err(|err| format!("invalid --screen value '{value}': {err}"))
}

pub fn parse_char_arg(args: &[String]) -> Option<String> {
    args.windows(2)
        .find(|w| w[0] == "--char")
        .map(|w| w[1].clone())
}

pub fn parse_load_scene_arg(args: &[String]) -> Result<Option<PathBuf>, String> {
    Ok(find_flag_value(args, &["--load-scene"])?.map(|(_, value)| PathBuf::from(value)))
}

pub fn print_help() {
    println!("game-engine {}", env!("CARGO_PKG_VERSION"));
    println!();
    println!("USAGE: game-engine [OPTIONS] [model.m2 | terrain.adt]");
    println!();
    println!("OPTIONS:");
    println!(
        "  --screen <SCREEN>   Start at screen: login, charselect, charcreate, charcreate-customize, campsitepopup, inworld, gamemenu, optionsmenu, trashbutton"
    );
    println!(
        "  --server <ADDR>     Game server address or alias (`dev`, `prod`); default: {DEFAULT_SERVER_ADDR}"
    );
    println!("  --char <NAME>       Pick character by name (with --screen inworld)");
    println!("  --load-scene <PATH> Load a saved semantic scene snapshot");
    println!("  --login-dev-admin   Connect to dev server as admin/admin");
    println!("  --dump-tree         Dump Bevy entity hierarchy and exit");
    println!("  --dump-ui-tree      Dump UI frame registry and exit");
    println!("  --dump-scene        Dump semantic scene tree and exit");
    println!("  screenshot <OUT>    Capture screenshot to file and exit");
    println!("  --run-js-ui-script <PATH>  Run JS UI automation script");
    println!("  --version           Print version");
    println!("  --help, -h          Show this help");
}

fn find_flag_value<'a>(
    args: &'a [String],
    flags: &[&str],
) -> Result<Option<(&'a str, &'a str)>, String> {
    for i in 0..args.len() {
        let arg = args[i].as_str();
        if flags.contains(&arg) {
            let Some(value) = args.get(i + 1).map(String::as_str) else {
                return Err(format!("missing value for {arg}"));
            };
            if value.starts_with("--") {
                return Err(format!("missing value for {arg}"));
            }
            return Ok(Some((arg, value)));
        }
    }

    Ok(None)
}

pub fn load_startup_automation_actions(
    args: &[String],
) -> Result<Vec<game_engine::ui::automation::UiAutomationAction>, String> {
    let mut actions = Vec::new();
    if let Some(script) = game_engine::ui::automation_script::parse_automation_script_arg(args) {
        actions.extend(game_engine::ui::automation_script::load_automation_script(
            &script.path,
        )?);
    }
    if let Some(script) = game_engine::ui::js_automation::parse_js_automation_arg(args) {
        actions.extend(game_engine::ui::js_automation::load_js_automation_script(
            &script.path,
        )?);
    }
    Ok(actions)
}

pub fn parse_asset_path_from_args(args: &[String]) -> Option<PathBuf> {
    let screenshot_idx = screenshot_arg_index(args);
    let mut i = 0;
    while i < args.len() {
        if screenshot_idx == Some(i) {
            i += 2;
            continue;
        }
        match args[i].as_str() {
            "--server" | "--state" | "--screen" | "--char" | "--load-scene" => {
                i += 2;
            }
            "--login-dev-admin" => {
                i += 1;
            }
            arg if arg.starts_with("--") => {
                i += 1;
            }
            path => return Some(PathBuf::from(path)),
        }
    }
    None
}

pub fn parse_asset_path() -> Option<PathBuf> {
    let args: Vec<String> = std::env::args().skip(1).collect();
    parse_asset_path_from_args(&args)
}
