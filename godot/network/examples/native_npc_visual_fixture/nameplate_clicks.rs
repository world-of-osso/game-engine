//! Physical-input routing fixture: visible combat plate, a different current target,
//! and a mesh-only miss. This server only echoes real received SetTarget messages.
use super::*;
use shared::components::{CombatStatus, UnitTarget};

pub(super) fn seed_layout(project: &FixtureProject, address: SocketAddr) -> Result<(), String> {
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
        .insert((CombatStatus(true), UnitTarget(Some(player.to_bits()))));
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
