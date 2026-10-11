#[path = "d4_triangle_count/appearance.rs"]
mod appearance;

use std::fs;
use std::path::Path;
use std::process::ExitCode;

const USAGE: &str = "Usage: d4_triangle_count META PAYLOAD";

fn main() -> ExitCode {
    match run() {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("d4_triangle_count: {error}");
            ExitCode::FAILURE
        }
    }
}

fn run() -> Result<(), String> {
    let args: Vec<_> = std::env::args_os().skip(1).collect();
    if args.len() == 1 && args[0] == "--help" {
        println!("{USAGE}");
        return Ok(());
    }
    if args.len() != 2
        || args
            .iter()
            .any(|arg| arg.to_string_lossy().starts_with('-'))
    {
        return Err(format!("expected exactly two positional paths; {USAGE}"));
    }

    let meta_path = Path::new(&args[0]);
    let payload_path = Path::new(&args[1]);
    let meta = read_input(meta_path)?;
    let payload = read_input(payload_path)?;
    let counts = appearance::count_triangles(&meta, &payload).map_err(|error| {
        format!(
            "META '{}' and PAYLOAD '{}': {error}",
            meta_path.display(),
            payload_path.display()
        )
    })?;

    println!("chunk\tlod\tsubobject\tsegment\tflags\thash\tvertices\ttriangles");
    for count in counts {
        println!(
            "{}\t{}\t{}\t{}\t0x{:08X}\t0x{:08X}\t{}\t{}",
            count.chunk,
            count.lod,
            count.subobject,
            count.segment,
            count.flags,
            count.hash,
            count.vertices,
            count.triangles
        );
    }
    eprintln!(
        "Counts are per segment; appearance variants and LODs are not a combined rendered character."
    );
    Ok(())
}

fn read_input(path: &Path) -> Result<Vec<u8>, String> {
    fs::read(path).map_err(|error| format!("cannot read '{}': {error}", path.display()))
}
