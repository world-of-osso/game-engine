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
}
