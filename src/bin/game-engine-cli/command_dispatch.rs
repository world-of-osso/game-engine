use std::path::PathBuf;

use game_engine::ipc::{Request, Response};
use peercred_ipc::Client;

use super::requests::{
    auction_request, barber_request, calendar_request, collection_request, combat_request,
    currency_request, death_request, duel_request, emote_request, equipment_request,
    export_character_request, export_scene_request, friend_request, group_request, guild_request,
    ignore_request, inspect_request, inventory_request, item_request, lfg_request, mail_request,
    map_request, presence_request, profession_request, pvp_request, quest_request,
    reputation_request, spell_request, status_request, talent_request, trade_request, who_request,
};
use super::*;

pub(crate) fn dispatch_command(socket: &PathBuf, command: Cmd, json: bool) -> Result<(), String> {
    if is_scene_export_command(&command) {
        return dispatch_scene_export_command(socket, command, json);
    }
    if is_social_status_command(&command) {
        return dispatch_social_status_command(socket, command, json);
    }
    dispatch_world_action_command(socket, command, json)
}

fn is_scene_export_command(command: &Cmd) -> bool {
    matches!(
        command,
        Cmd::Ping
            | Cmd::Screenshot { .. }
            | Cmd::DumpTree { .. }
            | Cmd::DumpUiTree { .. }
            | Cmd::DumpScene { .. }
            | Cmd::ExportCharacter { .. }
            | Cmd::ExportScene { .. }
    )
}

fn is_social_status_command(command: &Cmd) -> bool {
    matches!(
        command,
        Cmd::Auction { .. }
            | Cmd::Mail { .. }
            | Cmd::Trade { .. }
            | Cmd::Duel { .. }
            | Cmd::Talent { .. }
            | Cmd::Inspect { .. }
            | Cmd::Status { .. }
            | Cmd::Barber { .. }
            | Cmd::Death { .. }
            | Cmd::Currency { .. }
            | Cmd::Item { .. }
            | Cmd::Inventory { .. }
            | Cmd::Quest { .. }
            | Cmd::Group { .. }
            | Cmd::Friend { .. }
            | Cmd::Guild { .. }
            | Cmd::Calendar { .. }
            | Cmd::Who { .. }
            | Cmd::Presence { .. }
            | Cmd::Ignore { .. }
    )
}

fn dispatch_scene_export_command(socket: &PathBuf, command: Cmd, json: bool) -> Result<(), String> {
    match command {
        Cmd::Ping => handle_ping(socket, json),
        Cmd::Screenshot { output } => handle_screenshot(socket, &output, json),
        Cmd::DumpTree { filter } => handle_dump_tree(socket, filter, json),
        Cmd::DumpUiTree { filter } => handle_dump_ui_tree(socket, filter, json),
        Cmd::DumpScene { filter } => handle_dump_scene(socket, filter, json),
        Cmd::ExportCharacter {
            output,
            name,
            character_id,
        } => handle_export_character(socket, output, name, character_id, json),
        Cmd::ExportScene { output } => handle_export_scene(socket, output, json),
        _ => unreachable!("command routed to wrong scene/export dispatcher"),
    }
}

fn dispatch_social_status_command(
    socket: &PathBuf,
    command: Cmd,
    json: bool,
) -> Result<(), String> {
    match command {
        Cmd::Auction { .. }
        | Cmd::Mail { .. }
        | Cmd::Trade { .. }
        | Cmd::Duel { .. }
        | Cmd::Talent { .. }
        | Cmd::Inspect { .. }
        | Cmd::Status { .. }
        | Cmd::Barber { .. }
        | Cmd::Death { .. }
        | Cmd::Currency { .. } => dispatch_runtime_status_command(socket, command, json),
        Cmd::Item { .. }
        | Cmd::Inventory { .. }
        | Cmd::Quest { .. }
        | Cmd::Group { .. }
        | Cmd::Friend { .. }
        | Cmd::Guild { .. }
        | Cmd::Calendar { .. }
        | Cmd::Who { .. }
        | Cmd::Presence { .. }
        | Cmd::Ignore { .. } => dispatch_roster_query_command(socket, command, json),
        _ => unreachable!("command routed to wrong social/status dispatcher"),
    }
}

fn dispatch_runtime_status_command(
    socket: &PathBuf,
    command: Cmd,
    json: bool,
) -> Result<(), String> {
    match command {
        Cmd::Auction { command } => handle_auction(socket, command, json),
        Cmd::Mail { command } => handle_mail(socket, command, json),
        Cmd::Trade { command } => handle_trade(socket, command, json),
        Cmd::Duel { command } => handle_duel(socket, command, json),
        Cmd::Talent { command } => handle_talent(socket, command, json),
        Cmd::Inspect { command } => handle_inspect(socket, command, json),
        Cmd::Status { command } => handle_status(socket, command, json),
        Cmd::Barber { command } => handle_barber(socket, command, json),
        Cmd::Death { command } => handle_death(socket, command, json),
        Cmd::Currency { command } => handle_currency(socket, command, json),
        _ => unreachable!("command routed to wrong runtime/status dispatcher"),
    }
}

fn dispatch_roster_query_command(socket: &PathBuf, command: Cmd, json: bool) -> Result<(), String> {
    match command {
        Cmd::Item { command } => handle_item(socket, command, json),
        Cmd::Inventory { command } => handle_inventory(socket, command, json),
        Cmd::Quest { command } => handle_quest(socket, command, json),
        Cmd::Group { command } => handle_group(socket, command, json),
        Cmd::Friend { command } => handle_friend(socket, command, json),
        Cmd::Guild { command } => handle_guild(socket, command, json),
        Cmd::Calendar { command } => handle_calendar(socket, command, json),
        Cmd::Who { command } => handle_who(socket, command, json),
        Cmd::Presence { command } => handle_presence(socket, command, json),
        Cmd::Ignore { command } => handle_ignore(socket, command, json),
        _ => unreachable!("command routed to wrong roster/query dispatcher"),
    }
}

fn dispatch_world_action_command(socket: &PathBuf, command: Cmd, json: bool) -> Result<(), String> {
    match command {
        Cmd::Lfg { command } => handle_lfg(socket, command, json),
        Cmd::Pvp { command } => handle_pvp(socket, command, json),
        Cmd::Spell { command } => handle_spell(socket, command, json),
        Cmd::Emote { command } => handle_emote(socket, command, json),
        Cmd::Combat { command } => handle_combat(socket, command, json),
        Cmd::Reputation { command } => handle_reputation(socket, command, json),
        Cmd::Collection { command } => handle_collection(socket, command, json),
        Cmd::Profession { command } => handle_profession(socket, command, json),
        Cmd::Map { command } => handle_map(socket, command, json),
        Cmd::Equipment { command } => handle_equipment(socket, command, json),
        _ => unreachable!("command routed to wrong world/action dispatcher"),
    }
}

fn handle_ping(socket: &PathBuf, json: bool) -> Result<(), String> {
    let resp: Response = Client::call(socket, &Request::Ping).map_err(|e| format!("{e}"))?;
    if json {
        return print_json(&resp);
    }
    match resp {
        Response::Pong => {
            println!("pong");
            Ok(())
        }
        other => Err(format!("unexpected response: {other:?}")),
    }
}

fn handle_dump_tree(socket: &PathBuf, filter: Option<String>, json: bool) -> Result<(), String> {
    handle_tree_response(socket, Request::DumpTree { filter }, json)
}

fn handle_dump_ui_tree(socket: &PathBuf, filter: Option<String>, json: bool) -> Result<(), String> {
    handle_tree_response(socket, Request::DumpUiTree { filter }, json)
}

fn handle_dump_scene(socket: &PathBuf, filter: Option<String>, json: bool) -> Result<(), String> {
    handle_tree_response(socket, Request::DumpScene { filter }, json)
}

fn handle_tree_response(socket: &PathBuf, request: Request, json: bool) -> Result<(), String> {
    let resp: Response = Client::call(socket, &request).map_err(|e| format!("{e}"))?;
    if json {
        return print_json(&resp);
    }
    match resp {
        Response::Tree(tree) => {
            println!("{tree}");
            Ok(())
        }
        Response::Error(msg) => Err(msg),
        other => Err(format!("unexpected response: {other:?}")),
    }
}

fn handle_screenshot(socket: &PathBuf, output: &PathBuf, json: bool) -> Result<(), String> {
    let resp: Response = Client::call(socket, &Request::Screenshot).map_err(|e| format!("{e}"))?;
    match resp {
        Response::Screenshot(data) => {
            std::fs::write(output, &data)
                .map_err(|e| format!("failed to write {}: {e}", output.display()))?;
            if json {
                print_json(&serde_json::json!({
                    "path": output.display().to_string(),
                    "bytes": data.len()
                }))?;
            } else {
                println!("saved {} ({} bytes)", output.display(), data.len());
            }
            Ok(())
        }
        Response::Error(msg) => Err(msg),
        other => Err(format!("unexpected response: {other:?}")),
    }
}

fn handle_auction(socket: &PathBuf, command: AuctionCmd, json: bool) -> Result<(), String> {
    handle_text_response(socket, auction_request(command)?, json)
}

fn handle_mail(socket: &PathBuf, command: MailCmd, json: bool) -> Result<(), String> {
    handle_text_response(socket, mail_request(command)?, json)
}

fn handle_trade(socket: &PathBuf, command: TradeCmd, json: bool) -> Result<(), String> {
    handle_text_response(socket, trade_request(command)?, json)
}

fn handle_talent(socket: &PathBuf, command: TalentCmd, json: bool) -> Result<(), String> {
    handle_text_response(socket, talent_request(command)?, json)
}

fn handle_duel(socket: &PathBuf, command: DuelCmd, json: bool) -> Result<(), String> {
    handle_text_response(socket, duel_request(command)?, json)
}

fn handle_inspect(socket: &PathBuf, command: InspectCmd, json: bool) -> Result<(), String> {
    handle_text_response(socket, inspect_request(command)?, json)
}

fn handle_status(socket: &PathBuf, command: StatusCmd, json: bool) -> Result<(), String> {
    handle_text_response(socket, status_request(command)?, json)
}

fn handle_barber(socket: &PathBuf, command: BarberCmd, json: bool) -> Result<(), String> {
    handle_text_response(socket, barber_request(command)?, json)
}

fn handle_death(socket: &PathBuf, command: DeathCmd, json: bool) -> Result<(), String> {
    handle_text_response(socket, death_request(command)?, json)
}

fn handle_item(socket: &PathBuf, command: ItemCmd, json: bool) -> Result<(), String> {
    handle_text_response(socket, item_request(command)?, json)
}

fn handle_inventory(socket: &PathBuf, command: InventoryCmd, json: bool) -> Result<(), String> {
    handle_text_response(socket, inventory_request(command)?, json)
}

fn handle_quest(socket: &PathBuf, command: QuestCmd, json: bool) -> Result<(), String> {
    handle_text_response(socket, quest_request(command)?, json)
}

fn handle_group(socket: &PathBuf, command: GroupCmd, json: bool) -> Result<(), String> {
    handle_text_response(socket, group_request(command)?, json)
}

fn handle_friend(socket: &PathBuf, command: FriendCmd, json: bool) -> Result<(), String> {
    handle_text_response(socket, friend_request(command)?, json)
}

fn handle_guild(socket: &PathBuf, command: GuildCmd, json: bool) -> Result<(), String> {
    handle_text_response(socket, guild_request(command)?, json)
}

fn handle_calendar(socket: &PathBuf, command: CalendarCmd, json: bool) -> Result<(), String> {
    handle_text_response(socket, calendar_request(command)?, json)
}

fn handle_who(socket: &PathBuf, command: WhoCmd, json: bool) -> Result<(), String> {
    handle_text_response(socket, who_request(command)?, json)
}

fn handle_presence(socket: &PathBuf, command: PresenceCmd, json: bool) -> Result<(), String> {
    handle_text_response(socket, presence_request(command)?, json)
}

fn handle_ignore(socket: &PathBuf, command: IgnoreCmd, json: bool) -> Result<(), String> {
    handle_text_response(socket, ignore_request(command)?, json)
}

fn handle_lfg(socket: &PathBuf, command: LfgCmd, json: bool) -> Result<(), String> {
    handle_text_response(socket, lfg_request(command)?, json)
}

fn handle_pvp(socket: &PathBuf, command: PvpCmd, json: bool) -> Result<(), String> {
    handle_text_response(socket, pvp_request(command)?, json)
}

fn handle_spell(socket: &PathBuf, command: SpellCmd, json: bool) -> Result<(), String> {
    handle_text_response(socket, spell_request(command)?, json)
}

fn handle_emote(socket: &PathBuf, command: EmoteCmd, json: bool) -> Result<(), String> {
    handle_text_response(socket, emote_request(command)?, json)
}

fn handle_combat(socket: &PathBuf, command: CombatCmd, json: bool) -> Result<(), String> {
    handle_text_response(socket, combat_request(command)?, json)
}

fn handle_reputation(socket: &PathBuf, command: ReputationCmd, json: bool) -> Result<(), String> {
    handle_text_response(socket, reputation_request(command)?, json)
}

fn handle_currency(socket: &PathBuf, command: CurrencyCmd, json: bool) -> Result<(), String> {
    handle_text_response(socket, currency_request(command)?, json)
}

fn handle_collection(socket: &PathBuf, command: CollectionCmd, json: bool) -> Result<(), String> {
    handle_text_response(socket, collection_request(command)?, json)
}

fn handle_profession(socket: &PathBuf, command: ProfessionCmd, json: bool) -> Result<(), String> {
    handle_text_response(socket, profession_request(command)?, json)
}

fn handle_map(socket: &PathBuf, command: MapCmd, json: bool) -> Result<(), String> {
    handle_text_response(socket, map_request(command)?, json)
}

fn handle_equipment(socket: &PathBuf, command: EquipmentCmd, json: bool) -> Result<(), String> {
    handle_text_response(socket, equipment_request(command)?, json)
}

fn handle_export_character(
    socket: &PathBuf,
    output: PathBuf,
    name: Option<String>,
    character_id: Option<u64>,
    json: bool,
) -> Result<(), String> {
    handle_text_response(
        socket,
        export_character_request(output, name, character_id),
        json,
    )
}

fn handle_export_scene(socket: &PathBuf, output: PathBuf, json: bool) -> Result<(), String> {
    handle_text_response(socket, export_scene_request(output), json)
}

fn handle_text_response(socket: &PathBuf, request: Request, json: bool) -> Result<(), String> {
    let output = execute_text_request_output(socket, request, json)?;
    println!("{output}");
    Ok(())
}

pub(crate) fn execute_text_request_output(
    socket: &PathBuf,
    request: Request,
    json: bool,
) -> Result<String, String> {
    let resp: Response = Client::call(socket, &request).map_err(|e| format!("{e}"))?;
    format_text_response_output(resp, json)
}

pub(crate) fn print_json<T: serde::Serialize>(value: &T) -> Result<(), String> {
    let serialized = serialize_json(value)?;
    println!("{serialized}");
    Ok(())
}

pub(crate) fn format_text_response_output(resp: Response, json: bool) -> Result<String, String> {
    if json {
        return match resp {
            Response::Error(msg) => Err(msg),
            other => serialize_json(&other),
        };
    }
    match resp {
        Response::Text(text) => Ok(text),
        Response::Error(msg) => Err(msg),
        other => Err(format!("unexpected response: {other:?}")),
    }
}

pub(crate) fn serialize_json<T: serde::Serialize>(value: &T) -> Result<String, String> {
    serde_json::to_string_pretty(value).map_err(|e| format!("failed to encode json: {e}"))
}
