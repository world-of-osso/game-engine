use game_engine::ipc::Request;
use game_engine::item_info::ItemInfoQuery;
use game_engine::mail::{ClaimMail, DeleteMail, ListMailQuery, ReadMail, SendMail};
use shared::protocol::{
    AuctionDuration, AuctionSearchQuery, AuctionSortDir, AuctionSortField, BuyoutAuction,
    CancelAuction, ClaimAuctionMail, CreateAuction, PlaceBid,
};

use crate::{
    AuctionCmd, CollectionCmd, CombatCmd, EquipmentCmd, GroupCmd, InventoryCmd, ItemCmd, MailCmd,
    MapCmd, ProfessionCmd, QuestCmd, ReputationCmd, SpellCmd, StatusCmd, TalentCmd, TradeCmd,
    WaypointCmd,
};

pub fn mail_request(command: MailCmd) -> Result<Request, String> {
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

pub fn item_request(command: ItemCmd) -> Result<Request, String> {
    let request = match command {
        ItemCmd::Info { item_id } => Request::ItemInfo {
            query: ItemInfoQuery { item_id },
        },
    };
    Ok(request)
}

pub fn inventory_request(command: InventoryCmd) -> Result<Request, String> {
    let request = match command {
        InventoryCmd::List => Request::InventoryList,
        InventoryCmd::Search { text } => Request::InventorySearch { text },
        InventoryCmd::Whereis { item_id } => Request::InventoryWhereis { item_id },
    };
    Ok(request)
}

pub fn quest_request(command: QuestCmd) -> Result<Request, String> {
    let request = match command {
        QuestCmd::List => Request::QuestList,
        QuestCmd::Watch => Request::QuestWatch,
        QuestCmd::Show { id } => Request::QuestShow { quest_id: id },
    };
    Ok(request)
}

pub fn group_request(command: GroupCmd) -> Result<Request, String> {
    let request = match command {
        GroupCmd::Roster => Request::GroupRoster,
        GroupCmd::Status => Request::GroupStatus,
        GroupCmd::Invite { name } => Request::GroupInvite { name },
        GroupCmd::Uninvite { name } => Request::GroupUninvite { name },
    };
    Ok(request)
}

pub fn trade_request(command: TradeCmd) -> Result<Request, String> {
    let request = match command {
        TradeCmd::Status => Request::TradeStatus,
        TradeCmd::Initiate { name } => Request::TradeInitiate { name },
        TradeCmd::Accept => Request::TradeAccept,
        TradeCmd::Decline => Request::TradeDecline,
        TradeCmd::Cancel => Request::TradeCancel,
        TradeCmd::SetItem {
            slot,
            item_guid,
            stack,
        } => Request::TradeSetItem {
            slot,
            item_guid,
            stack_count: stack,
        },
        TradeCmd::ClearItem { slot } => Request::TradeClearItem { slot },
        TradeCmd::SetMoney { copper } => Request::TradeSetMoney { copper },
        TradeCmd::Confirm => Request::TradeConfirm,
    };
    Ok(request)
}

pub fn talent_request(command: TalentCmd) -> Result<Request, String> {
    let request = match command {
        TalentCmd::Status => Request::TalentStatus,
        TalentCmd::Apply { talent_id } => Request::TalentApply { talent_id },
        TalentCmd::Reset => Request::TalentReset,
    };
    Ok(request)
}

pub fn spell_request(command: SpellCmd) -> Result<Request, String> {
    let request = match command {
        SpellCmd::Cast { spell, target } => Request::SpellCast { spell, target },
        SpellCmd::Stop => Request::SpellStop,
    };
    Ok(request)
}

pub fn combat_request(command: CombatCmd) -> Result<Request, String> {
    let request = match command {
        CombatCmd::Log { lines } => Request::CombatLog { lines },
        CombatCmd::Recap { target } => Request::CombatRecap { target },
    };
    Ok(request)
}

pub fn reputation_request(command: ReputationCmd) -> Result<Request, String> {
    let request = match command {
        ReputationCmd::List => Request::ReputationList,
    };
    Ok(request)
}

pub fn collection_request(command: CollectionCmd) -> Result<Request, String> {
    let request = match command {
        CollectionCmd::Mounts { missing } => Request::CollectionMounts { missing },
        CollectionCmd::Pets { missing } => Request::CollectionPets { missing },
    };
    Ok(request)
}

pub fn profession_request(command: ProfessionCmd) -> Result<Request, String> {
    let request = match command {
        ProfessionCmd::Status => Request::ProfessionStatus,
        ProfessionCmd::Recipes { text } => Request::ProfessionRecipes { text },
        ProfessionCmd::Craft { recipe_id } => Request::ProfessionCraft { recipe_id },
        ProfessionCmd::Gather { node_id } => Request::ProfessionGather { node_id },
    };
    Ok(request)
}

pub fn map_request(command: MapCmd) -> Result<Request, String> {
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

pub fn equipment_request(command: EquipmentCmd) -> Result<Request, String> {
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

pub fn export_character_request(
    output: std::path::PathBuf,
    character_name: Option<String>,
    character_id: Option<u64>,
) -> Request {
    Request::ExportCharacter {
        output_path: output.display().to_string(),
        character_name,
        character_id,
    }
}

pub fn export_scene_request(output: std::path::PathBuf) -> Request {
    Request::ExportScene {
        output_path: output.display().to_string(),
    }
}

pub fn parse_equipment_slot(value: &str) -> Result<&'static str, String> {
    match value.to_ascii_lowercase().as_str() {
        "mainhand" | "main-hand" | "main" | "mh" => Ok("mainhand"),
        "offhand" | "off-hand" | "off" | "oh" => Ok("offhand"),
        _ => Err(format!(
            "invalid slot '{value}', expected mainhand or offhand"
        )),
    }
}

pub fn status_request(command: StatusCmd) -> Result<Request, String> {
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

pub struct AuctionBrowseRequestArgs {
    pub text: String,
    pub page: u32,
    pub page_size: u32,
    pub min_level: Option<u16>,
    pub max_level: Option<u16>,
    pub quality: Option<u8>,
    pub sort: String,
    pub dir: String,
}

struct AuctionBrowseCommand {
    text: String,
    page: u32,
    page_size: u32,
    min_level: Option<u16>,
    max_level: Option<u16>,
    quality: Option<u8>,
    sort: String,
    dir: String,
}

enum AuctionActionCommand {
    ClaimMail { mail_id: u64 },
    Bid { id: u64, amount: u32 },
    Buyout { id: u64 },
    Cancel { id: u64 },
}

enum AuctionNonSimpleCommand {
    Browse(AuctionBrowseCommand),
    Create {
        item_guid: u64,
        stack: u32,
        bid: u32,
        buyout: Option<u32>,
        duration: String,
    },
    Action(AuctionActionCommand),
}

pub fn auction_browse_request(args: AuctionBrowseRequestArgs) -> Result<Request, String> {
    Ok(Request::AuctionBrowse {
        query: AuctionSearchQuery {
            text: args.text,
            page: args.page,
            page_size: args.page_size,
            min_level: args.min_level,
            max_level: args.max_level,
            quality: args.quality,
            usable_only: false,
            sort_field: parse_sort_field(&args.sort)?,
            sort_dir: parse_sort_dir(&args.dir)?,
        },
    })
}

pub fn auction_create_request(
    item_guid: u64,
    stack: u32,
    bid: u32,
    buyout: Option<u32>,
    duration: String,
) -> Result<Request, String> {
    Ok(Request::AuctionCreate {
        create: CreateAuction {
            item_guid,
            stack_count: stack,
            min_bid: bid,
            buyout_price: buyout,
            duration: parse_duration(&duration)?,
        },
    })
}

pub fn auction_request(command: AuctionCmd) -> Result<Request, String> {
    if let Some(request) = simple_auction_request(&command) {
        return Ok(request);
    }
    auction_non_simple_request(to_non_simple_auction_command(command))
}

fn simple_auction_request(command: &AuctionCmd) -> Option<Request> {
    match command {
        AuctionCmd::Open => Some(Request::AuctionOpen),
        AuctionCmd::Status => Some(Request::AuctionStatus),
        AuctionCmd::Owned => Some(Request::AuctionOwned),
        AuctionCmd::Bids => Some(Request::AuctionBids),
        AuctionCmd::Inventory => Some(Request::AuctionInventory),
        AuctionCmd::Mailbox => Some(Request::AuctionMailbox),
        _ => None,
    }
}

fn to_non_simple_auction_command(command: AuctionCmd) -> AuctionNonSimpleCommand {
    match command {
        AuctionCmd::Browse { .. } => browse_non_simple_auction_command(command),
        AuctionCmd::Create { .. } => create_non_simple_auction_command(command),
        AuctionCmd::ClaimMail { .. }
        | AuctionCmd::Bid { .. }
        | AuctionCmd::Buyout { .. }
        | AuctionCmd::Cancel { .. } => action_non_simple_auction_command(command),
        AuctionCmd::Open
        | AuctionCmd::Status
        | AuctionCmd::Owned
        | AuctionCmd::Bids
        | AuctionCmd::Inventory
        | AuctionCmd::Mailbox => unreachable!("simple auction commands returned above"),
    }
}

fn browse_non_simple_auction_command(command: AuctionCmd) -> AuctionNonSimpleCommand {
    let AuctionCmd::Browse {
        text,
        page,
        page_size,
        min_level,
        max_level,
        quality,
        sort,
        dir,
    } = command
    else {
        unreachable!("browse helper must only receive browse command");
    };
    AuctionNonSimpleCommand::Browse(AuctionBrowseCommand {
        text,
        page,
        page_size,
        min_level,
        max_level,
        quality,
        sort,
        dir,
    })
}

fn create_non_simple_auction_command(command: AuctionCmd) -> AuctionNonSimpleCommand {
    let AuctionCmd::Create {
        item_guid,
        stack,
        bid,
        buyout,
        duration,
    } = command
    else {
        unreachable!("create helper must only receive create command");
    };
    AuctionNonSimpleCommand::Create {
        item_guid,
        stack,
        bid,
        buyout,
        duration,
    }
}

fn action_non_simple_auction_command(command: AuctionCmd) -> AuctionNonSimpleCommand {
    AuctionNonSimpleCommand::Action(match command {
        AuctionCmd::ClaimMail { mail_id } => AuctionActionCommand::ClaimMail { mail_id },
        AuctionCmd::Bid { id, amount } => AuctionActionCommand::Bid { id, amount },
        AuctionCmd::Buyout { id } => AuctionActionCommand::Buyout { id },
        AuctionCmd::Cancel { id } => AuctionActionCommand::Cancel { id },
        _ => unreachable!("action helper must only receive action command"),
    })
}

fn auction_non_simple_request(command: AuctionNonSimpleCommand) -> Result<Request, String> {
    match command {
        AuctionNonSimpleCommand::Browse(command) => auction_browse_command_request(command),
        AuctionNonSimpleCommand::Create { .. } => auction_create_command_request(command),
        AuctionNonSimpleCommand::Action(command) => Ok(auction_action_request(command)),
    }
}

fn auction_create_command_request(command: AuctionNonSimpleCommand) -> Result<Request, String> {
    let AuctionNonSimpleCommand::Create {
        item_guid,
        stack,
        bid,
        buyout,
        duration,
    } = command
    else {
        unreachable!("create helper must only receive create command");
    };
    auction_create_request(item_guid, stack, bid, buyout, duration)
}

fn auction_browse_command_request(command: AuctionBrowseCommand) -> Result<Request, String> {
    auction_browse_request(AuctionBrowseRequestArgs {
        text: command.text,
        page: command.page,
        page_size: command.page_size,
        min_level: command.min_level,
        max_level: command.max_level,
        quality: command.quality,
        sort: command.sort,
        dir: command.dir,
    })
}

fn auction_action_request(command: AuctionActionCommand) -> Request {
    match command {
        AuctionActionCommand::ClaimMail { mail_id } => Request::AuctionClaimMail {
            claim: ClaimAuctionMail { mail_id },
        },
        AuctionActionCommand::Bid { id, amount } => Request::AuctionBid {
            bid: PlaceBid {
                auction_id: id,
                amount,
            },
        },
        AuctionActionCommand::Buyout { id } => Request::AuctionBuyout {
            buyout: BuyoutAuction { auction_id: id },
        },
        AuctionActionCommand::Cancel { id } => Request::AuctionCancel {
            cancel: CancelAuction { auction_id: id },
        },
    }
}

pub fn parse_duration(value: &str) -> Result<AuctionDuration, String> {
    match value.to_ascii_lowercase().as_str() {
        "short" => Ok(AuctionDuration::Short),
        "medium" => Ok(AuctionDuration::Medium),
        "long" => Ok(AuctionDuration::Long),
        _ => Err(format!("invalid duration '{value}'")),
    }
}

pub fn parse_sort_field(value: &str) -> Result<AuctionSortField, String> {
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

pub fn parse_sort_dir(value: &str) -> Result<AuctionSortDir, String> {
    match value.to_ascii_lowercase().as_str() {
        "asc" => Ok(AuctionSortDir::Asc),
        "desc" => Ok(AuctionSortDir::Desc),
        _ => Err(format!("invalid sort direction '{value}'")),
    }
}
