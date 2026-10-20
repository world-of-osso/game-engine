//! Physical-input routing fixture: visible combat plate, a different current target,
//! and a mesh-only miss. This server only echoes real received SetTarget messages.
use super::*;
use shared::components::{CombatStatus, UnitTarget};

const CLICK_DISPLAY_ID: u32 = 2079;
const UNIT_FLAG_NOT_SELECTABLE: u32 = 0x02000000;

fn stage_authored_inputs(project: &FixtureProject) -> Result<(), String> {
    let checkout = fixture_support::checkout_root_from_executable("native_npc_visual_fixture")?;
    let source = checkout.join("data");
    let data = project.root.join("data");
    let active = data.join("db2/active");
    let active_path = fs::canonicalize(&active).map_err(|error| error.to_string())?;
    let private_root = fs::canonicalize(&project.root).map_err(|error| error.to_string())?;
    if !active_path.starts_with(&private_root) {
        return Err("Click-fixture DB2 staging requires a private active directory".into());
    }
    for name in STAGED_RETAIL_CSVS {
        fs::copy(source.join("db2/active").join(name), active.join(name))
            .map_err(|error| format!("Stage authored click-fixture CSV {name}: {error}"))?;
    }
    for name in [
        "customization-v5.sqlite",
        "char_texture-v2.sqlite",
        "creature_display.sqlite",
        "npc_appearance.sqlite",
    ] {
        fixture_data::backup_database(
            &source.join("cache").join(name),
            &data.join("cache").join(name),
        )?;
    }
    stage_asset_receipts(&source.join("cache"), &data.join("cache"))
}

fn stage_asset_receipts(source: &Path, target: &Path) -> Result<(), String> {
    for entry in
        fs::read_dir(source).map_err(|error| format!("List authored asset receipts: {error}"))?
    {
        let entry = entry.map_err(|error| format!("Read authored asset receipt entry: {error}"))?;
        let name = entry.file_name();
        let text = name.to_string_lossy();
        if text.starts_with("model-asset-index") && text.ends_with(".json") {
            fs::copy(entry.path(), target.join(name))
                .map_err(|error| format!("Stage authored asset receipt: {error}"))?;
        }
    }
    Ok(())
}

pub(super) fn seed_layout(project: &FixtureProject, address: SocketAddr) -> Result<(), String> {
    stage_authored_inputs(project)?;
    let skin = std::env::var("GODOT_NAMEPLATE_CLICK_SKIN")
        .map_err(|error| format!("Required click-fixture skin: {error}"))?;
    if !["Modern", "Forever"].contains(&skin.as_str()) {
        return Err(format!("Unknown click-fixture skin {skin}"));
    }
    let base = project.root.join("config/world-of-osso/ui_layout.ron");
    let path = game_engine_core::ui_layout_account::account_layout_path(
        &base,
        &address.to_string(),
        "fixture",
    )?;
    let selected = game_engine_core::ui_layout_data::set_active_layout(&path, 17, &skin)?;
    if selected.name != skin {
        return Err(format!(
            "Fixture selected {}, expected {skin}",
            selected.name
        ));
    }
    println!("FIXTURE NAMEPLATE_CLICK_SKIN {skin} {}", path.display());
    Ok(())
}

pub(super) fn prepare(app: &mut App, player: Entity, npc: Entity) {
    app.world_mut()
        .entity_mut(npc)
        // Authored display 2079 (Conservator Ilthalaine), in the imported coverage catalog.
        .insert((
            CombatStatus(true),
            UnitTarget(Some(player.to_bits())),
            ModelDisplay {
                display_id: CLICK_DISPLAY_ID,
            },
        ));
}

pub(super) fn apply_control(
    app: &mut App,
    line: &str,
    player: Entity,
    npc: Entity,
) -> Result<bool, String> {
    if let Some(json) = line.strip_prefix("FIXTURE NAMEPLATE_CLICK_OVERLAP ") {
        let position: [f32; 3] = serde_json::from_str(json)
            .map_err(|error| format!("Read far plate fixture position: {error}"))?;
        spawn_far_plate(app, player, position);
        return Ok(true);
    }
    let (combat, flags) = match line {
        "FIXTURE NAMEPLATE_CLICK_HIDE" => (false, 0),
        "FIXTURE NAMEPLATE_CLICK_NOT_SELECTABLE" => (true, UNIT_FLAG_NOT_SELECTABLE),
        "FIXTURE NAMEPLATE_CLICK_SHOW" => (true, 0),
        _ => return Ok(false),
    };
    app.world_mut().entity_mut(npc).insert((
        CombatStatus(combat),
        UnitFlags(flags),
        UnitTarget(combat.then_some(player.to_bits())),
    ));
    Ok(true)
}

fn spawn_far_plate(app: &mut App, player: Entity, [x, y, z]: [f32; 3]) {
    let far = spawn_named_npc(app, CLICK_DISPLAY_ID, "Fixture Far Plate");
    app.world_mut().entity_mut(far).insert((
        Position { x, y, z },
        UnitFactionTemplate(14),
        UnitFlags(0),
        Health {
            current: 10.0,
            max: 10.0,
        },
        CombatStatus(true),
        UnitTarget(Some(player.to_bits())),
    ));
}

pub(super) fn echo_targets(app: &mut App, player: Entity) -> Result<(), String> {
    let selected = app.world().resource::<Incoming>().selected_link;
    let targets = std::mem::take(&mut app.world_mut().resource_mut::<Incoming>().targets);
    for (link, request) in targets {
        if Some(link) != selected {
            return Err("Target request came from a different fixture connection".into());
        }
        println!(
            "FIXTURE NAMEPLATE_TARGET_RECEIVED {:?}",
            request.target_entity
        );
        app.world_mut()
            .entity_mut(player)
            .insert(UnitTarget(request.target_entity));
    }
    Ok(())
}
