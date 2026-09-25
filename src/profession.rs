//! Client professions runtime: the owner's learned lines and spells
//! ([`ProfessionStatusSnapshot`], filled from `ProfessionSnapshot` by
//! `networking::messages`), Retail skill-up and learn chat lines, and
//! [`CraftRequest`]s sent as `CraftRecipe` (the ProfessionsFrame Create buttons and
//! the `profession craft` IPC command).

use std::sync::mpsc;

use bevy::prelude::*;
use game_engine::network_runtime::messages::MessageSenders;
use shared::protocol::{CraftRecipe, ProfessionChannel, ProfessionSnapshot};

use crate::ipc::{Request, Response};
use crate::network_events::register_outgoing_handler;
use crate::professions_data::ProfessionCatalog;
use crate::status::ProfessionStatusSnapshot;

/// Cast a known recipe `casts` times (`C_TradeSkillUI.CraftRecipe`).
#[derive(Message, Debug, Clone, Copy, PartialEq, Eq)]
pub struct CraftRequest {
    pub spell_id: u32,
    pub casts: u16,
}

#[derive(Resource, Default)]
pub struct ProfessionRuntimeState {
    pending: Vec<(CraftRequest, Option<mpsc::Sender<Response>>)>,
}

pub struct ProfessionPlugin;

impl Plugin for ProfessionPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<ProfessionRuntimeState>()
            .add_message::<CraftRequest>()
            .add_systems(Update, queue_craft_requests);
        register_outgoing_handler(app, send_craft_requests, |world| {
            !world
                .resource::<ProfessionRuntimeState>()
                .pending
                .is_empty()
        });
    }
}

pub fn queue_ipc_request(
    runtime: &mut ProfessionRuntimeState,
    snapshot: &ProfessionStatusSnapshot,
    request: &Request,
    respond: mpsc::Sender<Response>,
) -> bool {
    match request {
        Request::ProfessionStatus => {
            let _ = respond.send(Response::Text(format_status(
                snapshot,
                crate::professions_data::profession_catalog(),
            )));
            true
        }
        Request::ProfessionCraft { recipe_id, casts } => {
            let request = CraftRequest {
                spell_id: *recipe_id,
                casts: (*casts).max(1),
            };
            runtime.pending.push((request, Some(respond)));
            true
        }
        _ => false,
    }
}

fn queue_craft_requests(
    mut requests: MessageReader<CraftRequest>,
    mut runtime: ResMut<ProfessionRuntimeState>,
) {
    for request in requests.read() {
        runtime.pending.push((*request, None));
    }
}

fn send_craft_requests(
    mut runtime: ResMut<ProfessionRuntimeState>,
    mut senders: MessageSenders<CraftRecipe>,
) {
    for (request, reply) in std::mem::take(&mut runtime.pending) {
        let mut sent = false;
        for mut sender in senders.iter_mut() {
            sender.send::<ProfessionChannel>(CraftRecipe {
                spell_id: request.spell_id,
                casts: request.casts,
            });
            sent = true;
        }
        if let Some(reply) = reply {
            let _ = reply.send(if sent {
                Response::Text(format!("craft {} x{}", request.spell_id, request.casts))
            } else {
                Response::Error("professions are unavailable: not connected".into())
            });
        }
    }
}

pub fn reset_runtime(runtime: &mut ProfessionRuntimeState) {
    *runtime = ProfessionRuntimeState::default();
}

/// Store `snapshot`; returns the Retail chat lines for what changed: new lines
/// (`ERR_SKILL_GAINED_S`), rank increases (`ERR_SKILL_UP_SI`) and new recipes
/// (`ERR_LEARN_RECIPE_S`). The first snapshot after entering the world is silent.
pub fn apply_snapshot(
    status: &mut ProfessionStatusSnapshot,
    snapshot: ProfessionSnapshot,
    catalog: &ProfessionCatalog,
) -> Vec<String> {
    let first = !status.received;
    let old = std::mem::replace(
        status,
        ProfessionStatusSnapshot {
            lines: snapshot.lines,
            spells: snapshot.spells,
            received: true,
        },
    );
    if first {
        return Vec::new();
    }
    let mut messages = Vec::new();
    for line in &status.lines {
        let Some(info) = catalog.line(line.skill_line) else {
            continue;
        };
        match old
            .lines
            .iter()
            .find(|known| known.skill_line == line.skill_line)
        {
            None => messages.push(format!("You have gained the {} skill.", info.name)),
            Some(known) if line.rank > known.rank && info.parent != 0 => messages.push(format!(
                "Your skill in {} has increased to {}.",
                info.name, line.rank
            )),
            Some(_) => {}
        }
    }
    for spell in status
        .spells
        .iter()
        .filter(|spell| !old.spells.contains(spell))
    {
        if let Some(recipe) = catalog.recipe(*spell) {
            messages.push(format!(
                "You have learned how to create a new item: {}.",
                recipe.name
            ));
        }
    }
    messages
}

fn format_status(snapshot: &ProfessionStatusSnapshot, catalog: &ProfessionCatalog) -> String {
    let lines = snapshot
        .lines
        .iter()
        .map(|line| {
            let name = catalog
                .line(line.skill_line)
                .map_or_else(|| line.skill_line.to_string(), |info| info.name.clone());
            format!("{name} {}/{}", line.rank, line.max_rank)
        })
        .collect::<Vec<_>>();
    let recipes = snapshot
        .spells
        .iter()
        .filter(|spell| catalog.recipe(**spell).is_some())
        .count();
    format!(
        "professions: {}\nrecipes: {recipes}",
        if lines.is_empty() {
            "none".to_string()
        } else {
            lines.join(", ")
        }
    )
}

/// `profession recipes --text`: known recipes whose name contains `text`.
pub fn format_recipes(
    snapshot: &ProfessionStatusSnapshot,
    catalog: &ProfessionCatalog,
    text: &str,
) -> String {
    let needle = text.trim().to_ascii_lowercase();
    let recipes: Vec<String> = snapshot
        .spells
        .iter()
        .filter_map(|spell| catalog.recipe(*spell))
        .filter(|recipe| needle.is_empty() || recipe.name.to_ascii_lowercase().contains(&needle))
        .map(|recipe| format!("{} {}", recipe.spell_id, recipe.name))
        .collect();
    format!(
        "recipes text={text}: {}\n{}",
        recipes.len(),
        if recipes.is_empty() {
            "-".to_string()
        } else {
            recipes.join("\n")
        }
    )
}

#[cfg(test)]
#[path = "profession_tests.rs"]
mod tests;
