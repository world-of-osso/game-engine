use game_engine::ipc::Request;
use game_engine::item_info::ItemInfoQuery;
use game_engine::mail::{ClaimMail, DeleteMail, ListMailQuery, ReadMail, SendMail};
use shared::protocol::{
    AuctionDuration, AuctionSearchQuery, AuctionSortDir, AuctionSortField, BuyoutAuction,
    CancelAuction, ClaimAuctionMail, CreateAuction, PlaceBid,
};

use crate::{
    AuctionCmd, CollectionCmd, CombatCmd, EquipmentCmd, GroupCmd, InventoryCmd, ItemCmd, MailCmd,
    MapCmd, ProfessionCmd, QuestCmd, ReputationCmd, SpellCmd, StatusCmd, WaypointCmd,
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
        ProfessionCmd::Recipes { text } => Request::ProfessionRecipes { text },
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

#[allow(clippy::too_many_arguments)]
pub fn auction_browse_request(
    text: String,
    page: u32,
    page_size: u32,
    min_level: Option<u16>,
    max_level: Option<u16>,
    quality: Option<u8>,
    sort: String,
    dir: String,
) -> Result<Request, String> {
    Ok(Request::AuctionBrowse {
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
    match command {
        AuctionCmd::Open => Ok(Request::AuctionOpen),
        AuctionCmd::Status => Ok(Request::AuctionStatus),
        AuctionCmd::Browse {
            text,
            page,
            page_size,
            min_level,
            max_level,
            quality,
            sort,
            dir,
        } => auction_browse_request(
            text, page, page_size, min_level, max_level, quality, sort, dir,
        ),
        AuctionCmd::Owned => Ok(Request::AuctionOwned),
        AuctionCmd::Bids => Ok(Request::AuctionBids),
        AuctionCmd::Inventory => Ok(Request::AuctionInventory),
        AuctionCmd::Mailbox => Ok(Request::AuctionMailbox),
        AuctionCmd::ClaimMail { mail_id } => Ok(Request::AuctionClaimMail {
            claim: ClaimAuctionMail { mail_id },
        }),
        AuctionCmd::Create {
            item_guid,
            stack,
            bid,
            buyout,
            duration,
        } => auction_create_request(item_guid, stack, bid, buyout, duration),
        AuctionCmd::Bid { id, amount } => Ok(Request::AuctionBid {
            bid: PlaceBid {
                auction_id: id,
                amount,
            },
        }),
        AuctionCmd::Buyout { id } => Ok(Request::AuctionBuyout {
            buyout: BuyoutAuction { auction_id: id },
        }),
        AuctionCmd::Cancel { id } => Ok(Request::AuctionCancel {
            cancel: CancelAuction { auction_id: id },
        }),
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
