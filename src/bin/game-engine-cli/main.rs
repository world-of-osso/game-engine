mod command_dispatch;
mod requests;
#[cfg(test)]
mod tests;

use std::path::PathBuf;

use clap::{Parser, Subcommand};
use game_engine::ipc::socket_glob;

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
    /// Duel operations via the running engine
    Duel {
        #[command(subcommand)]
        command: DuelCmd,
    },
    /// Talent operations via the running engine
    Talent {
        #[command(subcommand)]
        command: TalentCmd,
    },
    /// Inspect current target via the running engine
    Inspect {
        #[command(subcommand)]
        command: InspectCmd,
    },
    /// Runtime subsystem status via the running engine
    Status {
        #[command(subcommand)]
        command: StatusCmd,
    },
    /// Barber shop commands
    Barber {
        #[command(subcommand)]
        command: BarberCmd,
    },
    /// Death and respawn commands
    Death {
        #[command(subcommand)]
        command: DeathCmd,
    },
    /// Currency wallet operations via the running engine
    Currency {
        #[command(subcommand)]
        command: CurrencyCmd,
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
    /// Friends list commands
    Friend {
        #[command(subcommand)]
        command: FriendCmd,
    },
    /// Guild roster and info commands
    Guild {
        #[command(subcommand)]
        command: GuildCmd,
    },
    /// Calendar commands
    Calendar {
        #[command(subcommand)]
        command: CalendarCmd,
    },
    /// Who list query
    Who {
        #[command(subcommand)]
        command: WhoCmd,
    },
    /// Presence status commands
    Presence {
        #[command(subcommand)]
        command: PresenceCmd,
    },
    /// Ignore list commands
    Ignore {
        #[command(subcommand)]
        command: IgnoreCmd,
    },
    /// Dungeon finder commands
    Lfg {
        #[command(subcommand)]
        command: LfgCmd,
    },
    /// PVP status and queue commands
    Pvp {
        #[command(subcommand)]
        command: PvpCmd,
    },
    /// Spell commands
    Spell {
        #[command(subcommand)]
        command: SpellCmd,
    },
    /// Social emote commands
    Emote {
        #[command(subcommand)]
        command: EmoteCmd,
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
    Achievements,
    Barber,
    Calendar,
    Death,
    EncounterJournal,
    Friends,
    Guild,
    Who,
    Ignore,
    Lfg,
    Pvp,
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
pub(crate) enum FriendCmd {
    Status,
    Add {
        #[arg(long)]
        name: String,
    },
    Remove {
        #[arg(long)]
        name: String,
    },
}

#[derive(Subcommand)]
pub(crate) enum CalendarCmd {
    Query,
    Schedule {
        #[arg(long)]
        title: String,
        #[arg(long, default_value_t = 60)]
        in_minutes: u32,
        #[arg(long, default_value_t = 10)]
        max_signups: u8,
        #[arg(long, default_value_t = true)]
        raid: bool,
    },
    Confirm {
        #[arg(long)]
        event_id: u64,
    },
    Tentative {
        #[arg(long)]
        event_id: u64,
    },
    Decline {
        #[arg(long)]
        event_id: u64,
    },
}

#[derive(Subcommand)]
pub(crate) enum GuildCmd {
    Query,
    Status,
    Motd {
        #[arg(long)]
        text: String,
    },
    Info {
        #[arg(long)]
        text: String,
    },
    OfficerNote {
        #[arg(long)]
        name: String,
        #[arg(long)]
        note: String,
    },
}

#[derive(Subcommand)]
pub(crate) enum WhoCmd {
    Query {
        #[arg(long, default_value = "")]
        text: String,
    },
}

#[derive(Subcommand)]
pub(crate) enum PresenceCmd {
    Status,
    Afk,
    Dnd,
    Online,
}

#[derive(Subcommand)]
pub(crate) enum IgnoreCmd {
    Status,
    Add {
        #[arg(long)]
        name: String,
    },
    Remove {
        #[arg(long)]
        name: String,
    },
}

#[derive(Subcommand)]
pub(crate) enum BarberCmd {
    Status,
    Set {
        #[arg(long)]
        option: String,
        #[arg(long)]
        value: u8,
    },
    Reset,
    Apply,
}

#[derive(Subcommand)]
pub(crate) enum DeathCmd {
    Status,
    ReleaseSpirit,
    ResurrectAtCorpse,
    AcceptSpiritHealer,
    Stuck,
}

#[derive(Subcommand)]
pub(crate) enum LfgCmd {
    Status,
    Queue {
        #[arg(long)]
        role: String,
        #[arg(long = "dungeon-id", required = true)]
        dungeon_ids: Vec<u32>,
    },
    Dequeue,
    Accept,
    Decline,
}

#[derive(Subcommand)]
pub(crate) enum PvpCmd {
    Status,
    QueueBattleground {
        #[arg(long)]
        battleground_id: u32,
    },
    QueueRated {
        #[arg(long)]
        bracket: String,
    },
    Dequeue,
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
pub(crate) enum DuelCmd {
    Status,
    Challenge,
    Accept,
    Decline,
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
pub(crate) enum InspectCmd {
    Status,
    Query,
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
pub(crate) enum EmoteCmd {
    Dance,
    Wave,
    Sit,
    Sleep,
    Kneel,
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
pub(crate) enum CurrencyCmd {
    Status,
    Earn {
        #[arg(long)]
        currency_id: u32,
        #[arg(long)]
        amount: u32,
    },
    Spend {
        #[arg(long)]
        currency_id: u32,
        #[arg(long)]
        amount: u32,
    },
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
    SummonMount {
        #[arg(long)]
        mount_id: u32,
    },
    DismissMount,
    SummonPet {
        #[arg(long)]
        pet_id: u32,
    },
    DismissPet,
}

#[derive(Subcommand)]
pub(crate) enum ProfessionCmd {
    Status,
    Recipes {
        #[arg(long, default_value = "")]
        text: String,
    },
    Craft {
        #[arg(long)]
        recipe_id: u32,
    },
    Gather {
        #[arg(long)]
        node_id: u32,
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

fn main() {
    let cli = Cli::parse();
    let socket = resolve_socket(cli.socket);
    let result = command_dispatch::dispatch_command(&socket, cli.command, cli.json);
    if let Err(e) = result {
        eprintln!("error: {e}");
        std::process::exit(1);
    }
}
