use std::{env, fs, process};

fn main() {
    if let Err(error) = disassemble_files() {
        eprintln!("bls-disasm: {error}");
        process::exit(1);
    }
}

fn disassemble_files() -> Result<(), Box<dyn std::error::Error>> {
    let paths: Vec<_> = env::args_os().skip(1).collect();
    if paths.is_empty() {
        return Err("usage: bls-disasm <program.shex> [...]".into());
    }
    for path in paths {
        let bytes = fs::read(&path)?;
        let program = dxbc::shex::decode(&bytes).map_err(|error| format!("{path:?}: {error}"))?;
        if !program.warnings.is_empty() {
            return Err(format!("{path:?}: {:?}", program.warnings).into());
        }
        fs::write(
            std::path::Path::new(&path).with_extension("asm"),
            program.to_string(),
        )?;
    }
    Ok(())
}
