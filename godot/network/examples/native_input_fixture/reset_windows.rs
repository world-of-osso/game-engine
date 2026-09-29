//! Authenticated authored Options reset and canonical config persistence boundary.
use super::*;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn preflight_reports_all_missing_authored_inputs_without_staging() {
        let missing = missing_fixture_inputs(Path::new("/nonexistent/reset-fixture-data"));
        for name in [
            "cache/customization.sqlite",
            "cache/char_texture.sqlite",
            "ChrRaces.csv",
            "equipment_transforms.ron",
            "ChrCustomizationReq.csv",
            "ChrCustomizationReqChoice.csv",
            "ChrRaceXChrModel.csv",
            "ChrModel.csv",
            "ChrCustomizationOption.csv",
            "ChrCustomizationChoice.csv",
            "ChrCustomizationElement.csv",
            "ChrCustomizationMaterial.csv",
            "ChrCustomizationGeoset.csv",
            "CharHairGeosets.csv",
            "ChrCustomizationCategory.csv",
            "ChrModelTextureLayer.csv",
            "CharComponentTextureSections.csv",
            "CharComponentTextureLayouts.csv",
            "CharStartOutfit.csv",
            "ModelFileData.csv",
            "community-listfile.csv",
            "models",
            "textures",
        ] {
            assert!(
                missing.iter().any(|path| path.ends_with(name)),
                "missing {name}"
            );
        }
    }
}

const ASSET_TREES: &[&str] = &["models", "terrain", "textures"];
const CACHE_FILES: &[&str] = &[
    "customization.sqlite",
    "char_texture.sqlite",
    "creature_display.sqlite",
    "npc_appearance.sqlite",
];
const DATA_DIRS: &[&str] = &["glues", "fonts", "ui", "db2", "dbfilesclient"];
const DATA_FILES: &[&str] = &[
    "AreaTable.csv",
    "Light.csv",
    "LightData.csv",
    "WarbandScene.csv",
    "WarbandScenePlacement.csv",
    "WarbandScenePlacementOption.csv",
    "music_zone_links.csv",
    "music_manifest.csv",
    "community-listfile.csv",
    "CharStartOutfit.csv",
    "ItemModifiedAppearance.csv",
    "ItemAppearance.csv",
    "ItemDisplayInfo.csv",
    "TextureFileData.csv",
    "ItemDisplayInfoMaterialRes.csv",
    "ModelFileData.csv",
    "ChrRaces.csv",
    "equipment_transforms.ron",
    "ChrCustomizationReq.csv",
    "ChrCustomizationReqChoice.csv",
    "ChrRaceXChrModel.csv",
    "ChrModel.csv",
    "ChrCustomizationOption.csv",
    "ChrCustomizationChoice.csv",
    "ChrCustomizationElement.csv",
    "ChrCustomizationMaterial.csv",
    "ChrCustomizationGeoset.csv",
    "CharHairGeosets.csv",
    "ChrCustomizationCategory.csv",
    "ChrModelTextureLayer.csv",
    "CharComponentTextureSections.csv",
    "CharComponentTextureLayouts.csv",
];
const FOOTSTEP_IDS: &[u32] = &[540120, 540121, 540127, 540202];

fn locate_canonical_data_from_git(repo: &Path) -> Result<PathBuf, String> {
    let output = Command::new("git")
        .args([
            "-C",
            repo.to_str().ok_or("Non-UTF-8 checkout path")?,
            "rev-parse",
            "--path-format=absolute",
            "--git-common-dir",
        ])
        .output()
        .map_err(|error| format!("Locate canonical Git directory: {error}"))?;
    if !output.status.success() {
        return Err(format!(
            "Locate canonical Git directory: {}",
            String::from_utf8_lossy(&output.stderr).trim()
        ));
    }
    let path = String::from_utf8(output.stdout)
        .map_err(|error| format!("Decode canonical Git directory: {error}"))?;
    let common_dir = Path::new(path.trim());
    let checkout = common_dir.parent().ok_or_else(|| {
        format!(
            "Git common directory has no checkout: {}",
            common_dir.display()
        )
    })?;
    Ok(checkout.join("data"))
}

fn missing_fixture_inputs(source: &Path) -> Vec<PathBuf> {
    let paths = ASSET_TREES
        .iter()
        .chain(DATA_DIRS)
        .chain(DATA_FILES)
        .map(|name| source.join(name))
        .chain(
            CACHE_FILES
                .iter()
                .map(|name| source.join("cache").join(name)),
        )
        .chain(
            FOOTSTEP_IDS
                .iter()
                .map(|id| source.join("sounds/footsteps").join(format!("{id}.ogg"))),
        );
    paths.filter(|path| !path.exists()).collect()
}

pub(super) struct FixtureProject {
    root: PathBuf,
    pub(super) project: PathBuf,
}

impl FixtureProject {
    pub(super) fn create(repo: &Path, stage_listfile_cache: bool) -> Result<Self, String> {
        let source = repo.join("godot");
        let authored_data = locate_canonical_data_from_git(repo)?;
        let missing = missing_fixture_inputs(&authored_data);
        if !missing.is_empty() {
            return Err(format!(
                "Missing reset fixture inputs:\n{}",
                missing
                    .iter()
                    .map(|path| path.display().to_string())
                    .collect::<Vec<_>>()
                    .join("\n")
            ));
        }
        let root = repo
            .join("data")
            .join(format!("native-reset-fixture-{}", std::process::id()));
        let project = root.join("godot");
        let data = root.join("data");
        create_fixture_directories(&root, &project, &data)?;
        stage_fixture_data(&authored_data, &data)?;
        if stage_listfile_cache {
            stage_listfile_cache_backup(&authored_data, &data)?;
        }
        stage_fixture_project(repo, &source, &root, &project)?;
        stage_fixture_csv(&data)?;
        Ok(Self { root, project })
    }

    pub(super) fn root_path(&self) -> &Path {
        &self.root
    }
}

fn create_fixture_directories(root: &Path, project: &Path, data: &Path) -> Result<(), String> {
    for folder in [
        project,
        data,
        &data.join("cache"),
        &data.join("models"),
        &data.join("terrain"),
        &data.join("textures"),
        &root.join("user-data"),
    ] {
        fs::create_dir_all(folder)
            .map_err(|error| format!("Create {}: {error}", folder.display()))?;
    }
    Ok(())
}

fn stage_fixture_data(authored_data: &Path, data: &Path) -> Result<(), String> {
    for name in ASSET_TREES {
        stage_asset_tree(&authored_data.join(name), &data.join(name))?;
    }
    let footsteps = data.join("sounds/footsteps");
    fs::create_dir_all(&footsteps)
        .map_err(|error| format!("Create {}: {error}", footsteps.display()))?;
    for id in FOOTSTEP_IDS {
        let name = format!("{id}.ogg");
        let original = authored_data.join("sounds/footsteps").join(&name);
        fs::copy(&original, footsteps.join(name))
            .map_err(|error| format!("Stage {}: {error}", original.display()))?;
    }
    for name in CACHE_FILES {
        let original = authored_data.join("cache").join(name);
        fs::copy(&original, data.join("cache").join(name))
            .map_err(|error| format!("Stage {}: {error}", original.display()))?;
    }
    for name in DATA_DIRS.iter().chain(DATA_FILES) {
        link_required(&authored_data.join(name), &data.join(name))?;
    }
    Ok(())
}

fn stage_listfile_cache_backup(authored_data: &Path, data: &Path) -> Result<(), String> {
    let source = authored_data.join("local-listfile-cache.sqlite");
    if !source.is_file() {
        return Err(format!(
            "Required canonical listfile cache missing: {}",
            source.display()
        ));
    }
    let target = data.join("local-listfile-cache.sqlite");
    let command = format!(".backup '{}'", target.display());
    let output = Command::new("sqlite3")
        .args(["-readonly", "-cmd", ".timeout 5000"])
        .arg(&source)
        .arg(command)
        .output()
        .map_err(|error| format!("Backup {}: {error}", source.display()))?;
    if !output.status.success() {
        return Err(format!(
            "Backup {}: {}",
            source.display(),
            String::from_utf8_lossy(&output.stderr)
        ));
    }
    Ok(())
}

fn stage_fixture_project(
    repo: &Path,
    source: &Path,
    root: &Path,
    project: &Path,
) -> Result<(), String> {
    for name in ["project.godot", "scenes", "shaders", "tests", "ui"] {
        link_required(&source.join(name), &project.join(name))?;
    }
    fs::copy(
        source.join("game_engine.gdextension"),
        project.join("game_engine.gdextension"),
    )
    .map_err(|error| format!("Stage Godot extension manifest: {error}"))?;
    fs::create_dir_all(project.join(".godot"))
        .map_err(|error| format!("Create Godot extension cache: {error}"))?;
    fs::write(
        project.join(".godot/extension_list.cfg"),
        "res://game_engine.gdextension\n",
    )
    .map_err(|error| format!("Register extension manifest: {error}"))?;
    link_required(&repo.join("target"), &root.join("target"))?;
    Ok(())
}

impl Drop for FixtureProject {
    fn drop(&mut self) {
        if let Err(error) = fs::remove_dir_all(&self.root) {
            eprintln!("Remove reset fixture {}: {error}", self.root.display());
        }
    }
}

fn link_required(source: &Path, target: &Path) -> Result<(), String> {
    if !source.exists() {
        return Err(format!(
            "Required reset fixture asset missing: {}",
            source.display()
        ));
    }
    std::os::unix::fs::symlink(source, target)
        .map_err(|error| format!("Link {}: {error}", source.display()))
}

fn stage_asset_tree(source: &Path, target: &Path) -> Result<(), String> {
    for entry in
        fs::read_dir(source).map_err(|error| format!("Read {}: {error}", source.display()))?
    {
        let entry = entry.map_err(|error| format!("Read {} entry: {error}", source.display()))?;
        let destination = target.join(entry.file_name());
        if entry.path().is_dir() {
            fs::create_dir(&destination)
                .map_err(|error| format!("Create {}: {error}", destination.display()))?;
            stage_asset_tree(&entry.path(), &destination)?;
        } else if entry.path().is_file() {
            link_required(&entry.path(), &destination)?;
        }
    }
    Ok(())
}

fn stage_fixture_csv(data: &Path) -> Result<(), String> {
    for (name, contents) in [
        (
            "ZoneLight.csv",
            "ID,MapID,LightID,TransitionType,Zmin,Zmax\n1,99999,1,0,-100,100\n",
        ),
        ("ZoneLightPoint.csv", "ZoneLightID,PointOrder,Pos_0,Pos_1\n"),
    ] {
        fs::write(data.join(name), contents)
            .map_err(|error| format!("Stage reset fixture {name}: {error}"))?;
    }
    Ok(())
}

pub(super) fn run(
    app: &mut App,
    child: &mut Child,
    mut lines: Receiver<String>,
    readers: Vec<thread::JoinHandle<()>>,
    repo: &Path,
    config: &FixtureConfig,
    project: &Path,
    address: SocketAddr,
) -> Result<(), String> {
    let mut selected = None;
    let mut remote = None;
    let mut readers = Some(readers);
    let mut loading = false;
    let mut saved = false;
    let mut reopened = false;
    let mut passed = false;
    let deadline = Instant::now() + TIMEOUT;
    while Instant::now() < deadline {
        app.update();
        respond_to_login(app, StartupScreen::ResetWindows)?;
        respond_to_selection(app, StartupScreen::ResetWindows, &mut selected, &mut remote)?;
        let status = child.try_wait().map_err(|error| error.to_string())?;
        if status.is_some() {
            for reader in readers.take().expect("join output once") {
                reader.join().map_err(|_| "Godot output reader panicked")?;
            }
        }
        for line in lines.try_iter() {
            saved |= line.trim() == "FIXTURE MAP_SAVED";
            reopened |= line.trim() == "FIXTURE MAP_REOPENED";
            observe_reset_line(app, &line, selected.is_some(), &mut loading, &mut passed)?;
        }
        if let Some(status) = status {
            verify_reset_process_exit(status, saved, reopened, passed)?;
            if saved && !reopened {
                discard_map_save_players(app, &mut selected, &mut remote, loading)?;
                loading = false;
                saved = false;
                let (next, receiver, output_readers) = launch_godot(
                    repo,
                    project,
                    config,
                    address,
                    StartupScreen::ResetWindows,
                    true,
                );
                *child = next;
                lines = receiver;
                readers = Some(output_readers);
                continue;
            }
            return verify_completed_reset(
                &config.home,
                project,
                passed,
                reopened,
                loading,
                selected.is_some(),
            );
        }
        thread::sleep(TICK);
    }
    Err("timed out waiting for authenticated Reset Window Positions".into())
}

fn verify_reset_process_exit(
    status: std::process::ExitStatus,
    saved: bool,
    reopened: bool,
    passed: bool,
) -> Result<(), String> {
    if !status.success() {
        return Err(format!(
            "Godot reset fixture exited {status}; saved={saved}, reopened={reopened}, pass={passed}"
        ));
    }
    Ok(())
}

fn discard_map_save_players(
    app: &mut App,
    selected: &mut Option<Entity>,
    remote: &mut Option<Entity>,
    loading: bool,
) -> Result<(), String> {
    if !loading || selected.is_none() {
        return Err("Map-save process did not authenticate and load world".into());
    }
    app.world_mut()
        .despawn(selected.take().expect("saved map player"));
    if let Some(entity) = remote.take() {
        app.world_mut().despawn(entity);
    }
    Ok(())
}

fn verify_completed_reset(
    config: &Path,
    project: &Path,
    passed: bool,
    reopened: bool,
    loading: bool,
    selected: bool,
) -> Result<(), String> {
    if !passed || !reopened || !loading || !selected {
        return Err(format!(
            "Reset verification incomplete: reopened={reopened}, pass={passed}, loading={loading}"
        ));
    }
    verify_saved_layout(config)?;
    verify_fresh_process(config, project)
}

fn observe_reset_line(
    app: &mut App,
    line: &str,
    selected: bool,
    loading: &mut bool,
    passed: &mut bool,
) -> Result<(), String> {
    if line.trim() == "FIXTURE RESET_LOADING" && selected && !*loading {
        send::<_, TerrainChannel>(
            app,
            LoadTerrain {
                map_name: "azeroth".into(),
                initial_tile_y: 32,
                initial_tile_x: 48,
            },
        );
        *loading = true;
    }
    let missing_optional_texture = line.contains("GODOT_STDERR: ERROR: WorldObjects:")
        && line.contains("particle textures missing");
    if !missing_optional_texture
        && (line.contains("GODOT_STDERR: ERROR:") || line.contains("GODOT_STDERR: SCRIPT ERROR:"))
    {
        return Err(format!("Godot reset runtime error: {line}"));
    }
    if line
        .trim()
        .starts_with("PASS: authenticated authored reset")
    {
        *passed = true;
    }
    Ok(())
}

fn verify_saved_layout(config: &Path) -> Result<(), String> {
    let path = config.join("world-of-osso/ui_layout.ron");
    let raw = fs::read_to_string(&path).map_err(|error| error.to_string())?;
    if raw.contains("\"17\"") || !raw.contains("\"18\"") || !raw.contains("active_layout") {
        return Err(format!("Reset file not character-scoped: {raw}"));
    }
    Ok(())
}

fn verify_fresh_process(config: &Path, project: &Path) -> Result<(), String> {
    let binary = std::env::var_os("GODOT_BIN")
        .map(PathBuf::from)
        .unwrap_or_else(|| {
            PathBuf::from(std::env::var_os("HOME").expect("HOME for pinned Godot"))
                .join(".cache/game-engine/godot/4.7.2/Godot_v4.7.2-stable_linux.x86_64")
        });
    let output = Command::new(binary)
        .args(["--headless", "--path"])
        .arg(project)
        .args(["--script", "res://tests/options_reset_windows.gd"])
        .env("XDG_CONFIG_HOME", config)
        .env(
            "XDG_DATA_HOME",
            project
                .parent()
                .expect("isolated project root")
                .join("user-data"),
        )
        .env("GODOT_TEST_RESET_VERIFY", "1")
        .output()
        .map_err(|error| format!("relaunch Reset verification: {error}"))?;
    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);
    if !output.status.success() || !stdout.contains("PASS: fresh process read reset layout") {
        return Err(format!(
            "fresh-process layout read failed: {stdout}\n{stderr}"
        ));
    }
    println!("{stdout}");
    Ok(())
}
