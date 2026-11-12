//! Real Forever profiles through the replicated NPC visual path, without synthetic bodies.
use super::*;

pub(super) fn stage_authored_catalogs(project: &FixtureProject) -> Result<(), String> {
    let repo = fixture_support::checkout_root_from_executable("native_npc_visual_fixture")?;
    let source = repo.join("data");
    let data = project.root.join("data");
    for name in [
        "creature_display.sqlite",
        "npc_appearance.sqlite",
        "customization-v5.sqlite",
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
    link_authored_csvs(&source, &data)?;
    // Real Forever and Retail assets load through per-asset receipts and the products
    // tree they name; both arrived after this fixture was written.
    for entry in fs::read_dir(source.join("cache"))
        .map_err(|error| format!("List {}: {error}", source.join("cache").display()))?
    {
        let entry = entry.map_err(|error| format!("Read cache entry: {error}"))?;
        let name = entry.file_name();
        let name = name.to_string_lossy();
        if !name.starts_with("model-asset-index") || !name.ends_with(".json") {
            continue;
        }
        let target = data.join("cache").join(&*name);
        if target.symlink_metadata().is_ok() {
            fs::remove_file(&target)
                .map_err(|error| format!("Remove {}: {error}", target.display()))?;
        }
        std::os::unix::fs::symlink(entry.path(), &target)
            .map_err(|error| format!("Link receipt index {name}: {error}"))?;
    }
    // The synthetic player preview mapping names fixture-only ChrModels, which the
    // staged real customization catalogs do not declare; the signed-in fixture
    // character needs the authored race-to-model chain to reveal EnterWorld.
    let links = data.join("db2/active/ChrRaceXChrModel.csv");
    if links.symlink_metadata().is_ok() {
        fs::remove_file(&links).map_err(|error| format!("Remove {}: {error}", links.display()))?;
    }
    std::os::unix::fs::symlink(source.join("db2/active/ChrRaceXChrModel.csv"), &links)
        .map_err(|error| format!("Link authored ChrRaceXChrModel: {error}"))?;
    for name in ["equipment_transforms.ron"] {
        let target = data.join(name);
        if target.symlink_metadata().is_ok() {
            fs::remove_file(&target)
                .map_err(|error| format!("Remove {}: {error}", target.display()))?;
        }
        std::os::unix::fs::symlink(source.join(name), &target)
            .map_err(|error| format!("Link authored {name}: {error}"))?;
    }
    let products = data.join("products");
    if products.symlink_metadata().is_ok() {
        fs::remove_file(&products)
            .map_err(|error| format!("Remove {}: {error}", products.display()))?;
    }
    std::os::unix::fs::symlink(source.join("products"), &products)
        .map_err(|error| format!("Link extracted products: {error}"))
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
    eprintln!("FIXTURE SKYBORNE_SPAWNED 136968 139694");
    if std::env::var_os("SKYBORNE_DORNOGAL").is_some() {
        for (display, name, x) in [
            (117276, "Dornogal Earthen Female", 9.0),
            (117271, "Dornogal Earthen Male", 8.0),
            (121978, "Dornogal Blood Elf", 6.0),
            (2989, "Dornogal Control Guard", 1.0),
        ] {
            let npc = spawn_named_npc(app, display, name);
            app.world_mut()
                .entity_mut(npc)
                .insert(fixture_position("azeroth", x));
        }
        eprintln!("FIXTURE DORNOGAL_SPAWNED 117276 117271 121978");
    }
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
        completed |= read_skyborne_completion(&lines)?;
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

fn read_skyborne_completion(lines: &Receiver<String>) -> Result<bool, String> {
    let mut completed = false;
    for line in lines.try_iter() {
        if line.contains("SCRIPT ERROR") {
            return Err(format!("Native Skyborne script: {line}"));
        }
        if [
            "has no character joint",
            "unresolved textures",
            "missing textures",
        ]
        .iter()
        .any(|error| line.contains(error))
        {
            return Err(format!("Native Skyborne equipment error: {line}"));
        }
        completed |= line.trim() == "FIXTURE SKYBORNE_DONE";
    }
    Ok(completed)
}
