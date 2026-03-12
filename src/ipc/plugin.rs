//! Bevy plugin that integrates the IPC server with the render pipeline.

use std::sync::mpsc;

use bevy::prelude::*;
use bevy::render::view::screenshot::{Screenshot, ScreenshotCaptured};
use lightyear::prelude::MessageSender;
use lightyear::prelude::client::Connected;

use super::{Command, Request, Response, init};
use crate::auction_house::{AuctionHouseState, queue_ipc_request};
use crate::item_info::lookup_item_info;
use crate::mail::{MailState, queue_ipc_request as queue_mail_ipc_request};
use crate::status::{
    CharacterStatsSnapshot, CollectionStatusSnapshot, CombatLogStatusSnapshot,
    CurrenciesStatusSnapshot, EquippedGearStatusSnapshot, GroupStatusSnapshot,
    GuildVaultStatusSnapshot, MapStatusSnapshot, NetworkStatusSnapshot, ProfessionStatusSnapshot,
    QuestLogStatusSnapshot, ReputationsStatusSnapshot, SoundStatusSnapshot, TerrainStatusSnapshot,
    WarbankStatusSnapshot, Waypoint,
};
use crate::targeting::CurrentTarget;
use crate::ui::plugin::UiState;
use shared::protocol::{
    CombatChannel, GroupInviteIntent, GroupUninviteIntent, SpellCastIntent, StopSpellCast,
};

use super::format::{
    build_inventory_entries, format_bags_status, format_inventory_list, format_inventory_search,
    format_inventory_whereis, format_map_position, format_map_target, inventory_search_snapshot,
};

/// Channel sender to reply to an IPC caller waiting for a screenshot.
#[derive(Component)]
struct ScreenshotReply(mpsc::Sender<Response>);

#[derive(Debug, Clone)]
pub enum EquipmentControlCommand {
    Set { slot: String, model_path: String },
    Clear { slot: String },
}

#[derive(Resource, Default, Debug)]
pub struct EquipmentControlQueue {
    pub pending: Vec<EquipmentControlCommand>,
}

/// Type alias for the entity tree query used in dump and map-target operations.
pub(crate) type TreeQuery<'w, 's> = Query<
    'w,
    's,
    (
        Entity,
        Option<&'static Name>,
        Option<&'static Children>,
        Option<&'static Visibility>,
        &'static Transform,
    ),
>;

#[derive(bevy::ecs::system::SystemParam)]
struct StatusSnapshotParams<'w> {
    network: Res<'w, NetworkStatusSnapshot>,
    terrain: Res<'w, TerrainStatusSnapshot>,
    sound: Res<'w, SoundStatusSnapshot>,
    currencies: Res<'w, CurrenciesStatusSnapshot>,
    reputations: Res<'w, ReputationsStatusSnapshot>,
    character_stats: Res<'w, CharacterStatsSnapshot>,
    guild_vault: Res<'w, GuildVaultStatusSnapshot>,
    warbank: Res<'w, WarbankStatusSnapshot>,
    equipped_gear: Res<'w, EquippedGearStatusSnapshot>,
    quest_log: Res<'w, QuestLogStatusSnapshot>,
    group: Res<'w, GroupStatusSnapshot>,
    combat_log: Res<'w, CombatLogStatusSnapshot>,
    collection: Res<'w, CollectionStatusSnapshot>,
    profession: Res<'w, ProfessionStatusSnapshot>,
    map: ResMut<'w, MapStatusSnapshot>,
}

/// Plain-struct grouping of snapshot references passed into dispatch.
pub(crate) struct DispatchContext<'a> {
    pub network_status: &'a NetworkStatusSnapshot,
    pub terrain_status: &'a TerrainStatusSnapshot,
    pub sound_status: &'a SoundStatusSnapshot,
    pub currencies_status: &'a CurrenciesStatusSnapshot,
    pub reputations_status: &'a ReputationsStatusSnapshot,
    pub character_stats: &'a CharacterStatsSnapshot,
    pub guild_vault_status: &'a GuildVaultStatusSnapshot,
    pub warbank_status: &'a WarbankStatusSnapshot,
    pub equipped_gear_status: &'a EquippedGearStatusSnapshot,
    pub quest_status: &'a QuestLogStatusSnapshot,
    pub group_status: &'a GroupStatusSnapshot,
    pub combat_log_status: &'a CombatLogStatusSnapshot,
    pub collection_status: &'a CollectionStatusSnapshot,
    pub profession_status: &'a ProfessionStatusSnapshot,
    pub map_status: &'a mut MapStatusSnapshot,
    pub current_target: &'a CurrentTarget,
    pub connected: bool,
}

#[derive(bevy::ecs::system::SystemParam)]
struct SceneParams<'w, 's> {
    commands: Commands<'w, 's>,
    tree_query: TreeQuery<'w, 's>,
    parent_query: Query<'w, 's, &'static ChildOf>,
    ui_state: Res<'w, UiState>,
    scene_tree: Option<Res<'w, crate::scene_tree::SceneTree>>,
    transform_query: Query<'w, 's, &'static Transform>,
}

#[derive(bevy::ecs::system::SystemParam)]
struct WorldParams<'w> {
    auction_house: ResMut<'w, AuctionHouseState>,
    mail: ResMut<'w, MailState>,
}

#[derive(bevy::ecs::system::SystemParam)]
struct IpcSenderParams<'w, 's> {
    spell_cast_senders: Query<'w, 's, &'static mut MessageSender<SpellCastIntent>>,
    spell_stop_senders: Query<'w, 's, &'static mut MessageSender<StopSpellCast>>,
    group_invite_senders: Query<'w, 's, &'static mut MessageSender<GroupInviteIntent>>,
    group_uninvite_senders: Query<'w, 's, &'static mut MessageSender<GroupUninviteIntent>>,
    equipment_control: ResMut<'w, EquipmentControlQueue>,
    connected_query: Query<'w, 's, Entity, With<Connected>>,
}

pub struct IpcPlugin;

impl Plugin for IpcPlugin {
    fn build(&self, app: &mut App) {
        let (receiver, guard) = init();
        app.insert_non_send_resource(receiver)
            .insert_non_send_resource(guard)
            .init_resource::<EquipmentControlQueue>()
            .add_systems(Update, poll_ipc);
    }
}

/// Poll IPC commands each frame and dispatch them.
fn poll_ipc(
    receiver: NonSend<mpsc::Receiver<Command>>,
    mut scene: SceneParams,
    mut world: WorldParams,
    mut snapshots: StatusSnapshotParams,
    current_target: Res<CurrentTarget>,
    mut sender_params: IpcSenderParams,
) {
    let connected = !sender_params.connected_query.is_empty();
    while let Ok(cmd) = receiver.try_recv() {
        let ctx = build_dispatch_context(&mut snapshots, &current_target, connected);
        dispatch(cmd, &mut scene, &mut world, ctx, &mut sender_params);
    }
}

fn build_dispatch_context<'a>(
    snapshots: &'a mut StatusSnapshotParams,
    current_target: &'a CurrentTarget,
    connected: bool,
) -> DispatchContext<'a> {
    DispatchContext {
        network_status: &snapshots.network,
        terrain_status: &snapshots.terrain,
        sound_status: &snapshots.sound,
        currencies_status: &snapshots.currencies,
        reputations_status: &snapshots.reputations,
        character_stats: &snapshots.character_stats,
        guild_vault_status: &snapshots.guild_vault,
        warbank_status: &snapshots.warbank,
        equipped_gear_status: &snapshots.equipped_gear,
        quest_status: &snapshots.quest_log,
        group_status: &snapshots.group,
        combat_log_status: &snapshots.combat_log,
        collection_status: &snapshots.collection,
        profession_status: &snapshots.profession,
        map_status: snapshots.map.as_mut(),
        current_target,
        connected,
    }
}

fn dispatch(
    cmd: Command,
    scene: &mut SceneParams,
    world: &mut WorldParams,
    ctx: DispatchContext,
    sender_params: &mut IpcSenderParams,
) {
    if queue_ipc_request(&mut world.auction_house, &cmd.request, cmd.respond.clone()) {
        return;
    }
    if queue_mail_ipc_request(world.mail.as_mut(), &cmd.request, cmd.respond.clone()) {
        return;
    }
    if dispatch_scene_request(&cmd, scene) {
        return;
    }
    if super::format::dispatch_status_request(&cmd, &ctx) {
        return;
    }
    if dispatch_inventory_request(&cmd, &mut world.auction_house, &ctx) {
        return;
    }
    if dispatch_combat_request(&cmd, &ctx, sender_params) {
        return;
    }
    dispatch_map_and_equipment_request(cmd, ctx, &scene.tree_query, sender_params);
}

/// Returns true if the request was handled.
fn dispatch_scene_request(cmd: &Command, scene: &mut SceneParams) -> bool {
    match &cmd.request {
        Request::Ping => {
            let _ = cmd.respond.send(Response::Pong);
        }
        Request::Screenshot => {
            scene
                .commands
                .spawn(Screenshot::primary_window())
                .insert(ScreenshotReply(cmd.respond.clone()))
                .observe(on_screenshot_captured);
        }
        Request::DumpTree { filter } => {
            let tree =
                crate::dump::build_tree(&scene.tree_query, &scene.parent_query, filter.as_deref());
            let _ = cmd.respond.send(Response::Tree(tree));
        }
        Request::DumpUiTree { filter } => {
            let tree = crate::dump::build_ui_tree(&scene.ui_state.registry, filter.as_deref());
            let _ = cmd.respond.send(Response::Tree(tree));
        }
        Request::DumpScene { filter: _ } => {
            let text = match &scene.scene_tree {
                Some(tree) => crate::dump::build_scene_tree(tree, &scene.transform_query),
                None => "(no scene tree)".into(),
            };
            let _ = cmd.respond.send(Response::Tree(text));
        }
        _ => return false,
    }
    true
}

/// Returns true if the request was handled.
fn dispatch_inventory_request(
    cmd: &Command,
    auction_house: &mut AuctionHouseState,
    ctx: &DispatchContext,
) -> bool {
    let guild_vault = &ctx.guild_vault_status.entries;
    let warbank = &ctx.warbank_status.entries;
    match &cmd.request {
        Request::BagsStatus => {
            let _ = cmd.respond.send(Response::Text(format_bags_status(
                auction_house.inventory.as_ref(),
            )));
        }
        Request::ItemInfo { query } => {
            dispatch_item_info(cmd, auction_house, query.item_id);
        }
        Request::InventoryList => {
            let entries =
                build_inventory_entries(auction_house.inventory.as_ref(), guild_vault, warbank);
            let _ = cmd
                .respond
                .send(Response::Text(format_inventory_list(&entries)));
        }
        Request::InventorySearch { text } => {
            dispatch_inventory_search(cmd, auction_house, guild_vault, warbank, text);
        }
        Request::InventoryWhereis { item_id } => {
            let entries =
                build_inventory_entries(auction_house.inventory.as_ref(), guild_vault, warbank);
            let _ = cmd
                .respond
                .send(Response::Text(format_inventory_whereis(&entries, *item_id)));
        }
        _ => return false,
    }
    true
}

fn dispatch_inventory_search(
    cmd: &Command,
    auction_house: &AuctionHouseState,
    guild_vault: &[crate::status::StorageItemEntry],
    warbank: &[crate::status::StorageItemEntry],
    text: &str,
) {
    let entries = build_inventory_entries(auction_house.inventory.as_ref(), guild_vault, warbank);
    let snapshot = inventory_search_snapshot(&entries, text);
    let _ = cmd
        .respond
        .send(Response::Text(format_inventory_search(&snapshot, text)));
}

fn dispatch_item_info(cmd: &Command, auction_house: &AuctionHouseState, item_id: u32) {
    match lookup_item_info(item_id) {
        Ok(Some(item)) => {
            let appearance_known = auction_house
                .inventory
                .as_ref()
                .is_some_and(|inv| inv.items.iter().any(|entry| entry.item_id == item.item_id));
            let _ = cmd
                .respond
                .send(Response::Text(super::format::format_item_info(
                    &item,
                    appearance_known,
                )));
        }
        Ok(None) => {
            let _ = cmd
                .respond
                .send(Response::Error(format!("item {item_id} not found")));
        }
        Err(error) => {
            let _ = cmd.respond.send(Response::Error(error));
        }
    }
}

/// Returns true if the request was handled.
fn dispatch_combat_request(
    cmd: &Command,
    ctx: &DispatchContext,
    sender_params: &mut IpcSenderParams,
) -> bool {
    match &cmd.request {
        Request::SpellCast { spell, target } => {
            handle_spell_cast(
                cmd,
                spell.clone(),
                target.clone(),
                ctx.current_target,
                ctx.connected,
                &mut sender_params.spell_cast_senders,
            );
        }
        Request::SpellStop => {
            handle_spell_stop(cmd, ctx.connected, &mut sender_params.spell_stop_senders);
        }
        Request::GroupInvite { name } => {
            handle_group_invite(
                cmd,
                name.clone(),
                ctx.connected,
                &mut sender_params.group_invite_senders,
            );
        }
        Request::GroupUninvite { name } => {
            handle_group_uninvite(
                cmd,
                name.clone(),
                ctx.connected,
                &mut sender_params.group_uninvite_senders,
            );
        }
        _ => return false,
    }
    true
}

fn dispatch_map_and_equipment_request(
    cmd: Command,
    ctx: DispatchContext,
    tree_query: &TreeQuery,
    sender_params: &mut IpcSenderParams,
) {
    match cmd.request {
        Request::MapPosition => {
            let _ = cmd
                .respond
                .send(Response::Text(format_map_position(ctx.map_status)));
        }
        Request::MapTarget => {
            let _ = cmd.respond.send(Response::Text(format_map_target(
                ctx.map_status,
                ctx.current_target,
                tree_query,
            )));
        }
        Request::MapWaypointAdd { x, y } => handle_waypoint_add(cmd, ctx.map_status, x, y),
        Request::MapWaypointClear => handle_waypoint_clear(cmd, ctx.map_status),
        Request::EquipmentSet { .. } => {
            if let Request::EquipmentSet { slot, model_path } = cmd.request {
                handle_equipment_set(
                    cmd.respond,
                    &mut sender_params.equipment_control,
                    slot,
                    model_path,
                );
            }
        }
        Request::EquipmentClear { .. } => {
            if let Request::EquipmentClear { slot } = cmd.request {
                handle_equipment_clear(cmd.respond, &mut sender_params.equipment_control, slot);
            }
        }
        _ => {}
    }
}

fn handle_waypoint_add(cmd: Command, map_status: &mut MapStatusSnapshot, x: f32, y: f32) {
    map_status.waypoint = Some(Waypoint { x, y });
    let _ = cmd
        .respond
        .send(Response::Text(format_map_position(map_status)));
}

fn handle_waypoint_clear(cmd: Command, map_status: &mut MapStatusSnapshot) {
    map_status.waypoint = None;
    let _ = cmd
        .respond
        .send(Response::Text(format_map_position(map_status)));
}

fn handle_equipment_set(
    respond: std::sync::mpsc::Sender<Response>,
    equipment_control: &mut EquipmentControlQueue,
    slot: String,
    model_path: String,
) {
    equipment_control
        .pending
        .push(EquipmentControlCommand::Set {
            slot: slot.clone(),
            model_path: model_path.clone(),
        });
    let _ = respond.send(Response::Text(format!(
        "equipment set queued slot={slot} model={model_path}"
    )));
}

fn handle_equipment_clear(
    respond: std::sync::mpsc::Sender<Response>,
    equipment_control: &mut EquipmentControlQueue,
    slot: String,
) {
    equipment_control
        .pending
        .push(EquipmentControlCommand::Clear { slot: slot.clone() });
    let _ = respond.send(Response::Text(format!(
        "equipment clear queued slot={slot}"
    )));
}

fn resolve_spell_cast_intent(
    cmd: &Command,
    spell: &str,
    target: Option<&str>,
    current_target: &CurrentTarget,
) -> Option<SpellCastIntent> {
    let target_bits = match super::format::resolve_spell_target(target, current_target) {
        Ok(bits) => bits,
        Err(error) => {
            let _ = cmd.respond.send(Response::Error(error));
            return None;
        }
    };
    let (spell_id, spell_token) = match super::format::resolve_spell_identifier(spell) {
        Ok(value) => value,
        Err(error) => {
            let _ = cmd.respond.send(Response::Error(error));
            return None;
        }
    };
    Some(SpellCastIntent {
        spell_id,
        spell: spell_token,
        target_entity: target_bits,
    })
}

fn handle_spell_cast(
    cmd: &Command,
    spell: String,
    target: Option<String>,
    current_target: &CurrentTarget,
    connected: bool,
    senders: &mut Query<&mut MessageSender<SpellCastIntent>>,
) {
    if !connected {
        let _ = cmd.respond.send(Response::Error(
            "spell cast is unavailable: not connected".into(),
        ));
        return;
    }
    let Some(intent) = resolve_spell_cast_intent(cmd, &spell, target.as_deref(), current_target)
    else {
        return;
    };
    if send_combat_message(senders, intent.clone()) {
        let target_text = intent
            .target_entity
            .map(|b| b.to_string())
            .unwrap_or_else(|| "-".into());
        let _ = cmd.respond.send(Response::Text(format!(
            "spell cast submitted spell={} target={target_text}",
            intent.spell
        )));
    } else {
        let _ = cmd.respond.send(Response::Error(
            "spell cast is unavailable: not connected".into(),
        ));
    }
}

fn handle_spell_stop(
    cmd: &Command,
    connected: bool,
    senders: &mut Query<&mut MessageSender<StopSpellCast>>,
) {
    if !connected {
        let _ = cmd.respond.send(Response::Error(
            "spell stop is unavailable: not connected".into(),
        ));
        return;
    }
    if send_combat_message(senders, StopSpellCast) {
        let _ = cmd
            .respond
            .send(Response::Text("spell stop submitted".into()));
    } else {
        let _ = cmd.respond.send(Response::Error(
            "spell stop is unavailable: not connected".into(),
        ));
    }
}

fn handle_group_invite(
    cmd: &Command,
    name: String,
    connected: bool,
    senders: &mut Query<&mut MessageSender<GroupInviteIntent>>,
) {
    if !connected {
        let _ = cmd.respond.send(Response::Error(
            "group invite is unavailable: not connected".into(),
        ));
    } else if send_combat_message(senders, GroupInviteIntent { name: name.clone() }) {
        let _ = cmd
            .respond
            .send(Response::Text(format!("group invite submitted for {name}")));
    } else {
        let _ = cmd
            .respond
            .send(Response::Error("group invite sender unavailable".into()));
    }
}

fn handle_group_uninvite(
    cmd: &Command,
    name: String,
    connected: bool,
    senders: &mut Query<&mut MessageSender<GroupUninviteIntent>>,
) {
    if !connected {
        let _ = cmd.respond.send(Response::Error(
            "group uninvite is unavailable: not connected".into(),
        ));
    } else if send_combat_message(senders, GroupUninviteIntent { name: name.clone() }) {
        let _ = cmd.respond.send(Response::Text(format!(
            "group uninvite submitted for {name}"
        )));
    } else {
        let _ = cmd
            .respond
            .send(Response::Error("group uninvite sender unavailable".into()));
    }
}

fn send_combat_message<T: Clone + lightyear::prelude::Message>(
    senders: &mut Query<&mut MessageSender<T>>,
    message: T,
) -> bool {
    let mut sent = false;
    for mut sender in senders.iter_mut() {
        sender.send::<CombatChannel>(message.clone());
        sent = true;
    }
    sent
}

/// Per-entity observer triggered when this screenshot is captured.
fn on_screenshot_captured(
    trigger: On<ScreenshotCaptured>,
    query: Query<&ScreenshotReply>,
    mut commands: Commands,
) {
    let entity = trigger.event_target();
    let Ok(reply) = query.get(entity) else {
        return;
    };
    let response = encode_screenshot(&trigger.image);
    let _ = reply.0.send(response);
    commands.entity(entity).despawn();
}

fn encode_screenshot(img: &bevy::image::Image) -> Response {
    match crate::screenshot::encode_webp(img, 15.0) {
        Ok(webp_data) => Response::Screenshot(webp_data),
        Err(err) => Response::Error(err),
    }
}
