//! Bevy plugin that integrates the IPC server with the render pipeline.

use std::path::Path;
use std::sync::mpsc;

use bevy::camera::primitives::Aabb;
use bevy::picking::mesh_picking::ray_cast::MeshRayCast;
use bevy::prelude::*;
use bevy::render::view::screenshot::{Screenshot, ScreenshotCaptured};
use lightyear::prelude::MessageSender;
use lightyear::prelude::client::Connected;

#[cfg(feature = "ipc")]
use super::init;
use super::{Command, Request, Response};
use crate::auction_house::{AuctionHouseState, queue_ipc_request};
use crate::barber_shop::{
    BarberShopRuntimeState, queue_ipc_request as queue_barber_shop_ipc_request,
};
use crate::character_export::{build_export_character_payload, write_export_character_file};
use crate::collection::{
    CollectionRuntimeState, queue_ipc_request as queue_collection_ipc_request,
};
use crate::currency::{CurrencyRuntimeState, queue_ipc_request as queue_currency_ipc_request};
use crate::duel::{DuelClientState, queue_ipc_request_with_snapshot as queue_duel_ipc_request};
use crate::friends::{FriendsRuntimeState, queue_ipc_request as queue_friends_ipc_request};
use crate::ignore_list::{
    IgnoreListRuntimeState, queue_ipc_request as queue_ignore_list_ipc_request,
};
use crate::inspect::{InspectRuntimeState, queue_ipc_request as queue_inspect_ipc_request};
use crate::item_info::lookup_item_info;
use crate::lfg::{LfgRuntimeState, queue_ipc_request as queue_lfg_ipc_request};
use crate::mail::{MailState, queue_ipc_request as queue_mail_ipc_request};
use crate::profession::{
    ProfessionRuntimeState, queue_ipc_request as queue_profession_ipc_request,
};
use crate::pvp::{PvpRuntimeState, queue_ipc_request as queue_pvp_ipc_request};
use crate::status::{
    AchievementsStatusSnapshot, BarberShopStatusSnapshot, CharacterRosterStatusSnapshot,
    CharacterStatsSnapshot, CollectionStatusSnapshot, CombatLogStatusSnapshot,
    CurrenciesStatusSnapshot, DuelStatusSnapshot, EquipmentAppearanceStatusSnapshot,
    EquippedGearStatusSnapshot, FriendsStatusSnapshot, GroupStatusSnapshot,
    GuildVaultStatusSnapshot, IgnoreListStatusSnapshot, LfgStatusSnapshot, MapStatusSnapshot,
    NetworkStatusSnapshot, ProfessionStatusSnapshot, PvpStatusSnapshot, QuestLogStatusSnapshot,
    ReputationsStatusSnapshot, SoundStatusSnapshot, TalentStatusSnapshot, TerrainStatusSnapshot,
    WarbankStatusSnapshot, Waypoint,
};
use crate::talent::{TalentRuntimeState, queue_ipc_request as queue_talent_ipc_request};
use crate::targeting::CurrentTarget;
use crate::trade::{TradeClientState, queue_ipc_request as queue_trade_ipc_request};
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
pub(crate) type TreeQuery<'w, 's> = Query<'w, 's, crate::dump::TreeQueryData<'static>>;

#[derive(bevy::ecs::system::SystemParam)]
struct StatusSnapshotParams<'w> {
    achievements: Res<'w, AchievementsStatusSnapshot>,
    barber_shop: Res<'w, BarberShopStatusSnapshot>,
    network: Res<'w, NetworkStatusSnapshot>,
    terrain: Res<'w, TerrainStatusSnapshot>,
    sound: Res<'w, SoundStatusSnapshot>,
    currencies: Res<'w, CurrenciesStatusSnapshot>,
    reputations: Res<'w, ReputationsStatusSnapshot>,
    character_stats: Res<'w, CharacterStatsSnapshot>,
    guild_vault: Res<'w, GuildVaultStatusSnapshot>,
    warbank: Res<'w, WarbankStatusSnapshot>,
    equipped_gear: Res<'w, EquippedGearStatusSnapshot>,
    equipment_appearance: Res<'w, EquipmentAppearanceStatusSnapshot>,
    quest_log: Res<'w, QuestLogStatusSnapshot>,
    group: Res<'w, GroupStatusSnapshot>,
    combat_log: Res<'w, CombatLogStatusSnapshot>,
    collection: Res<'w, CollectionStatusSnapshot>,
    friends: Res<'w, FriendsStatusSnapshot>,
    ignore_list: Res<'w, IgnoreListStatusSnapshot>,
    lfg: Res<'w, LfgStatusSnapshot>,
    pvp: Res<'w, PvpStatusSnapshot>,
    profession: Res<'w, ProfessionStatusSnapshot>,
    character_roster: Res<'w, CharacterRosterStatusSnapshot>,
    map: ResMut<'w, MapStatusSnapshot>,
}

/// Plain-struct grouping of snapshot references passed into dispatch.
pub(crate) struct DispatchContext<'a> {
    pub achievements_status: &'a AchievementsStatusSnapshot,
    pub barber_shop_status: &'a BarberShopStatusSnapshot,
    pub network_status: &'a NetworkStatusSnapshot,
    pub terrain_status: &'a TerrainStatusSnapshot,
    pub sound_status: &'a SoundStatusSnapshot,
    pub currencies_status: &'a CurrenciesStatusSnapshot,
    pub reputations_status: &'a ReputationsStatusSnapshot,
    pub character_stats: &'a CharacterStatsSnapshot,
    pub guild_vault_status: &'a GuildVaultStatusSnapshot,
    pub warbank_status: &'a WarbankStatusSnapshot,
    pub equipped_gear_status: &'a EquippedGearStatusSnapshot,
    pub equipment_appearance_status: &'a EquipmentAppearanceStatusSnapshot,
    pub quest_status: &'a QuestLogStatusSnapshot,
    pub group_status: &'a GroupStatusSnapshot,
    pub combat_log_status: &'a CombatLogStatusSnapshot,
    pub collection_status: &'a CollectionStatusSnapshot,
    pub friends_status: &'a FriendsStatusSnapshot,
    pub ignore_list_status: &'a IgnoreListStatusSnapshot,
    pub lfg_status: &'a LfgStatusSnapshot,
    pub pvp_status: &'a PvpStatusSnapshot,
    pub profession_status: &'a ProfessionStatusSnapshot,
    pub character_roster: &'a CharacterRosterStatusSnapshot,
    pub map_status: &'a mut MapStatusSnapshot,
    pub current_target: &'a CurrentTarget,
    pub connected: bool,
}

#[derive(bevy::ecs::system::SystemParam)]
struct SceneParams<'w, 's> {
    commands: Commands<'w, 's>,
    tree_query: TreeQuery<'w, 's>,
    parent_query: Query<'w, 's, &'static ChildOf>,
    global_transform_query: Query<'w, 's, &'static GlobalTransform>,
    aabb_query: Query<'w, 's, (Entity, &'static Aabb, &'static GlobalTransform)>,
    camera_query: Query<'w, 's, (&'static Camera, &'static GlobalTransform), With<Camera3d>>,
    ray_cast: MeshRayCast<'w, 's>,
    ui_state: Res<'w, UiState>,
    scene_tree: Option<Res<'w, crate::scene_tree::SceneTree>>,
    transform_query: Query<'w, 's, &'static Transform>,
}

#[derive(bevy::ecs::system::SystemParam)]
struct WorldParams<'w> {
    auction_house: ResMut<'w, AuctionHouseState>,
    barber_shop: ResMut<'w, BarberShopRuntimeState>,
    barber_shop_status: ResMut<'w, BarberShopStatusSnapshot>,
    collection: ResMut<'w, CollectionRuntimeState>,
    friends: ResMut<'w, FriendsRuntimeState>,
    friends_status: Res<'w, FriendsStatusSnapshot>,
    ignore_list: ResMut<'w, IgnoreListRuntimeState>,
    ignore_list_status: Res<'w, IgnoreListStatusSnapshot>,
    lfg: ResMut<'w, LfgRuntimeState>,
    lfg_status: Res<'w, LfgStatusSnapshot>,
    pvp: ResMut<'w, PvpRuntimeState>,
    pvp_status: Res<'w, PvpStatusSnapshot>,
    duel: ResMut<'w, DuelClientState>,
    duel_status: Res<'w, DuelStatusSnapshot>,
    currency: ResMut<'w, CurrencyRuntimeState>,
    currencies_status: Res<'w, CurrenciesStatusSnapshot>,
    profession: ResMut<'w, ProfessionRuntimeState>,
    profession_status: Res<'w, ProfessionStatusSnapshot>,
    inspect: ResMut<'w, InspectRuntimeState>,
    inspect_status: Res<'w, crate::status::InspectStatusSnapshot>,
    trade: ResMut<'w, TradeClientState>,
    talent: ResMut<'w, TalentRuntimeState>,
    talent_status: Res<'w, TalentStatusSnapshot>,
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
        app.init_resource::<EquipmentControlQueue>();
        #[cfg(feature = "ipc")]
        {
            let (receiver, guard) = init();
            app.insert_non_send_resource(receiver)
                .insert_non_send_resource(guard)
                .add_systems(Update, poll_ipc);
        }
    }
}

/// Poll IPC commands each frame and dispatch them.
#[cfg(feature = "ipc")]
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
        achievements_status: &snapshots.achievements,
        barber_shop_status: &snapshots.barber_shop,
        network_status: &snapshots.network,
        terrain_status: &snapshots.terrain,
        sound_status: &snapshots.sound,
        currencies_status: &snapshots.currencies,
        reputations_status: &snapshots.reputations,
        character_stats: &snapshots.character_stats,
        guild_vault_status: &snapshots.guild_vault,
        warbank_status: &snapshots.warbank,
        equipped_gear_status: &snapshots.equipped_gear,
        equipment_appearance_status: &snapshots.equipment_appearance,
        quest_status: &snapshots.quest_log,
        group_status: &snapshots.group,
        combat_log_status: &snapshots.combat_log,
        collection_status: &snapshots.collection,
        friends_status: &snapshots.friends,
        ignore_list_status: &snapshots.ignore_list,
        lfg_status: &snapshots.lfg,
        pvp_status: &snapshots.pvp,
        profession_status: &snapshots.profession,
        character_roster: &snapshots.character_roster,
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
    if queue_barber_shop_ipc_request(
        &mut world.barber_shop,
        &mut world.barber_shop_status,
        &cmd.request,
        cmd.respond.clone(),
    ) {
        return;
    }
    if queue_collection_ipc_request(&mut world.collection, &cmd.request, cmd.respond.clone()) {
        return;
    }
    if queue_friends_ipc_request(
        &mut world.friends,
        &world.friends_status,
        &cmd.request,
        cmd.respond.clone(),
    ) {
        return;
    }
    if queue_ignore_list_ipc_request(
        &mut world.ignore_list,
        &world.ignore_list_status,
        &cmd.request,
        cmd.respond.clone(),
    ) {
        return;
    }
    if queue_lfg_ipc_request(
        &mut world.lfg,
        &world.lfg_status,
        &cmd.request,
        cmd.respond.clone(),
    ) {
        return;
    }
    if queue_pvp_ipc_request(
        &mut world.pvp,
        &world.pvp_status,
        &cmd.request,
        cmd.respond.clone(),
    ) {
        return;
    }
    if queue_profession_ipc_request(
        &mut world.profession,
        &world.profession_status,
        &cmd.request,
        cmd.respond.clone(),
    ) {
        return;
    }
    if queue_currency_ipc_request(
        &mut world.currency,
        &world.currencies_status,
        &cmd.request,
        cmd.respond.clone(),
    ) {
        return;
    }
    if queue_duel_ipc_request(
        &mut world.duel,
        &world.duel_status,
        ctx.current_target,
        &cmd.request,
        cmd.respond.clone(),
    ) {
        return;
    }
    if queue_inspect_ipc_request(
        &mut world.inspect,
        &world.inspect_status,
        &cmd.request,
        cmd.respond.clone(),
    ) {
        return;
    }
    if queue_trade_ipc_request(&mut world.trade, &cmd.request, cmd.respond.clone()) {
        return;
    }
    if queue_talent_ipc_request(
        &mut world.talent,
        &world.talent_status,
        &cmd.request,
        cmd.respond.clone(),
    ) {
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
        Request::Ping => reply_scene_ping(cmd),
        Request::Screenshot => queue_scene_screenshot(cmd, scene),
        Request::DumpTree { filter } => reply_scene_tree_dump(cmd, scene, filter.as_deref()),
        Request::DumpUiTree { filter } => reply_scene_ui_tree_dump(cmd, scene, filter.as_deref()),
        Request::DumpScene { filter: _ } => reply_scene_dump(cmd, scene),
        Request::ExportScene { output_path } => reply_scene_export(cmd, scene, output_path),
        _ => return false,
    }
    true
}

fn reply_scene_ping(cmd: &Command) {
    let _ = cmd.respond.send(Response::Pong);
}

fn queue_scene_screenshot(cmd: &Command, scene: &mut SceneParams) {
    scene
        .commands
        .spawn(Screenshot::primary_window())
        .insert(ScreenshotReply(cmd.respond.clone()))
        .observe(on_screenshot_captured);
}

fn reply_scene_tree_dump(cmd: &Command, scene: &SceneParams, filter: Option<&str>) {
    let tree = crate::dump::build_tree(&scene.tree_query, &scene.parent_query, filter);
    let _ = cmd.respond.send(Response::Tree(tree));
}

fn reply_scene_ui_tree_dump(cmd: &Command, scene: &SceneParams, filter: Option<&str>) {
    let tree = crate::dump::build_ui_tree(&scene.ui_state.registry, filter);
    let _ = cmd.respond.send(Response::Tree(tree));
}

fn reply_scene_dump(cmd: &Command, scene: &mut SceneParams) {
    let text = build_scene_dump_text(scene);
    let _ = cmd.respond.send(Response::Tree(text));
}

fn build_scene_dump_text(scene: &mut SceneParams) -> String {
    match &scene.scene_tree {
        Some(tree) => crate::dump::build_scene_tree(
            tree,
            &scene.transform_query,
            &scene.global_transform_query,
            &scene.parent_query,
            &scene.aabb_query,
            &scene.camera_query,
            &mut scene.ray_cast,
        ),
        None => "(no scene tree)".into(),
    }
}

fn reply_scene_export(cmd: &Command, scene: &SceneParams, output_path: &str) {
    let response = match export_scene(scene, output_path) {
        Ok(message) => Response::Text(message),
        Err(error) => Response::Error(error),
    };
    let _ = cmd.respond.send(response);
}

fn export_scene(scene: &SceneParams, output_path: &str) -> Result<String, String> {
    let tree = scene
        .scene_tree
        .as_ref()
        .ok_or_else(|| "no scene tree available to export".to_string())?;
    let snapshot = crate::scene_tree::snapshot_scene_tree(tree, &scene.transform_query);
    crate::scene_tree::write_scene_snapshot_file(Path::new(output_path), &snapshot)
        .map(|_| format!("scene exported to {output_path}"))
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
        Request::MapPosition => respond_with_map_position(cmd, ctx.map_status),
        Request::MapTarget => respond_with_map_target(cmd, ctx, tree_query),
        Request::MapWaypointAdd { x, y } => handle_waypoint_add(cmd, ctx.map_status, x, y),
        Request::MapWaypointClear => handle_waypoint_clear(cmd, ctx.map_status),
        Request::EquipmentSet { .. } => {
            dispatch_equipment_set_request(cmd, &mut sender_params.equipment_control);
        }
        Request::EquipmentClear { .. } => {
            dispatch_equipment_clear_request(cmd, &mut sender_params.equipment_control);
        }
        Request::ExportCharacter {
            output_path,
            character_name,
            character_id,
        } => {
            handle_export_character(
                cmd.respond,
                &ctx,
                &output_path,
                character_name.as_deref(),
                character_id,
            );
        }
        _ => {}
    }
}

fn respond_with_map_position(cmd: Command, map_status: &MapStatusSnapshot) {
    let _ = cmd
        .respond
        .send(Response::Text(format_map_position(map_status)));
}

fn respond_with_map_target(cmd: Command, ctx: DispatchContext, tree_query: &TreeQuery) {
    let _ = cmd.respond.send(Response::Text(format_map_target(
        ctx.map_status,
        ctx.current_target,
        tree_query,
    )));
}

fn dispatch_equipment_set_request(cmd: Command, equipment_control: &mut EquipmentControlQueue) {
    if let Request::EquipmentSet { slot, model_path } = cmd.request {
        handle_equipment_set(cmd.respond, equipment_control, slot, model_path);
    }
}

fn dispatch_equipment_clear_request(cmd: Command, equipment_control: &mut EquipmentControlQueue) {
    if let Request::EquipmentClear { slot } = cmd.request {
        handle_equipment_clear(cmd.respond, equipment_control, slot);
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

fn handle_export_character(
    respond: std::sync::mpsc::Sender<Response>,
    ctx: &DispatchContext,
    output_path: &str,
    character_name: Option<&str>,
    character_id: Option<u64>,
) {
    let payload = match build_export_character_payload(
        ctx.character_stats,
        ctx.equipped_gear_status,
        ctx.equipment_appearance_status,
        &ctx.character_roster.entries,
        character_name,
        character_id,
    ) {
        Ok(payload) => payload,
        Err(error) => {
            let _ = respond.send(Response::Error(error));
            return;
        }
    };
    let output = Path::new(output_path);
    if let Err(error) = write_export_character_file(output, &payload) {
        let _ = respond.send(Response::Error(error));
        return;
    }
    let _ = respond.send(Response::Text(format!(
        "exported character {} to {}",
        payload.name,
        output.display()
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
