//! Offline acquisition from a previously authenticated product/build resolution snapshot.
//! Never reads active .build.info/config/root, and never publishes files itself.
use osso_asset_resolver::FrozenArchiveReader;
use std::path::{Path, PathBuf};

fn main() {
    if let Err(error) = extract() {
        eprintln!("Frozen source extraction: {error}");
        std::process::exit(1);
    }
}

fn extract() -> Result<(), String> {
    let args: Vec<_> = std::env::args_os().skip(1).collect();
    if args.len() < 5 {
        return Err("Expected archive-dir frozen-resolution keys-file-or-- out-dir FDID...".into());
    }
    let archives = Path::new(&args[0]);
    let resolution = Path::new(&args[1]);
    let key_file = (args[2] != "-").then(|| Path::new(&args[2]));
    let output = Path::new(&args[3]);
    let reader = FrozenArchiveReader::open(archives, key_file)?;
    let connection = rusqlite::Connection::open_with_flags(
        resolution,
        rusqlite::OpenFlags::SQLITE_OPEN_READ_ONLY,
    )
    .map_err(|error| format!("Frozen resolution {}: {error}", resolution.display()))?;
    std::fs::create_dir_all(output).map_err(|error| error.to_string())?;
    let mut failures = 0;
    for argument in &args[4..] {
        let fdid: u32 = argument
            .to_str()
            .ok_or("Non-UTF8 FDID")?
            .parse()
            .map_err(|error| format!("Invalid FDID: {error}"))?;
        match extract_one(&reader, &connection, output, fdid) {
            Ok(path) => println!("Extracted frozen FDID {fdid} -> {}", path.display()),
            Err(error) => {
                eprintln!("Frozen FDID {fdid}: {error}");
                failures += 1;
            }
        }
    }
    if failures != 0 {
        return Err(format!("{failures} failed acquisitions"));
    }
    Ok(())
}

fn extract_one(
    reader: &FrozenArchiveReader,
    connection: &rusqlite::Connection,
    output: &Path,
    fdid: u32,
) -> Result<PathBuf, String> {
    let key: Vec<u8> = connection
        .query_row(
            "SELECT encoding_key FROM resolution WHERE fdid=?1",
            [fdid],
            |row| row.get(0),
        )
        .map_err(|error| format!("Frozen FDID {fdid} resolution: {error}"))?;
    let key: [u8; 16] = key
        .try_into()
        .map_err(|_| "Invalid frozen encoding key length")?;
    let bytes = reader.read_encoding_key(key)?;
    let path = output.join(format!("{fdid}.dat"));
    std::fs::write(&path, bytes).map_err(|error| format!("{}: {error}", path.display()))?;
    Ok(path)
}
