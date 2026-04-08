use game_engine::ipc::Request;
use game_engine::item_info::ItemInfoQuery;
use game_engine::mail::{ClaimMail, DeleteMail, ListMailQuery, ReadMail, SendMail};
use shared::protocol::{
    AuctionDuration, AuctionSearchQuery, AuctionSortDir, AuctionSortField, BuyoutAuction,
    CancelAuction, ClaimAuctionMail, CreateAuction, EmoteKind, PlaceBid, PvpBracketSnapshot,
};

use crate::{
    AuctionCmd, BarberCmd, CollectionCmd, CombatCmd, CurrencyCmd, DeathCmd, DuelCmd, EmoteCmd,
    EquipmentCmd, FriendCmd, GroupCmd, IgnoreCmd, InspectCmd, InventoryCmd, ItemCmd, LfgCmd,
    MailCmd, MapCmd, ProfessionCmd, PvpCmd, QuestCmd, ReputationCmd, SpellCmd, StatusCmd,
    TalentCmd, TradeCmd, WaypointCmd,
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

pub fn barber_request(command: BarberCmd) -> Result<Request, String> {
    let request = match command {
        BarberCmd::Status => Request::BarberStatus,
        BarberCmd::Set { option, value } => Request::BarberSet {
            option: parse_barber_option(&option)?,
            value,
        },
        BarberCmd::Reset => Request::BarberReset,
        BarberCmd::Apply => Request::BarberApply,
    };
    Ok(request)
}

pub fn death_request(command: DeathCmd) -> Result<Request, String> {
    let request = match command {
        DeathCmd::Status => Request::DeathStatus,
        DeathCmd::ReleaseSpirit => Request::DeathReleaseSpirit,
        DeathCmd::ResurrectAtCorpse => Request::DeathResurrectAtCorpse,
        DeathCmd::AcceptSpiritHealer => Request::DeathAcceptSpiritHealer,
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

pub fn friend_request(command: FriendCmd) -> Result<Request, String> {
    let request = match command {
        FriendCmd::Status => Request::FriendsStatus,
        FriendCmd::Add { name } => Request::FriendAdd { name },
        FriendCmd::Remove { name } => Request::FriendRemove { name },
    };
    Ok(request)
}

pub fn ignore_request(command: IgnoreCmd) -> Result<Request, String> {
    let request = match command {
        IgnoreCmd::Status => Request::IgnoreStatus,
        IgnoreCmd::Add { name } => Request::IgnoreAdd { name },
        IgnoreCmd::Remove { name } => Request::IgnoreRemove { name },
    };
    Ok(request)
}

pub fn lfg_request(command: LfgCmd) -> Result<Request, String> {
    let request = match command {
        LfgCmd::Status => Request::LfgStatus,
        LfgCmd::Queue { role, dungeon_ids } => Request::LfgQueue {
            role: parse_group_role(&role)?,
            dungeon_ids,
        },
        LfgCmd::Dequeue => Request::LfgDequeue,
        LfgCmd::Accept => Request::LfgAccept,
        LfgCmd::Decline => Request::LfgDecline,
    };
    Ok(request)
}

pub fn pvp_request(command: PvpCmd) -> Result<Request, String> {
    let request = match command {
        PvpCmd::Status => Request::PvpStatus,
        PvpCmd::QueueBattleground { battleground_id } => {
            Request::PvpQueueBattleground { battleground_id }
        }
        PvpCmd::QueueRated { bracket } => Request::PvpQueueRated {
            bracket: parse_pvp_bracket(&bracket)?,
        },
        PvpCmd::Dequeue => Request::PvpDequeue,
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

pub fn duel_request(command: DuelCmd) -> Result<Request, String> {
    let request = match command {
        DuelCmd::Status => Request::DuelStatus,
        DuelCmd::Challenge => Request::DuelChallenge,
        DuelCmd::Accept => Request::DuelAccept,
        DuelCmd::Decline => Request::DuelDecline,
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

pub fn inspect_request(command: InspectCmd) -> Result<Request, String> {
    let request = match command {
        InspectCmd::Status => Request::InspectStatus,
        InspectCmd::Query => Request::InspectQuery,
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

pub fn emote_request(command: EmoteCmd) -> Result<Request, String> {
    let emote = match command {
        EmoteCmd::Dance => EmoteKind::Dance,
        EmoteCmd::Wave => EmoteKind::Wave,
    };
    Ok(Request::Emote { emote })
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

pub fn currency_request(command: CurrencyCmd) -> Result<Request, String> {
    let request = match command {
        CurrencyCmd::Status => Request::CurrenciesStatus,
        CurrencyCmd::Earn {
            currency_id,
            amount,
        } => Request::CurrencyEarn {
            currency_id,
            amount,
        },
        CurrencyCmd::Spend {
            currency_id,
            amount,
        } => Request::CurrencySpend {
            currency_id,
            amount,
        },
    };
    Ok(request)
}

pub fn collection_request(command: CollectionCmd) -> Result<Request, String> {
    let request = match command {
        CollectionCmd::Mounts { missing } => Request::CollectionMounts { missing },
        CollectionCmd::Pets { missing } => Request::CollectionPets { missing },
        CollectionCmd::SummonMount { mount_id } => Request::CollectionSummonMount { mount_id },
        CollectionCmd::DismissMount => Request::CollectionDismissMount,
        CollectionCmd::SummonPet { pet_id } => Request::CollectionSummonPet { pet_id },
        CollectionCmd::DismissPet => Request::CollectionDismissPet,
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
        StatusCmd::Achievements => Request::AchievementsStatus,
        StatusCmd::Barber => Request::BarberStatus,
        StatusCmd::Death => Request::DeathStatus,
        StatusCmd::EncounterJournal => Request::EncounterJournalStatus,
        StatusCmd::Friends => Request::FriendsStatus,
        StatusCmd::Ignore => Request::IgnoreStatus,
        StatusCmd::Lfg => Request::LfgStatus,
        StatusCmd::Pvp => Request::PvpStatus,
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

fn parse_pvp_bracket(value: &str) -> Result<PvpBracketSnapshot, String> {
    match value.to_ascii_lowercase().as_str() {
        "2v2" | "arena2v2" => Ok(PvpBracketSnapshot::Arena2v2),
        "3v3" | "arena3v3" => Ok(PvpBracketSnapshot::Arena3v3),
        "rbg" | "ratedbg" | "rated-battleground" => Ok(PvpBracketSnapshot::RatedBattleground),
        "solo" | "solo-shuffle" | "soloshuffle" => Ok(PvpBracketSnapshot::SoloShuffle),
        _ => Err(format!("unknown pvp bracket '{value}'")),
    }
}

fn parse_group_role(value: &str) -> Result<game_engine::status::GroupRole, String> {
    match value.trim().to_ascii_lowercase().as_str() {
        "tank" => Ok(game_engine::status::GroupRole::Tank),
        "healer" | "heal" => Ok(game_engine::status::GroupRole::Healer),
        "damage" | "dps" => Ok(game_engine::status::GroupRole::Damage),
        "none" => Ok(game_engine::status::GroupRole::None),
        _ => Err(format!("unknown role: {value}")),
    }
}

fn parse_barber_option(value: &str) -> Result<game_engine::ipc::BarberOption, String> {
    match value.trim().to_ascii_lowercase().as_str() {
        "hair-style" | "hairstyle" | "hair_style" => Ok(game_engine::ipc::BarberOption::HairStyle),
        "hair-color" | "haircolor" | "hair_color" => Ok(game_engine::ipc::BarberOption::HairColor),
        "facial-hair" | "facialhair" | "facial_hair" => {
            Ok(game_engine::ipc::BarberOption::FacialHair)
        }
        "skin-color" | "skincolor" | "skin_color" => Ok(game_engine::ipc::BarberOption::SkinColor),
        "face" => Ok(game_engine::ipc::BarberOption::Face),
        _ => Err(format!("unknown barber option: {value}")),
    }
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
