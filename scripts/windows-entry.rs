//! Player entry executable: runtime and project resolve relative to the downloaded entry.
#![cfg_attr(windows, windows_subsystem = "windows")]

use std::{
    env,
    process::{Command, ExitCode},
};

fn main() -> ExitCode {
    match launch() {
        Ok(code) => ExitCode::from(code as u8),
        Err(error) => {
            eprintln!("World of Osso: {error}");
            ExitCode::FAILURE
        }
    }
}

fn launch() -> std::io::Result<i32> {
    let entry = env::current_exe()?;
    let directory = entry
        .parent()
        .expect("entry executable has a parent directory");
    let status = Command::new(directory.join("runtime/godot.exe"))
        .current_dir(directory)
        .arg("--path")
        .arg(directory.join("godot"))
        .arg("--")
        .args(env::args_os().skip(1))
        .env("GAME_ENGINE_ASSET_MODE", "extracted-only")
        .status()?;
    Ok(status.code().unwrap_or(1))
}
