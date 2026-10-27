use rayon::prelude::*;
use serde::Serialize;
use sha2::{Digest, Sha256};
use std::{
    env,
    fs::File,
    io::{self, Read},
    path::Path,
};

const MIB: usize = 1024 * 1024;
const MIN: usize = 256 * 1024;
const MAX: usize = 4 * MIB;

#[derive(Serialize)]
struct Chunk {
    size: usize,
    sha256: String,
}
#[derive(Serialize)]
struct IndexedFile {
    size: u64,
    sha256: String,
    chunks: Vec<Chunk>,
}

fn digest(bytes: &[u8]) -> String {
    format!("{:x}", Sha256::digest(bytes))
}

fn index_file(path: &Path, gear: &[u64; 256]) -> io::Result<IndexedFile> {
    let mut file = File::open(path)?;
    let single = file.metadata()?.len() <= MIB as u64;
    let mut file_hash = Sha256::new();
    let mut current = Vec::with_capacity(MAX);
    let mut chunks = Vec::new();
    let mut rolling = 0u64;
    let mut size = 0u64;
    let mut buffer = [0u8; 64 * 1024];
    loop {
        let count = file.read(&mut buffer)?;
        if count == 0 {
            break;
        }
        file_hash.update(&buffer[..count]);
        size += count as u64;
        if single {
            current.extend_from_slice(&buffer[..count]);
            continue;
        }
        for &byte in &buffer[..count] {
            current.push(byte);
            rolling = rolling.wrapping_shl(1).wrapping_add(gear[byte as usize]);
            if current.len() >= MIN && (rolling & (MIB as u64 - 1) == 0 || current.len() == MAX) {
                chunks.push(Chunk {
                    size: current.len(),
                    sha256: digest(&current),
                });
                current.clear();
                rolling = 0;
            }
        }
    }
    if !current.is_empty() || single {
        chunks.push(Chunk {
            size: current.len(),
            sha256: digest(&current),
        });
    }
    Ok(IndexedFile {
        size,
        sha256: format!("{:x}", file_hash.finalize()),
        chunks,
    })
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let root = env::args_os().nth(1).ok_or("missing source directory")?;
    let paths: Vec<String> = serde_json::from_reader(io::stdin().lock())?;
    let gear = std::array::from_fn(|i| {
        let hash = Sha256::digest([i as u8]);
        u64::from_le_bytes(hash[..8].try_into().unwrap())
    });
    let files: Result<Vec<_>, _> = paths
        .par_iter()
        .map(|path| {
            index_file(&Path::new(&root).join(path), &gear)
                .map_err(|error| format!("cannot index {path}: {error}"))
        })
        .collect();
    serde_json::to_writer(io::stdout().lock(), &files?)?;
    Ok(())
}
