//! Real Forever profiles through the replicated NPC visual path, without synthetic bodies.
use super::*;

pub(super) fn stage_authored_catalogs(project: &FixtureProject) -> Result<(), String> {
    let repo = fixture_support::checkout_root_from_executable("native_npc_visual_fixture")?;
    let source = repo.join("data");
    let data = project.root.join("data");
    for name in [
        "creature_display.sqlite",
        "npc_appearance.sqlite",
        "customization-v4.sqlite",
        "char_texture-v2.sqlite",
        "outfit_links-v3.sqlite",
    ] {
        let target = data.join("cache").join(name);
        if target.exists() {
            fs::remove_file(&target)
                .map_err(|error| format!("Remove {}: {error}", target.display()))?;
        }
        let status = Command::new("sqlite3")
            .args(["-readonly", "-cmd", ".timeout 5000"])
            .arg(source.join("cache").join(name))
            .arg(format!(".backup '{}'", target.display()))
            .status()
            .map_err(|error| format!("Snapshot {name}: {error}"))?;
        if !status.success() {
            return Err(format!("Snapshot {name} exited {status}"));
        }
    }
    link_authored_csvs(&source, &data)
}

fn link_authored_csvs(source: &Path, data: &Path) -> Result<(), String> {
    for entry in
        fs::read_dir(source).map_err(|error| format!("Read {}: {error}", source.display()))?
    {
        let entry = entry.map_err(|error| format!("Read authored CSV entry: {error}"))?;
        let name = entry.file_name();
        let text = name.to_string_lossy();
        // Retain the existing fixture's measured terrain and neutral lighting.
        if !text.ends_with(".csv")
            || text.starts_with("Light")
            || text.starts_with("ZoneLight")
            || text == "Map.csv"
        {
            continue;
        }
        let target = data.join(&name);
        if target.symlink_metadata().is_ok() {
            fs::remove_file(&target)
                .map_err(|error| format!("Remove {}: {error}", target.display()))?;
        }
        std::os::unix::fs::symlink(entry.path(), &target)
            .map_err(|error| format!("Link {}: {error}", target.display()))?;
    }
    Ok(())
}

fn spawn_authored_npcs(app: &mut App, existing: Entity) {
    app.world_mut().despawn(existing);
    let ailee = spawn_named_npc(app, 136968, "Ailee Farheart");
    let ventaari = spawn_named_npc(app, 139694, "Ventaari Brightwish");
    app.world_mut()
        .entity_mut(ailee)
        .insert(fixture_position("azeroth", 5.0));
    app.world_mut()
        .entity_mut(ventaari)
        .insert(fixture_position("azeroth", 7.0));
    println!("FIXTURE SKYBORNE_SPAWNED 136968 139694");
}

pub(super) fn run(
    app: &mut App,
    child: &mut Child,
    lines: Receiver<String>,
    readers: Vec<thread::JoinHandle<()>>,
) -> Result<(), String> {
    let (mut player, mut npc) = (None, None);
    let mut spawned = false;
    let mut completed = false;
    let mut readers = Some(readers);
    let deadline = Instant::now() + Duration::from_secs(420);
    while Instant::now() < deadline {
        app.update();
        respond_to_login(app)?;
        respond_to_selection(app, &mut player, &mut npc)?;
        if !spawned && let Some(existing) = npc {
            spawn_authored_npcs(app, existing);
            spawned = true;
        }
        let status = child.try_wait().map_err(|error| error.to_string())?;
        if status.is_some() {
            for reader in readers.take().expect("fixture readers") {
                reader.join().map_err(|_| "Godot reader panicked")?;
            }
        }
        for line in lines.try_iter() {
            if line.contains("SCRIPT ERROR") {
                return Err(format!("Native Skyborne script: {line}"));
            }
            if line.trim() == "FIXTURE SKYBORNE_DONE" {
                completed = true;
            }
        }
        if let Some(status) = status {
            return if status.success() && completed {
                Ok(())
            } else {
                Err(format!(
                    "Skyborne fixture exited {status}; completed: {completed}"
                ))
            };
        }
        thread::sleep(TICK);
    }
    Err("Timed out waiting for native Skyborne fixture".into())
}
