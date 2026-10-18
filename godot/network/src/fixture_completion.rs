//! Fail-closed completion boundary for native GDScript process fixtures.
use std::process::ExitStatus;

pub struct Completion {
    expected: usize,
    markers: usize,
}

impl Completion {
    pub fn new(expected: usize) -> Self {
        Self {
            expected,
            markers: 0,
        }
    }

    /// Observe every line on both streams, including startup output. No error allowlist.
    pub fn observe(&mut self, line: &str) -> Result<(), String> {
        let line = line.strip_prefix("GODOT_STDERR: ").unwrap_or(line).trim();
        if line.contains("SCRIPT ERROR") || line.contains("ERROR:") || line.starts_with("FAIL ") {
            return Err(format!("Godot fixture failure: {line}"));
        }
        if line.starts_with("ALL_CASES_DONE") {
            self.markers += 1;
            let expected = format!("ALL_CASES_DONE {0}/{0}", self.expected);
            if self.markers != 1 || line != expected {
                return Err(format!(
                    "invalid completion marker {line:?}; expected exactly one {expected:?}"
                ));
            }
        }
        Ok(())
    }

    pub fn finish(&self, status: ExitStatus) -> Result<(), String> {
        if !status.success() || self.markers != 1 {
            return Err(format!(
                "fixture exited {status}; completion markers={} expected=1 ({} cases)",
                self.markers, self.expected
            ));
        }
        Ok(())
    }
}

pub fn validate_output(output: &std::process::Output, expected: usize) -> Result<(), String> {
    let mut completion = Completion::new(expected);
    for stream in [&output.stdout, &output.stderr] {
        for line in String::from_utf8_lossy(stream).lines() {
            completion.observe(line)?;
        }
    }
    completion.finish(output.status)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::{
        fs,
        os::unix::process::ExitStatusExt,
        process::{Command, Output},
    };

    #[test]
    fn completion_requires_exit_count_unique_marker_and_clean_streams() {
        let output = |stdout: &str, stderr: &str, code| Output {
            status: ExitStatus::from_raw(code << 8),
            stdout: stdout.as_bytes().to_vec(),
            stderr: stderr.as_bytes().to_vec(),
        };
        assert!(validate_output(&output("ALL_CASES_DONE 2/2\n", "", 0), 2).is_ok());
        for (stdout, stderr, code) in [
            ("PASS legacy\n", "", 0),
            ("ALL_CASES_DONE 1/2\n", "", 0),
            ("ALL_CASES_DONE 2/2\nALL_CASES_DONE 2/2\n", "", 0),
            ("ALL_CASES_DONE 2/2\n", "ALL_CASES_DONE 2/2\n", 0),
            ("ALL_CASES_DONE 2/2\nERROR: stdout failure\n", "", 0),
            ("ALL_CASES_DONE 2/2\n", "SCRIPT ERROR: nested failure\n", 0),
            ("ALL_CASES_DONE 2/2\n", "ERROR: startup failure\n", 0),
            ("ALL_CASES_DONE 2/2\n", "", 1),
        ] {
            assert!(
                validate_output(&output(stdout, stderr, code), 2).is_err(),
                "accepted {stdout:?} {stderr:?} code={code}"
            );
        }
    }

    #[test]
    fn converted_fixtures_parse() {
        let directory =
            std::env::temp_dir().join(format!("gdcontract-parse-{}", std::process::id()));
        fs::create_dir_all(&directory).unwrap();
        fs::write(directory.join("project.godot"), "config_version=5\n").unwrap();
        let tests = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../tests");
        std::os::unix::fs::symlink(tests, directory.join("tests")).unwrap();
        let godot = std::env::var_os("GODOT_BIN").expect("GODOT_BIN required for parser proof");
        for fixture in [
            "campsite_selected_hover",
            "communities_fidelity",
            "dropdown_caption_centering",
            "guild_control_fidelity",
            "guild_member_notes_live",
            "guild_tabard_fidelity",
            "house_access_fidelity",
            "hud_edit_controls_fidelity",
            "loot_item_hitarea",
            "minimap_zoom_fidelity",
            "pvp_ready_placement",
            "ready_check_difficulty",
            "ready_check_portrait",
            "ready_check_title_bounds",
            "registration_flow",
            "world_menu_flow",
            "world_sound_flow",
        ] {
            let output = Command::new(&godot)
                .args(["--headless", "--path"])
                .arg(&directory)
                .args([
                    "--script",
                    &format!("res://tests/{fixture}.gd"),
                    "--check-only",
                ])
                .output()
                .unwrap();
            let stderr = String::from_utf8_lossy(&output.stderr);
            assert!(
                output.status.success() && !stderr.contains("ERROR:"),
                "{fixture}: {stderr}"
            );
        }
        fs::remove_dir_all(directory).unwrap();
    }

    #[test]
    fn nested_awaited_failure_is_not_a_pass() {
        let directory = std::env::temp_dir().join(format!("gdcontract-{}", std::process::id()));
        fs::create_dir_all(&directory).unwrap();
        fs::write(directory.join("project.godot"), "config_version=5\n").unwrap();
        fs::write(
            directory.join("completion.gd"),
            include_str!("../../tests/completion.gd"),
        )
        .unwrap();
        let godot = std::env::var_os("GODOT_BIN").expect("GODOT_BIN required for process proof");
        let scripts = [
            (
                r#"extends SceneTree
func _initialize():
    call_deferred("run")
func nested():
    await process_frame
    assert(false, "deliberate nested failure")
func run():
    await nested()
    print("PASS misleading legacy success")
    print("ALL_CASES_DONE 1/1")
    quit(0)
"#,
                false,
            ),
            (
                r#"extends SceneTree
var completion = preload("res://completion.gd").new(self, 1)
func _initialize():
    call_deferred("run")
func nested() -> bool:
    await process_frame
    return completion.require(false, "deliberate checked nested failure")
func run():
    if not completion.step(await nested(), "nested"):
        return
    completion.case_done()
    completion.finish(true)
"#,
                false,
            ),
            (
                r#"extends SceneTree
var completion = preload("res://completion.gd").new(self, 1)
func _initialize():
    call_deferred("run")
func nested() -> bool:
    await process_frame
    return completion.require(true)
func run():
    if not completion.step(await nested(), "nested"):
        return
    completion.case_done()
    completion.finish(true)
"#,
                true,
            ),
        ];
        for (script, succeeds) in scripts {
            fs::write(directory.join("fixture.gd"), script).unwrap();
            let output = Command::new(&godot)
                .args(["--headless", "--path"])
                .arg(&directory)
                .args(["--script", "fixture.gd", "--quit-after", "120"])
                .output()
                .unwrap();
            let verdict = validate_output(&output, 1);
            assert_eq!(
                verdict.is_ok(),
                succeeds,
                "{verdict:?}\nstdout={}\nstderr={}",
                String::from_utf8_lossy(&output.stdout),
                String::from_utf8_lossy(&output.stderr)
            );
            if script.contains("deliberate checked") {
                assert!(
                    !output.status.success(),
                    "checked nested failure must exit nonzero"
                );
                assert!(!String::from_utf8_lossy(&output.stdout).contains("ALL_CASES_DONE"));
            }
        }
        fs::remove_dir_all(directory).unwrap();
    }
}
