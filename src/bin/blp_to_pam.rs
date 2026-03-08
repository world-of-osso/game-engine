use std::env;
use std::fs::File;
use std::io::Write;
use std::path::Path;

fn main() -> Result<(), String> {
    let mut args = env::args().skip(1);
    let input = args
        .next()
        .ok_or_else(|| "usage: blp_to_pam <input.blp> <output.pam>".to_string())?;
    let output = args
        .next()
        .ok_or_else(|| "usage: blp_to_pam <input.blp> <output.pam>".to_string())?;
    let (rgba, width, height) = game_engine::asset::blp::load_blp_rgba(Path::new(&input))?;

    let mut file = File::create(&output).map_err(|e| format!("failed to create output: {e}"))?;
    write!(
        file,
        "P7\nWIDTH {}\nHEIGHT {}\nDEPTH 4\nMAXVAL 255\nTUPLTYPE RGB_ALPHA\nENDHDR\n",
        width, height
    )
    .map_err(|e| format!("failed to write header: {e}"))?;
    file.write_all(&rgba)
        .map_err(|e| format!("failed to write pixels: {e}"))?;
    Ok(())
}
