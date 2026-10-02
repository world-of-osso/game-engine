use std::{
    ops::{Deref, DerefMut},
    os::unix::process::CommandExt,
    path::{Path, PathBuf},
    process::{Child, Command},
};

/// A client process the fixture owns. It leads its own process group, so dropping it
/// (on return, early `?`, or panic unwind) SIGKILLs the whole group: a wrapper such as
/// `sh -c` or cage cannot outlive the fixture or leave the real Godot process behind.
/// The kernel also SIGKILLs the leader if the fixture dies without unwinding (SIGINT).
pub struct FixtureChild(Child);

impl FixtureChild {
    pub fn spawn(command: &mut Command) -> std::io::Result<Self> {
        command.process_group(0);
        // SAFETY: prctl is async-signal-safe and touches no parent state.
        unsafe {
            command.pre_exec(|| {
                if libc::prctl(libc::PR_SET_PDEATHSIG, libc::SIGKILL) == 0 {
                    Ok(())
                } else {
                    Err(std::io::Error::last_os_error())
                }
            });
        }
        command.spawn().map(Self)
    }
}

impl Deref for FixtureChild {
    type Target = Child;

    fn deref(&self) -> &Child {
        &self.0
    }
}

impl DerefMut for FixtureChild {
    fn deref_mut(&mut self) -> &mut Child {
        &mut self.0
    }
}

impl Drop for FixtureChild {
    fn drop(&mut self) {
        let group = self.0.id() as libc::pid_t;
        // Group members outlive an exited leader, so kill the group unconditionally.
        // SAFETY: kill has no memory-safety preconditions.
        if unsafe { libc::kill(-group, libc::SIGKILL) } != 0 {
            let error = std::io::Error::last_os_error();
            if error.raw_os_error() != Some(libc::ESRCH) {
                eprintln!("Kill fixture client group {group}: {error}");
            }
        }
        if let Err(error) = self.0.wait() {
            eprintln!("Reap fixture client {group}: {error}");
        }
    }
}

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

    fn process_gone(pid: libc::pid_t) -> bool {
        let deadline = std::time::Instant::now() + std::time::Duration::from_secs(5);
        while std::time::Instant::now() < deadline {
            // Gone, or a zombie awaiting its (new) parent's reap: no longer running.
            match std::fs::read_to_string(format!("/proc/{pid}/stat")) {
                Err(_) => return true,
                Ok(stat)
                    if stat
                        .rsplit(')')
                        .next()
                        .unwrap()
                        .trim_start()
                        .starts_with('Z') =>
                {
                    return true;
                }
                Ok(_) => std::thread::sleep(std::time::Duration::from_millis(20)),
            }
        }
        false
    }

    #[test]
    fn panic_kills_wrapped_client_group() {
        use std::io::BufRead;
        let pids = std::sync::Mutex::new(Vec::new());
        let unwound = std::panic::catch_unwind(|| {
            // The wrapper's grandchild stands in for Godot under `sh -c` / cage.
            let mut child = FixtureChild::spawn(
                Command::new("sh")
                    .args(["-c", "sleep 300 & echo $!; wait"])
                    .stdout(std::process::Stdio::piped()),
            )
            .unwrap();
            let mut line = String::new();
            std::io::BufReader::new(child.stdout.take().unwrap())
                .read_line(&mut line)
                .unwrap();
            let mut pids = pids.lock().unwrap();
            pids.push(child.id() as libc::pid_t);
            pids.push(line.trim().parse::<libc::pid_t>().unwrap());
            assert!(!process_gone(pids[1]), "grandchild {} not running", pids[1]);
            drop(pids);
            panic!("fixture failure mid-run");
        });
        assert!(unwound.is_err());
        let pids = pids.into_inner().unwrap();
        assert_eq!(pids.len(), 2);
        for pid in pids {
            assert!(
                process_gone(pid),
                "process {pid} survived the fixture panic"
            );
        }
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
