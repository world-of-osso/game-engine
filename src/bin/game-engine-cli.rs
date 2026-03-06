use std::path::PathBuf;

use clap::{Parser, Subcommand};
use game_engine::ipc::{Request, Response, socket_glob};
use game_engine::item_info::ItemInfoQuery;
use game_engine::mail::{ClaimMail, DeleteMail, ListMailQuery, ReadMail, SendMail};
use peercred_ipc::Client;
use shared::protocol::{
    AuctionDuration, AuctionSearchQuery, AuctionSortDir, AuctionSortField, BuyoutAuction,
    CancelAuction, ClaimAuctionMail, CreateAuction, PlaceBid,
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

    #[command(subcommand)]
    command: Cmd,
}

#[derive(Subcommand)]
enum Cmd {
    /// Check if the engine is running
    Ping,
    /// Capture a screenshot and save to file
    Screenshot {
        /// Output file path (default: screenshot.webp)
        #[arg(default_value = "screenshot.webp")]
        output: PathBuf,
    },
    /// Dump the entity hierarchy
    DumpTree {
        /// Filter by entity name (case-insensitive substring)
        #[arg(short, long)]
        filter: Option<String>,
    },
    /// Dump the UI frame hierarchy
    DumpUiTree {
        /// Filter by frame name (case-insensitive substring)
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
}

#[derive(Subcommand)]
enum AuctionCmd {
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
enum MailCmd {
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
enum StatusCmd {
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
enum ItemCmd {
    Info {
        #[arg(long)]
        item_id: u32,
    },
}

#[derive(Subcommand)]
enum InventoryCmd {
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
enum QuestCmd {
    List,
    Watch,
    Show {
        #[arg(long)]
        id: u32,
    },
}

#[derive(Subcommand)]
enum GroupCmd {
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
enum SpellCmd {
    Cast {
        #[arg(long)]
        spell: String,
        #[arg(long)]
        target: Option<String>,
    },
    Stop,
}

#[derive(Subcommand)]
enum CombatCmd {
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
enum ReputationCmd {
    List,
}

#[derive(Subcommand)]
enum CollectionCmd {
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
enum ProfessionCmd {
    Recipes {
        #[arg(long, default_value = "")]
        text: String,
    },
}

#[derive(Subcommand)]
enum MapCmd {
    Position,
    Target,
    Waypoint {
        #[command(subcommand)]
        command: WaypointCmd,
    },
}

#[derive(Subcommand)]
enum WaypointCmd {
    Add {
        #[arg(long)]
        x: f32,
        #[arg(long)]
        y: f32,
    },
    Clear,
}

#[derive(Subcommand)]
enum EquipmentCmd {
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

fn main() {
    let cli = Cli::parse();
    let socket = match cli.socket {
        Some(s) => s,
        None => match find_socket() {
            Ok(s) => s,
            Err(e) => {
                eprintln!("error: {e}");
                std::process::exit(1);
            }
        },
    };

    let result = match cli.command {
        Cmd::Ping => handle_ping(&socket),
        Cmd::Screenshot { output } => handle_screenshot(&socket, &output),
        Cmd::DumpTree { filter } => handle_dump_tree(&socket, filter),
        Cmd::DumpUiTree { filter } => handle_dump_ui_tree(&socket, filter),
        Cmd::Auction { command } => handle_auction(&socket, command),
        Cmd::Mail { command } => handle_mail(&socket, command),
        Cmd::Status { command } => handle_status(&socket, command),
        Cmd::Item { command } => handle_item(&socket, command),
        Cmd::Inventory { command } => handle_inventory(&socket, command),
        Cmd::Quest { command } => handle_quest(&socket, command),
        Cmd::Group { command } => handle_group(&socket, command),
        Cmd::Spell { command } => handle_spell(&socket, command),
        Cmd::Combat { command } => handle_combat(&socket, command),
        Cmd::Reputation { command } => handle_reputation(&socket, command),
        Cmd::Collection { command } => handle_collection(&socket, command),
        Cmd::Profession { command } => handle_profession(&socket, command),
        Cmd::Map { command } => handle_map(&socket, command),
        Cmd::Equipment { command } => handle_equipment(&socket, command),
    };

    if let Err(e) = result {
        eprintln!("error: {e}");
        std::process::exit(1);
    }
}

fn handle_ping(socket: &PathBuf) -> Result<(), String> {
    let resp: Response = Client::call(socket, &Request::Ping).map_err(|e| format!("{e}"))?;

    match resp {
        Response::Pong => {
            println!("pong");
            Ok(())
        }
        other => Err(format!("unexpected response: {other:?}")),
    }
}

fn handle_dump_tree(socket: &PathBuf, filter: Option<String>) -> Result<(), String> {
    let resp: Response =
        Client::call(socket, &Request::DumpTree { filter }).map_err(|e| format!("{e}"))?;
    match resp {
        Response::Tree(tree) => {
            println!("{tree}");
            Ok(())
        }
        Response::Error(msg) => Err(msg),
        other => Err(format!("unexpected response: {other:?}")),
    }
}

fn handle_dump_ui_tree(socket: &PathBuf, filter: Option<String>) -> Result<(), String> {
    let resp: Response =
        Client::call(socket, &Request::DumpUiTree { filter }).map_err(|e| format!("{e}"))?;
    match resp {
        Response::Tree(tree) => {
            println!("{tree}");
            Ok(())
        }
        Response::Error(msg) => Err(msg),
        other => Err(format!("unexpected response: {other:?}")),
    }
}

fn handle_screenshot(socket: &PathBuf, output: &PathBuf) -> Result<(), String> {
    let resp: Response = Client::call(socket, &Request::Screenshot).map_err(|e| format!("{e}"))?;

    match resp {
        Response::Screenshot(data) => {
            std::fs::write(output, &data)
                .map_err(|e| format!("failed to write {}: {e}", output.display()))?;
            println!("saved {} ({} bytes)", output.display(), data.len());
            Ok(())
        }
        Response::Error(msg) => Err(msg),
        other => Err(format!("unexpected response: {other:?}")),
    }
}

fn handle_auction(socket: &PathBuf, command: AuctionCmd) -> Result<(), String> {
    let request = match command {
        AuctionCmd::Open => Request::AuctionOpen,
        AuctionCmd::Status => Request::AuctionStatus,
        AuctionCmd::Browse {
            text,
            page,
            page_size,
            min_level,
            max_level,
            quality,
            sort,
            dir,
        } => Request::AuctionBrowse {
            query: AuctionSearchQuery {
                text,
                page,
                page_size,
                min_level,
                max_level,
                quality,
                usable_only: false,
                sort_field: parse_sort_field(&sort)?,
                sort_dir: parse_sort_dir(&dir)?,
            },
        },
        AuctionCmd::Owned => Request::AuctionOwned,
        AuctionCmd::Bids => Request::AuctionBids,
        AuctionCmd::Inventory => Request::AuctionInventory,
        AuctionCmd::Mailbox => Request::AuctionMailbox,
        AuctionCmd::ClaimMail { mail_id } => Request::AuctionClaimMail {
            claim: ClaimAuctionMail { mail_id },
        },
        AuctionCmd::Create {
            item_guid,
            stack,
            bid,
            buyout,
            duration,
        } => Request::AuctionCreate {
            create: CreateAuction {
                item_guid,
                stack_count: stack,
                min_bid: bid,
                buyout_price: buyout,
                duration: parse_duration(&duration)?,
            },
        },
        AuctionCmd::Bid { id, amount } => Request::AuctionBid {
            bid: PlaceBid {
                auction_id: id,
                amount,
            },
        },
        AuctionCmd::Buyout { id } => Request::AuctionBuyout {
            buyout: BuyoutAuction { auction_id: id },
        },
        AuctionCmd::Cancel { id } => Request::AuctionCancel {
            cancel: CancelAuction { auction_id: id },
        },
    };

    match Client::call(socket, &request).map_err(|e| format!("{e}"))? {
        Response::Text(text) => {
            println!("{text}");
            Ok(())
        }
        Response::Error(msg) => Err(msg),
        other => Err(format!("unexpected response: {other:?}")),
    }
}

fn handle_mail(socket: &PathBuf, command: MailCmd) -> Result<(), String> {
    let request = mail_request(command)?;

    match Client::call(socket, &request).map_err(|e| format!("{e}"))? {
        Response::Text(text) => {
            println!("{text}");
            Ok(())
        }
        Response::Error(msg) => Err(msg),
        other => Err(format!("unexpected response: {other:?}")),
    }
}

fn handle_status(socket: &PathBuf, command: StatusCmd) -> Result<(), String> {
    let request = status_request(command)?;

    match Client::call(socket, &request).map_err(|e| format!("{e}"))? {
        Response::Text(text) => {
            println!("{text}");
            Ok(())
        }
        Response::Error(msg) => Err(msg),
        other => Err(format!("unexpected response: {other:?}")),
    }
}

fn handle_item(socket: &PathBuf, command: ItemCmd) -> Result<(), String> {
    let request = item_request(command)?;

    match Client::call(socket, &request).map_err(|e| format!("{e}"))? {
        Response::Text(text) => {
            println!("{text}");
            Ok(())
        }
        Response::Error(msg) => Err(msg),
        other => Err(format!("unexpected response: {other:?}")),
    }
}

fn handle_inventory(socket: &PathBuf, command: InventoryCmd) -> Result<(), String> {
    let request = inventory_request(command)?;

    match Client::call(socket, &request).map_err(|e| format!("{e}"))? {
        Response::Text(text) => {
            println!("{text}");
            Ok(())
        }
        Response::Error(msg) => Err(msg),
        other => Err(format!("unexpected response: {other:?}")),
    }
}

fn handle_quest(socket: &PathBuf, command: QuestCmd) -> Result<(), String> {
    let request = quest_request(command)?;

    match Client::call(socket, &request).map_err(|e| format!("{e}"))? {
        Response::Text(text) => {
            println!("{text}");
            Ok(())
        }
        Response::Error(msg) => Err(msg),
        other => Err(format!("unexpected response: {other:?}")),
    }
}

fn handle_group(socket: &PathBuf, command: GroupCmd) -> Result<(), String> {
    let request = group_request(command)?;

    match Client::call(socket, &request).map_err(|e| format!("{e}"))? {
        Response::Text(text) => {
            println!("{text}");
            Ok(())
        }
        Response::Error(msg) => Err(msg),
        other => Err(format!("unexpected response: {other:?}")),
    }
}

fn handle_spell(socket: &PathBuf, command: SpellCmd) -> Result<(), String> {
    let request = spell_request(command)?;

    match Client::call(socket, &request).map_err(|e| format!("{e}"))? {
        Response::Text(text) => {
            println!("{text}");
            Ok(())
        }
        Response::Error(msg) => Err(msg),
        other => Err(format!("unexpected response: {other:?}")),
    }
}

fn handle_combat(socket: &PathBuf, command: CombatCmd) -> Result<(), String> {
    let request = combat_request(command)?;

    match Client::call(socket, &request).map_err(|e| format!("{e}"))? {
        Response::Text(text) => {
            println!("{text}");
            Ok(())
        }
        Response::Error(msg) => Err(msg),
        other => Err(format!("unexpected response: {other:?}")),
    }
}

fn handle_reputation(socket: &PathBuf, command: ReputationCmd) -> Result<(), String> {
    let request = reputation_request(command)?;

    match Client::call(socket, &request).map_err(|e| format!("{e}"))? {
        Response::Text(text) => {
            println!("{text}");
            Ok(())
        }
        Response::Error(msg) => Err(msg),
        other => Err(format!("unexpected response: {other:?}")),
    }
}

fn handle_collection(socket: &PathBuf, command: CollectionCmd) -> Result<(), String> {
    let request = collection_request(command)?;

    match Client::call(socket, &request).map_err(|e| format!("{e}"))? {
        Response::Text(text) => {
            println!("{text}");
            Ok(())
        }
        Response::Error(msg) => Err(msg),
        other => Err(format!("unexpected response: {other:?}")),
    }
}

fn handle_profession(socket: &PathBuf, command: ProfessionCmd) -> Result<(), String> {
    let request = profession_request(command)?;

    match Client::call(socket, &request).map_err(|e| format!("{e}"))? {
        Response::Text(text) => {
            println!("{text}");
            Ok(())
        }
        Response::Error(msg) => Err(msg),
        other => Err(format!("unexpected response: {other:?}")),
    }
}

fn handle_map(socket: &PathBuf, command: MapCmd) -> Result<(), String> {
    let request = map_request(command)?;

    match Client::call(socket, &request).map_err(|e| format!("{e}"))? {
        Response::Text(text) => {
            println!("{text}");
            Ok(())
        }
        Response::Error(msg) => Err(msg),
        other => Err(format!("unexpected response: {other:?}")),
    }
}

fn handle_equipment(socket: &PathBuf, command: EquipmentCmd) -> Result<(), String> {
    let request = equipment_request(command)?;

    match Client::call(socket, &request).map_err(|e| format!("{e}"))? {
        Response::Text(text) => {
            println!("{text}");
            Ok(())
        }
        Response::Error(msg) => Err(msg),
        other => Err(format!("unexpected response: {other:?}")),
    }
}

fn mail_request(command: MailCmd) -> Result<Request, String> {
    let request = match command {
        MailCmd::Status => Request::MailStatus,
        MailCmd::List {
            character,
            include_deleted,
        } => Request::MailList {
            query: ListMailQuery {
                character,
                include_deleted,
            },
        },
        MailCmd::Read { mail_id } => Request::MailRead {
            read: ReadMail { mail_id },
        },
        MailCmd::Send {
            to,
            from,
            subject,
            body,
            money,
        } => Request::MailSend {
            mail: SendMail {
                to,
                from,
                subject,
                body,
                money,
            },
        },
        MailCmd::Claim { mail_id } => Request::MailClaim {
            claim: ClaimMail { mail_id },
        },
        MailCmd::Delete { mail_id } => Request::MailDelete {
            delete: DeleteMail { mail_id },
        },
    };
    Ok(request)
}

fn item_request(command: ItemCmd) -> Result<Request, String> {
    let request = match command {
        ItemCmd::Info { item_id } => Request::ItemInfo {
            query: ItemInfoQuery { item_id },
        },
    };
    Ok(request)
}

fn inventory_request(command: InventoryCmd) -> Result<Request, String> {
    let request = match command {
        InventoryCmd::List => Request::InventoryList,
        InventoryCmd::Search { text } => Request::InventorySearch { text },
        InventoryCmd::Whereis { item_id } => Request::InventoryWhereis { item_id },
    };
    Ok(request)
}

fn quest_request(command: QuestCmd) -> Result<Request, String> {
    let request = match command {
        QuestCmd::List => Request::QuestList,
        QuestCmd::Watch => Request::QuestWatch,
        QuestCmd::Show { id } => Request::QuestShow { quest_id: id },
    };
    Ok(request)
}

fn group_request(command: GroupCmd) -> Result<Request, String> {
    let request = match command {
        GroupCmd::Roster => Request::GroupRoster,
        GroupCmd::Status => Request::GroupStatus,
        GroupCmd::Invite { name } => Request::GroupInvite { name },
        GroupCmd::Uninvite { name } => Request::GroupUninvite { name },
    };
    Ok(request)
}

fn spell_request(command: SpellCmd) -> Result<Request, String> {
    let request = match command {
        SpellCmd::Cast { spell, target } => Request::SpellCast { spell, target },
        SpellCmd::Stop => Request::SpellStop,
    };
    Ok(request)
}

fn combat_request(command: CombatCmd) -> Result<Request, String> {
    let request = match command {
        CombatCmd::Log { lines } => Request::CombatLog { lines },
        CombatCmd::Recap { target } => Request::CombatRecap { target },
    };
    Ok(request)
}

fn reputation_request(command: ReputationCmd) -> Result<Request, String> {
    let request = match command {
        ReputationCmd::List => Request::ReputationList,
    };
    Ok(request)
}

fn collection_request(command: CollectionCmd) -> Result<Request, String> {
    let request = match command {
        CollectionCmd::Mounts { missing } => Request::CollectionMounts { missing },
        CollectionCmd::Pets { missing } => Request::CollectionPets { missing },
    };
    Ok(request)
}

fn profession_request(command: ProfessionCmd) -> Result<Request, String> {
    let request = match command {
        ProfessionCmd::Recipes { text } => Request::ProfessionRecipes { text },
    };
    Ok(request)
}

fn map_request(command: MapCmd) -> Result<Request, String> {
    let request = match command {
        MapCmd::Position => Request::MapPosition,
        MapCmd::Target => Request::MapTarget,
        MapCmd::Waypoint { command } => match command {
            WaypointCmd::Add { x, y } => Request::MapWaypointAdd { x, y },
            WaypointCmd::Clear => Request::MapWaypointClear,
        },
    };
    Ok(request)
}

fn equipment_request(command: EquipmentCmd) -> Result<Request, String> {
    let request = match command {
        EquipmentCmd::Set { slot, model } => Request::EquipmentSet {
            slot: parse_equipment_slot(&slot)?.to_string(),
            model_path: model.display().to_string(),
        },
        EquipmentCmd::Clear { slot } => Request::EquipmentClear {
            slot: parse_equipment_slot(&slot)?.to_string(),
        },
    };
    Ok(request)
}

fn parse_equipment_slot(value: &str) -> Result<&'static str, String> {
    match value.to_ascii_lowercase().as_str() {
        "mainhand" | "main-hand" | "main" | "mh" => Ok("mainhand"),
        "offhand" | "off-hand" | "off" | "oh" => Ok("offhand"),
        _ => Err(format!(
            "invalid slot '{value}', expected mainhand or offhand"
        )),
    }
}

fn status_request(command: StatusCmd) -> Result<Request, String> {
    let request = match command {
        StatusCmd::Network => Request::NetworkStatus,
        StatusCmd::Terrain => Request::TerrainStatus,
        StatusCmd::Sound => Request::SoundStatus,
        StatusCmd::Currencies => Request::CurrenciesStatus,
        StatusCmd::Reputations => Request::ReputationsStatus,
        StatusCmd::CharacterStats => Request::CharacterStatsStatus,
        StatusCmd::Bags => Request::BagsStatus,
        StatusCmd::GuildVault => Request::GuildVaultStatus,
        StatusCmd::Warbank => Request::WarbankStatus,
        StatusCmd::EquippedGear => Request::EquippedGearStatus,
    };
    Ok(request)
}

fn parse_duration(value: &str) -> Result<AuctionDuration, String> {
    match value.to_ascii_lowercase().as_str() {
        "short" => Ok(AuctionDuration::Short),
        "medium" => Ok(AuctionDuration::Medium),
        "long" => Ok(AuctionDuration::Long),
        _ => Err(format!("invalid duration '{value}'")),
    }
}

fn parse_sort_field(value: &str) -> Result<AuctionSortField, String> {
    match value.to_ascii_lowercase().as_str() {
        "name" => Ok(AuctionSortField::Name),
        "bid" | "min_bid" => Ok(AuctionSortField::MinBid),
        "buyout" => Ok(AuctionSortField::Buyout),
        "time" | "time_left" => Ok(AuctionSortField::TimeLeft),
        "quality" => Ok(AuctionSortField::Quality),
        "level" | "required_level" => Ok(AuctionSortField::RequiredLevel),
        _ => Err(format!("invalid sort field '{value}'")),
    }
}

fn parse_sort_dir(value: &str) -> Result<AuctionSortDir, String> {
    match value.to_ascii_lowercase().as_str() {
        "asc" => Ok(AuctionSortDir::Asc),
        "desc" => Ok(AuctionSortDir::Desc),
        _ => Err(format!("invalid sort direction '{value}'")),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use game_engine::mail::{DeleteMail, ListMailQuery, ReadMail, SendMail};

    #[test]
    fn mail_send_command_maps_to_send_request() {
        let request = mail_request(MailCmd::Send {
            to: "Thrall".into(),
            from: "Jaina".into(),
            subject: "Supplies".into(),
            body: "Three crates are ready.".into(),
            money: 1250,
        })
        .expect("valid send command");

        assert_eq!(
            request,
            Request::MailSend {
                mail: SendMail {
                    to: "Thrall".into(),
                    from: "Jaina".into(),
                    subject: "Supplies".into(),
                    body: "Three crates are ready.".into(),
                    money: 1250,
                },
            }
        );
    }

    #[test]
    fn mail_list_command_maps_to_list_request() {
        let request = mail_request(MailCmd::List {
            character: Some("Thrall".into()),
            include_deleted: true,
        })
        .expect("valid list command");

        assert_eq!(
            request,
            Request::MailList {
                query: ListMailQuery {
                    character: Some("Thrall".into()),
                    include_deleted: true,
                },
            }
        );
    }

    #[test]
    fn mail_read_command_maps_to_read_request() {
        let request = mail_request(MailCmd::Read { mail_id: 42 }).expect("valid read command");

        assert_eq!(
            request,
            Request::MailRead {
                read: ReadMail { mail_id: 42 },
            }
        );
    }

    #[test]
    fn mail_delete_command_maps_to_delete_request() {
        let request = mail_request(MailCmd::Delete { mail_id: 42 }).expect("valid delete command");

        assert_eq!(
            request,
            Request::MailDelete {
                delete: DeleteMail { mail_id: 42 },
            }
        );
    }

    #[test]
    fn network_status_command_maps_to_request() {
        let request = status_request(StatusCmd::Network).expect("valid status command");

        assert_eq!(request, Request::NetworkStatus);
    }

    #[test]
    fn terrain_status_command_maps_to_request() {
        let request = status_request(StatusCmd::Terrain).expect("valid status command");

        assert_eq!(request, Request::TerrainStatus);
    }

    #[test]
    fn sound_status_command_maps_to_request() {
        let request = status_request(StatusCmd::Sound).expect("valid status command");

        assert_eq!(request, Request::SoundStatus);
    }

    #[test]
    fn currencies_status_command_maps_to_request() {
        let request = status_request(StatusCmd::Currencies).expect("valid status command");

        assert_eq!(request, Request::CurrenciesStatus);
    }

    #[test]
    fn reputations_status_command_maps_to_request() {
        let request = status_request(StatusCmd::Reputations).expect("valid status command");

        assert_eq!(request, Request::ReputationsStatus);
    }

    #[test]
    fn character_stats_status_command_maps_to_request() {
        let request = status_request(StatusCmd::CharacterStats).expect("valid status command");

        assert_eq!(request, Request::CharacterStatsStatus);
    }

    #[test]
    fn bags_status_command_maps_to_request() {
        let request = status_request(StatusCmd::Bags).expect("valid status command");

        assert_eq!(request, Request::BagsStatus);
    }

    #[test]
    fn guild_vault_status_command_maps_to_request() {
        let request = status_request(StatusCmd::GuildVault).expect("valid status command");

        assert_eq!(request, Request::GuildVaultStatus);
    }

    #[test]
    fn warbank_status_command_maps_to_request() {
        let request = status_request(StatusCmd::Warbank).expect("valid status command");

        assert_eq!(request, Request::WarbankStatus);
    }

    #[test]
    fn equipped_gear_status_command_maps_to_request() {
        let request = status_request(StatusCmd::EquippedGear).expect("valid status command");

        assert_eq!(request, Request::EquippedGearStatus);
    }

    #[test]
    fn item_info_command_maps_to_request() {
        let request = item_request(ItemCmd::Info { item_id: 2589 }).expect("valid item command");

        assert_eq!(
            request,
            Request::ItemInfo {
                query: ItemInfoQuery { item_id: 2589 },
            }
        );
    }

    #[test]
    fn spell_cast_command_maps_to_ipc_request() {
        let request = spell_request(SpellCmd::Cast {
            spell: "133".into(),
            target: Some("current".into()),
        })
        .expect("valid spell cast command");

        assert!(matches!(request, Request::SpellCast { .. }));
    }

    #[test]
    fn inventory_search_command_maps_to_request() {
        let request = inventory_request(InventoryCmd::Search {
            text: "torch".into(),
        })
        .expect("valid inventory search command");

        assert_eq!(
            request,
            Request::InventorySearch {
                text: "torch".into()
            }
        );
    }

    #[test]
    fn quest_list_command_maps_to_request() {
        let request = quest_request(QuestCmd::List).expect("valid quest list command");

        assert_eq!(request, Request::QuestList);
    }

    #[test]
    fn group_roster_command_maps_to_request() {
        let request = group_request(GroupCmd::Roster).expect("valid group roster command");

        assert_eq!(request, Request::GroupRoster);
    }

    #[test]
    fn map_waypoint_add_command_maps_to_request() {
        let request = map_request(MapCmd::Waypoint {
            command: WaypointCmd::Add { x: 42.1, y: 65.7 },
        })
        .expect("valid waypoint command");

        assert_eq!(request, Request::MapWaypointAdd { x: 42.1, y: 65.7 });
    }

    #[test]
    fn combat_log_command_maps_to_request() {
        let request = combat_request(CombatCmd::Log { lines: 10 }).expect("valid combat command");

        assert_eq!(request, Request::CombatLog { lines: 10 });
    }

    #[test]
    fn reputation_list_command_maps_to_request() {
        let request =
            reputation_request(ReputationCmd::List).expect("valid reputation list command");

        assert_eq!(request, Request::ReputationList);
    }

    #[test]
    fn collection_mounts_missing_command_maps_to_request() {
        let request = collection_request(CollectionCmd::Mounts { missing: true })
            .expect("valid collection mounts command");

        assert_eq!(request, Request::CollectionMounts { missing: true });
    }

    #[test]
    fn profession_recipes_command_maps_to_request() {
        let request = profession_request(ProfessionCmd::Recipes {
            text: "potion".into(),
        })
        .expect("valid profession recipes command");

        assert_eq!(
            request,
            Request::ProfessionRecipes {
                text: "potion".into()
            }
        );
    }

    #[test]
    fn map_target_command_maps_to_request() {
        let request = map_request(MapCmd::Target).expect("valid map target command");

        assert_eq!(request, Request::MapTarget);
    }

    #[test]
    fn equipment_set_command_maps_to_request() {
        let request = equipment_request(EquipmentCmd::Set {
            slot: "mainhand".into(),
            model: PathBuf::from("data/models/club_1h_torch_a_01.m2"),
        })
        .expect("valid equipment set command");

        assert_eq!(
            request,
            Request::EquipmentSet {
                slot: "mainhand".into(),
                model_path: "data/models/club_1h_torch_a_01.m2".into(),
            }
        );
    }

    #[test]
    fn equipment_clear_command_maps_to_request() {
        let request = equipment_request(EquipmentCmd::Clear {
            slot: "offhand".into(),
        })
        .expect("valid equipment clear command");

        assert_eq!(
            request,
            Request::EquipmentClear {
                slot: "offhand".into(),
            }
        );
    }
}
