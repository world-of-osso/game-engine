use std::path::{Path, PathBuf};

pub fn checkout_root_from_executable(name: &str) -> Result<PathBuf, String> {
    let executable = std::env::current_exe()
        .map_err(|error| format!("Read fixture executable path: {error}"))?;
    checkout_root_for_path(&executable, name)
}

fn checkout_root_for_path(executable: &Path, name: &str) -> Result<PathBuf, String> {
    let invalid_path = || {
        format!(
            "Expected {name} at <checkout>/target/debug/examples/{name}; got {}",
            executable.display()
        )
    };
    if executable.file_name().and_then(|file| file.to_str()) != Some(name) {
        return Err(invalid_path());
    }
    let examples = executable.parent().ok_or_else(invalid_path)?;
    let debug = examples.parent().ok_or_else(invalid_path)?;
    let target = debug.parent().ok_or_else(invalid_path)?;
    if examples.file_name().and_then(|part| part.to_str()) != Some("examples")
        || debug.file_name().and_then(|part| part.to_str()) != Some("debug")
        || target.file_name().and_then(|part| part.to_str()) != Some("target")
    {
        return Err(invalid_path());
    }
    let root = target.parent().ok_or_else(invalid_path)?;
    if !root.join("godot/project.godot").is_file() {
        return Err(format!(
            "Missing checkout Godot project at {}",
            root.display()
        ));
    }
    Ok(root.to_path_buf())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn finds_checkout_from_installed_fixture() {
        let workspace = std::env::temp_dir().join(format!("fixture-path-{}", std::process::id()));
        std::fs::create_dir_all(workspace.join("godot")).unwrap();
        std::fs::write(workspace.join("godot/project.godot"), "config_version=5").unwrap();
        let executable = workspace.join("target/debug/examples/native_input_fixture");
        assert_eq!(
            checkout_root_for_path(&executable, "native_input_fixture").unwrap(),
            workspace
        );
        std::fs::remove_dir_all(workspace).unwrap();
    }

    #[test]
    fn rejects_remote_or_unexpected_executable() {
        let remote =
            Path::new("/src/game-engine-godot-conversion/godot/network/native_input_fixture");
        assert!(checkout_root_for_path(remote, "native_input_fixture").is_err());
        let wrong = Path::new("/checkout/target/debug/examples/other_fixture");
        assert!(checkout_root_for_path(wrong, "native_input_fixture").is_err());
    }
}
