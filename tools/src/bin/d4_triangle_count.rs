#[path = "d4_triangle_count/appearance.rs"]
mod appearance;

use std::fs;
use std::path::{Path, PathBuf};
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
    let Some([meta_path, payload_path]) = read_positional_paths()? else {
        println!("{USAGE}");
        return Ok(());
    };
    let meta = read_input(&meta_path)?;
    let payload = read_input(&payload_path)?;
    let counts = appearance::count_triangles(&meta, &payload).map_err(|error| {
        format!(
            "META '{}' and PAYLOAD '{}': {error}",
            meta_path.display(),
            payload_path.display()
        )
    })?;

    print_triangle_counts_tsv(&counts);
    Ok(())
}

fn read_positional_paths() -> Result<Option<[PathBuf; 2]>, String> {
    let args: Vec<_> = std::env::args_os().skip(1).collect();
    if args.len() == 1 && args[0] == "--help" {
        return Ok(None);
    }
    if args.len() != 2
        || args
            .iter()
            .any(|arg| arg.to_string_lossy().starts_with('-'))
    {
        return Err(format!("expected exactly two positional paths; {USAGE}"));
    }
    Ok(Some([PathBuf::from(&args[0]), PathBuf::from(&args[1])]))
}

fn print_triangle_counts_tsv(counts: &[appearance::MeshCount]) {
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
}

fn read_input(path: &Path) -> Result<Vec<u8>, String> {
    fs::read(path).map_err(|error| format!("cannot read '{}': {error}", path.display()))
}
