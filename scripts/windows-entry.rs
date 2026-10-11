//! Player entry executable: runtime and project resolve relative to the downloaded entry.
#![cfg_attr(windows, windows_subsystem = "windows")]

use std::{
    env,
    process::{self, Command},
};

fn main() {
    let code = match launch() {
        Ok(code) => code,
        Err(error) => {
            eprintln!("World of Osso: {error}");
            1
        }
    };
    process::exit(code);
}

fn launch() -> std::io::Result<i32> {
    let entry = env::current_exe()?;
    let directory = entry
        .parent()
        .expect("entry executable has a parent directory");
    // CreateProcessW cannot use a current directory beyond MAX_PATH. Both runtime
    // and project are absolute; launching from the volume root supports long installs.
    let working_dir = directory.ancestors().last().expect("entry path is rooted");
    let status = Command::new(directory.join("runtime/godot.exe"))
        .current_dir(working_dir)
        .arg("--path")
        .arg(directory.join("godot"))
        .arg("--")
        .args(env::args_os().skip(1))
        .env("GAME_ENGINE_ASSET_MODE", "extracted-only")
        .status()?;
    Ok(status.code().unwrap_or(1))
}
