use std::collections::VecDeque;
use std::sync::mpsc;

use bevy::ecs::system::SystemParam;
use bevy::prelude::*;
use lightyear::prelude::*;
use shared::protocol::{
    CraftProfessionRecipe, GatherProfessionNode, ProfessionChannel, ProfessionSkillSnapshot,
    ProfessionStateUpdate, QueryProfessions,
};

use crate::ipc::{Request, Response};
use crate::status::{
    ProfessionRecipeEntry, ProfessionSkillEntry, ProfessionSkillUpEntry, ProfessionStatusSnapshot,
};

const KNOWN_GATHER_NODES: &[(u32, &str)] = &[(1, "Copper Vein")];

#[derive(Resource, Default)]
pub struct ProfessionRuntimeState {
    pending_actions: VecDeque<Action>,
    pending_replies: VecDeque<mpsc::Sender<Response>>,
    queried_inworld: bool,
}

#[derive(Debug)]
enum Action {
    Craft(u32),
    Gather(u32),
}

pub struct ProfessionPlugin;

impl Plugin for ProfessionPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<ProfessionRuntimeState>();
        app.add_systems(Update, request_professions_on_enter_world);
        app.add_systems(Update, send_pending_actions);
        app.add_systems(Update, receive_profession_updates);
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
            let _ = respond.send(Response::Text(format_status(snapshot)));
            true
        }
        Request::ProfessionCraft { recipe_id } => {
            queue_craft_action(runtime, *recipe_id);
            runtime.pending_replies.push_back(respond);
            true
        }
        Request::ProfessionGather { node_id } => {
            runtime.pending_actions.push_back(Action::Gather(*node_id));
            runtime.pending_replies.push_back(respond);
            true
        }
        _ => false,
    }
}

pub fn queue_craft_action(runtime: &mut ProfessionRuntimeState, recipe_id: u32) {
    runtime.pending_actions.push_back(Action::Craft(recipe_id));
}

pub fn queue_gather_action(runtime: &mut ProfessionRuntimeState, node_id: u32) {
    runtime.pending_actions.push_back(Action::Gather(node_id));
}

fn request_professions_on_enter_world(
    mut runtime: ResMut<ProfessionRuntimeState>,
    snapshot: Res<ProfessionStatusSnapshot>,
    mut senders: Query<&mut MessageSender<QueryProfessions>>,
) {
    if runtime.queried_inworld || !snapshot.skills.is_empty() || !snapshot.recipes.is_empty() {
        return;
    }
    if send_all(&mut senders, QueryProfessions) {
        runtime.queried_inworld = true;
    }
}

#[derive(SystemParam)]
struct ProfessionSenders<'w, 's> {
    craft: Query<'w, 's, &'static mut MessageSender<CraftProfessionRecipe>>,
    gather: Query<'w, 's, &'static mut MessageSender<GatherProfessionNode>>,
}

fn send_pending_actions(
    mut runtime: ResMut<ProfessionRuntimeState>,
    mut senders: ProfessionSenders,
) {
    while let Some(action) = runtime.pending_actions.pop_front() {
        let sent = match action {
            Action::Craft(recipe_id) => {
                send_all(&mut senders.craft, CraftProfessionRecipe { recipe_id })
            }
            Action::Gather(node_id) => {
                send_all(&mut senders.gather, GatherProfessionNode { node_id })
            }
        };
        if !sent && let Some(reply) = runtime.pending_replies.pop_front() {
            let _ = reply.send(Response::Error(
                "professions are unavailable: not connected".into(),
            ));
        }
    }
}

fn send_all<T: Clone + lightyear::prelude::Message>(
    senders: &mut Query<&mut MessageSender<T>>,
    message: T,
) -> bool {
    let mut sent = false;
    for mut sender in senders.iter_mut() {
        sender.send::<ProfessionChannel>(message.clone());
        sent = true;
    }
    sent
}

fn receive_profession_updates(
    mut runtime: ResMut<ProfessionRuntimeState>,
    mut snapshot: ResMut<ProfessionStatusSnapshot>,
    mut receivers: Query<&mut MessageReceiver<ProfessionStateUpdate>>,
) {
    for mut receiver in receivers.iter_mut() {
        for update in receiver.receive() {
            apply_profession_state_update(&mut snapshot, update);
            if let Some(reply) = runtime.pending_replies.pop_front() {
                let response = if let Some(error) = &snapshot.last_error {
                    Response::Error(error.clone())
                } else {
                    Response::Text(format_status(&snapshot))
                };
                let _ = reply.send(response);
            }
        }
    }
}

fn apply_profession_state_update(
    snapshot: &mut ProfessionStatusSnapshot,
    update: ProfessionStateUpdate,
) {
    if let Some(profession_snapshot) = update.snapshot {
        snapshot.skills = profession_snapshot
            .skills
            .into_iter()
            .map(map_skill_snapshot)
            .collect();
        snapshot.recipes = profession_snapshot
            .recipes
            .into_iter()
            .map(|recipe| ProfessionRecipeEntry {
                spell_id: recipe.spell_id,
                profession: recipe.profession,
                name: recipe.name,
                craftable: recipe.craftable,
                cooldown: recipe.cooldown,
            })
            .collect();
    }
    snapshot.last_server_message = update.message;
    snapshot.last_skill_up = update.skill_up.map(map_skill_up_snapshot);
    snapshot.last_error = update.error;
}

fn map_skill_snapshot(skill: ProfessionSkillSnapshot) -> ProfessionSkillEntry {
    ProfessionSkillEntry {
        profession: skill.profession,
        current: skill.current,
        max: skill.max,
    }
}

fn map_skill_up_snapshot(skill: ProfessionSkillSnapshot) -> ProfessionSkillUpEntry {
    ProfessionSkillUpEntry {
        profession: skill.profession,
        current: skill.current,
        max: skill.max,
    }
}

pub fn reset_runtime(runtime: &mut ProfessionRuntimeState) {
    *runtime = ProfessionRuntimeState::default();
}

fn format_status(snapshot: &ProfessionStatusSnapshot) -> String {
    let mut lines = Vec::new();
    lines.push(format!("professions: {}", format_skill_list(snapshot)));
    lines.push(format!("recipes: {}", snapshot.recipes.len()));
    lines.push(format!("gather_nodes: {}", format_known_gather_nodes()));
    push_optional_line(
        &mut lines,
        "message",
        snapshot.last_server_message.as_deref(),
    );
    push_optional_skill_up_line(&mut lines, snapshot.last_skill_up.as_ref());
    push_optional_line(&mut lines, "error", snapshot.last_error.as_deref());
    lines.join("\n")
}

fn format_skill_list(snapshot: &ProfessionStatusSnapshot) -> String {
    let skills = snapshot
        .skills
        .iter()
        .map(format_skill_entry)
        .collect::<Vec<_>>();
    if skills.is_empty() {
        "none".into()
    } else {
        skills.join(", ")
    }
}

fn format_skill_entry(skill: &ProfessionSkillEntry) -> String {
    format!("{} {}/{}", skill.profession, skill.current, skill.max)
}

fn push_optional_skill_up_line(lines: &mut Vec<String>, skill_up: Option<&ProfessionSkillUpEntry>) {
    if let Some(skill_up) = skill_up {
        lines.push(format!(
            "skill_up: {} {}/{}",
            skill_up.profession, skill_up.current, skill_up.max
        ));
    }
}

fn push_optional_line(lines: &mut Vec<String>, label: &str, value: Option<&str>) {
    if let Some(value) = value {
        lines.push(format!("{label}: {value}"));
    }
}

fn format_known_gather_nodes() -> String {
    KNOWN_GATHER_NODES
        .iter()
        .map(|(id, name)| format!("{id}:{name}"))
        .collect::<Vec<_>>()
        .join(", ")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn format_status_reports_profession_skills() {
        let snapshot = ProfessionStatusSnapshot {
            skills: vec![ProfessionSkillEntry {
                profession: "Mining".into(),
                current: 12,
                max: 75,
            }],
            recipes: vec![ProfessionRecipeEntry {
                spell_id: 5001,
                profession: "Blacksmithing".into(),
                name: "Copper Bracers".into(),
                craftable: true,
                cooldown: None,
            }],
            last_server_message: Some("crafted Copper Bracers".into()),
            last_skill_up: Some(ProfessionSkillUpEntry {
                profession: "Blacksmithing".into(),
                current: 13,
                max: 75,
            }),
            last_error: None,
        };

        let text = format_status(&snapshot);

        assert!(text.contains("Mining 12/75"));
        assert!(text.contains("recipes: 1"));
        assert!(text.contains("crafted Copper Bracers"));
        assert!(text.contains("skill_up: Blacksmithing 13/75"));
    }

    #[test]
    fn queue_gather_action_enqueues_node_request() {
        let mut runtime = ProfessionRuntimeState::default();

        queue_gather_action(&mut runtime, 1);

        match runtime.pending_actions.front() {
            Some(Action::Gather(node_id)) => assert_eq!(*node_id, 1),
            other => panic!("expected queued gather action, got {other:?}"),
        }
    }

    #[test]
    fn queue_craft_action_enqueues_recipe_request() {
        let mut runtime = ProfessionRuntimeState::default();

        queue_craft_action(&mut runtime, 5001);

        match runtime.pending_actions.front() {
            Some(Action::Craft(recipe_id)) => assert_eq!(*recipe_id, 5001),
            other => panic!("expected queued craft action, got {other:?}"),
        }
    }
}
