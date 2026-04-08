mod requests;
#[cfg(test)]
mod tests;

use std::path::PathBuf;

use clap::{Parser, Subcommand};
use game_engine::ipc::{Request, Response, socket_glob};
use peercred_ipc::Client;

use requests::{
    auction_request, collection_request, combat_request, equipment_request,
    export_character_request, export_scene_request, group_request, inventory_request, item_request,
    mail_request, map_request, profession_request, quest_request, reputation_request,
    spell_request, status_request, talent_request, trade_request,
};

#[derive(Parser)]
#[command(
    name = "game-engine-cli",
    about = "Control a running game-engine instance"
)]
struct Cli {
    /// Unix socket path (auto-discovered if omitted)
    #[arg(short, long)]
    socket: Option<PathBuf>,
    /// Output responses as JSON
    #[arg(long)]
    json: bool,
    #[command(subcommand)]
    command: Cmd,
}

#[derive(Subcommand)]
enum Cmd {
    /// Check if the engine is running
    Ping,
    /// Capture a screenshot and save to file
    Screenshot {
        #[arg(default_value = "screenshot.webp")]
        output: PathBuf,
    },
    /// Dump the entity hierarchy
    DumpTree {
        #[arg(short, long)]
        filter: Option<String>,
    },
    /// Dump the UI frame hierarchy
    DumpUiTree {
        #[arg(short, long)]
        filter: Option<String>,
    },
    /// Dump the semantic scene tree
    DumpScene {
        #[arg(short, long)]
        filter: Option<String>,
    },
    /// Auction house operations via the running engine
    Auction {
        #[command(subcommand)]
        command: AuctionCmd,
    },
    /// Mailbox operations via the running engine
    Mail {
        #[command(subcommand)]
        command: MailCmd,
    },
    /// Trade operations via the running engine
    Trade {
        #[command(subcommand)]
        command: TradeCmd,
    },
    /// Talent operations via the running engine
    Talent {
        #[command(subcommand)]
        command: TalentCmd,
    },
    /// Runtime subsystem status via the running engine
    Status {
        #[command(subcommand)]
        command: StatusCmd,
    },
    /// Item information lookups via the running engine
    Item {
        #[command(subcommand)]
        command: ItemCmd,
    },
    /// Inventory and storage inspection
    Inventory {
        #[command(subcommand)]
        command: InventoryCmd,
    },
    /// Quest log views
    Quest {
        #[command(subcommand)]
        command: QuestCmd,
    },
    /// Group roster commands
    Group {
        #[command(subcommand)]
        command: GroupCmd,
    },
    /// Spell commands
    Spell {
        #[command(subcommand)]
        command: SpellCmd,
    },
    /// Combat text views
    Combat {
        #[command(subcommand)]
        command: CombatCmd,
    },
    /// Reputation reports
    Reputation {
        #[command(subcommand)]
        command: ReputationCmd,
    },
    /// Collection reports
    Collection {
        #[command(subcommand)]
        command: CollectionCmd,
    },
    /// Profession reports
    Profession {
        #[command(subcommand)]
        command: ProfessionCmd,
    },
    /// Map and waypoint utilities
    Map {
        #[command(subcommand)]
        command: MapCmd,
    },
    /// Runtime equipment rendering controls
    Equipment {
        #[command(subcommand)]
        command: EquipmentCmd,
    },
    /// Export a character to a JSON file from the running engine
    ExportCharacter {
        output: PathBuf,
        #[arg(long)]
        name: Option<String>,
        #[arg(long)]
        character_id: Option<u64>,
    },
    /// Export the semantic scene tree to a JSON snapshot file from the running engine
    ExportScene { output: PathBuf },
}

#[derive(Subcommand)]
pub(crate) enum AuctionCmd {
    Open,
    Status,
    Browse {
        #[arg(long, default_value = "")]
        text: String,
        #[arg(long, default_value_t = 0)]
        page: u32,
        #[arg(long, default_value_t = 20)]
        page_size: u32,
        #[arg(long)]
        min_level: Option<u16>,
        #[arg(long)]
        max_level: Option<u16>,
        #[arg(long)]
        quality: Option<u8>,
        #[arg(long, default_value = "name")]
        sort: String,
        #[arg(long, default_value = "asc")]
        dir: String,
    },
    Owned,
    Bids,
    Inventory,
    Mailbox,
    ClaimMail {
        #[arg(long)]
        mail_id: u64,
    },
    Create {
        #[arg(long)]
        item_guid: u64,
        #[arg(long)]
        stack: u32,
        #[arg(long)]
        bid: u32,
        #[arg(long)]
        buyout: Option<u32>,
        #[arg(long, default_value = "medium")]
        duration: String,
    },
    Bid {
        #[arg(long)]
        id: u64,
        #[arg(long)]
        amount: u32,
    },
    Buyout {
        #[arg(long)]
        id: u64,
    },
    Cancel {
        #[arg(long)]
        id: u64,
    },
}

#[derive(Subcommand)]
pub(crate) enum MailCmd {
    Status,
    List {
        #[arg(long)]
        character: Option<String>,
        #[arg(long)]
        include_deleted: bool,
    },
    Read {
        #[arg(long)]
        mail_id: u64,
    },
    Send {
        #[arg(long)]
        to: String,
        #[arg(long)]
        from: String,
        #[arg(long)]
        subject: String,
        #[arg(long)]
        body: String,
        #[arg(long, default_value_t = 0)]
        money: u64,
    },
    Claim {
        #[arg(long)]
        mail_id: u64,
    },
    Delete {
        #[arg(long)]
        mail_id: u64,
    },
}

#[derive(Subcommand)]
pub(crate) enum StatusCmd {
    Network,
    Terrain,
    Sound,
    Currencies,
    Reputations,
    CharacterStats,
    Bags,
    GuildVault,
    Warbank,
    EquippedGear,
}

#[derive(Subcommand)]
pub(crate) enum ItemCmd {
    Info {
        #[arg(long)]
        item_id: u32,
    },
}

#[derive(Subcommand)]
pub(crate) enum InventoryCmd {
    List,
    Search {
        #[arg(long, default_value = "")]
        text: String,
    },
    Whereis {
        #[arg(long)]
        item_id: u32,
    },
}

#[derive(Subcommand)]
pub(crate) enum QuestCmd {
    List,
    Watch,
    Show {
        #[arg(long)]
        id: u32,
    },
}

#[derive(Subcommand)]
pub(crate) enum GroupCmd {
    Roster,
    Status,
    Invite {
        #[arg(long)]
        name: String,
    },
    Uninvite {
        #[arg(long)]
        name: String,
    },
}

#[derive(Subcommand)]
pub(crate) enum TradeCmd {
    Status,
    Initiate {
        #[arg(long)]
        name: String,
    },
    Accept,
    Decline,
    Cancel,
    SetItem {
        #[arg(long)]
        slot: u8,
        #[arg(long)]
        item_guid: u64,
        #[arg(long)]
        stack: u16,
    },
    ClearItem {
        #[arg(long)]
        slot: u8,
    },
    SetMoney {
        #[arg(long)]
        copper: u32,
    },
    Confirm,
}

#[derive(Subcommand)]
pub(crate) enum TalentCmd {
    Status,
    Apply {
        #[arg(long)]
        talent_id: u32,
    },
    Reset,
}

#[derive(Subcommand)]
pub(crate) enum SpellCmd {
    Cast {
        #[arg(long)]
        spell: String,
        #[arg(long)]
        target: Option<String>,
    },
    Stop,
}

#[derive(Subcommand)]
pub(crate) enum CombatCmd {
    Log {
        #[arg(long, default_value_t = 30)]
        lines: u16,
    },
    Recap {
        #[arg(long)]
        target: Option<String>,
    },
}

#[derive(Subcommand)]
pub(crate) enum ReputationCmd {
    List,
}

#[derive(Subcommand)]
pub(crate) enum CollectionCmd {
    Mounts {
        #[arg(long)]
        missing: bool,
    },
    Pets {
        #[arg(long)]
        missing: bool,
    },
}

#[derive(Subcommand)]
pub(crate) enum ProfessionCmd {
    Recipes {
        #[arg(long, default_value = "")]
        text: String,
    },
}

#[derive(Subcommand)]
pub(crate) enum MapCmd {
    Position,
    Target,
    Waypoint {
        #[command(subcommand)]
        command: WaypointCmd,
    },
}

#[derive(Subcommand)]
pub(crate) enum WaypointCmd {
    Add {
        #[arg(long)]
        x: f32,
        #[arg(long)]
        y: f32,
    },
    Clear,
}

#[derive(Subcommand)]
pub(crate) enum EquipmentCmd {
    /// Set an equipped model path for a slot
    Set {
        #[arg(long)]
        slot: String,
        #[arg(long)]
        model: PathBuf,
    },
    /// Clear a slot so its model despawns
    Clear {
        #[arg(long)]
        slot: String,
    },
}

fn find_socket() -> Result<PathBuf, String> {
    let pattern = socket_glob();
    let mut sockets: Vec<PathBuf> = glob::glob(&pattern)
        .map_err(|e| format!("bad glob: {e}"))?
        .filter_map(Result::ok)
        .collect();
    match sockets.len() {
        0 => Err("no running game-engine instance found".into()),
        1 => Ok(sockets.remove(0)),
        n => Err(format!("{n} instances found, specify --socket")),
    }
}

fn resolve_socket(cli_socket: Option<PathBuf>) -> PathBuf {
    match cli_socket {
        Some(s) => s,
        None => match find_socket() {
            Ok(s) => s,
            Err(e) => {
                eprintln!("error: {e}");
                std::process::exit(1);
            }
        },
    }
}

fn dispatch_command(socket: &PathBuf, command: Cmd, json: bool) -> Result<(), String> {
    match command {
        Cmd::Ping => handle_ping(socket, json),
        Cmd::Screenshot { output } => handle_screenshot(socket, &output, json),
        Cmd::DumpTree { filter } => handle_dump_tree(socket, filter, json),
        Cmd::DumpUiTree { filter } => handle_dump_ui_tree(socket, filter, json),
        Cmd::DumpScene { filter } => handle_dump_scene(socket, filter, json),
        Cmd::Auction { command } => handle_auction(socket, command, json),
        Cmd::Mail { command } => handle_mail(socket, command, json),
        Cmd::Trade { command } => handle_trade(socket, command, json),
        Cmd::Talent { command } => handle_talent(socket, command, json),
        Cmd::Status { command } => handle_status(socket, command, json),
        Cmd::Item { command } => handle_item(socket, command, json),
        Cmd::Inventory { command } => handle_inventory(socket, command, json),
        Cmd::Quest { command } => handle_quest(socket, command, json),
        Cmd::Group { command } => handle_group(socket, command, json),
        Cmd::Spell { command } => handle_spell(socket, command, json),
        Cmd::Combat { command } => handle_combat(socket, command, json),
        Cmd::Reputation { command } => handle_reputation(socket, command, json),
        Cmd::Collection { command } => handle_collection(socket, command, json),
        Cmd::Profession { command } => handle_profession(socket, command, json),
        Cmd::Map { command } => handle_map(socket, command, json),
        Cmd::Equipment { command } => handle_equipment(socket, command, json),
        Cmd::ExportCharacter {
            output,
            name,
            character_id,
        } => handle_export_character(socket, output, name, character_id, json),
        Cmd::ExportScene { output } => handle_export_scene(socket, output, json),
    }
}

fn main() {
    let cli = Cli::parse();
    let socket = resolve_socket(cli.socket);
    let result = dispatch_command(&socket, cli.command, cli.json);
    if let Err(e) = result {
        eprintln!("error: {e}");
        std::process::exit(1);
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
    let resp: Response =
        Client::call(socket, &Request::DumpTree { filter }).map_err(|e| format!("{e}"))?;
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

fn handle_dump_ui_tree(socket: &PathBuf, filter: Option<String>, json: bool) -> Result<(), String> {
    let resp: Response =
        Client::call(socket, &Request::DumpUiTree { filter }).map_err(|e| format!("{e}"))?;
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

fn handle_dump_scene(socket: &PathBuf, filter: Option<String>, json: bool) -> Result<(), String> {
    let resp: Response =
        Client::call(socket, &Request::DumpScene { filter }).map_err(|e| format!("{e}"))?;
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

fn handle_status(socket: &PathBuf, command: StatusCmd, json: bool) -> Result<(), String> {
    handle_text_response(socket, status_request(command)?, json)
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

fn handle_spell(socket: &PathBuf, command: SpellCmd, json: bool) -> Result<(), String> {
    handle_text_response(socket, spell_request(command)?, json)
}

fn handle_combat(socket: &PathBuf, command: CombatCmd, json: bool) -> Result<(), String> {
    handle_text_response(socket, combat_request(command)?, json)
}

fn handle_reputation(socket: &PathBuf, command: ReputationCmd, json: bool) -> Result<(), String> {
    handle_text_response(socket, reputation_request(command)?, json)
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
