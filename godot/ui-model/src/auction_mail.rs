//! Native equivalent of Retail's mail API: localize auction subjects and decode
//! invoices before MailFrame renders them (Retail MailFrame.lua:554-630).
use std::collections::HashMap;
use std::sync::OnceLock;

use shared::protocol::MailHeader;

use crate::mail_frame_component::AuctionInvoiceView;
use crate::spell_catalog::csv_records::CsvTable;

static STRINGS: OnceLock<HashMap<String, String>> = OnceLock::new();

pub fn text(tag: &str) -> &'static str {
    let strings = STRINGS.get_or_init(load_retail_strings);
    strings
        .get(tag)
        .unwrap_or_else(|| panic!("Retail GlobalStrings missing {tag}"))
}

fn load_retail_strings() -> HashMap<String, String> {
    let path = crate::paths::resolve_data_path("db2/12.1.0.69933/GlobalStrings.csv");
    let table = CsvTable::read(&path).expect("read Retail GlobalStrings for auction mail");
    let tag = table
        .column("BaseTag")
        .expect("GlobalStrings BaseTag column");
    let value = table
        .column("TagText_lang")
        .expect("GlobalStrings TagText_lang column");
    table
        .records()
        .map(|row| (row[tag].to_string(), row[value].to_string()))
        .collect()
}

/// Player mail is literal text; auction mail carries a GlobalStrings tag and item name.
pub fn subject(mail: &MailHeader) -> String {
    if mail.from_player || mail.sender != "Auction House" {
        return mail.subject.clone();
    }
    match auction_subject_parts(mail) {
        Ok((tag, item)) => text(tag).replace("%s", item),
        Err(error) => {
            eprintln!("{error}");
            error
        }
    }
}

fn auction_subject_parts(mail: &MailHeader) -> Result<(&str, &str), String> {
    let parts = mail.subject.split_once(':').filter(|(tag, _)| {
        matches!(
            *tag,
            "AUCTION_SOLD_MAIL_SUBJECT"
                | "AUCTION_WON_MAIL_SUBJECT"
                | "AUCTION_EXPIRED_MAIL_SUBJECT"
                | "AUCTION_OUTBID_MAIL_SUBJECT"
                | "AUCTION_REMOVED_MAIL_SUBJECT"
        )
    });
    parts.ok_or_else(|| format!("Unsupported auction mail subject (mail {})", mail.mail_id))
}

pub fn invoice(mail: &MailHeader) -> Result<Option<AuctionInvoiceView>, String> {
    if mail.from_player || mail.sender != "Auction House" {
        return Ok(None);
    }
    let (tag, item) = auction_subject_parts(mail)?;
    let seller = match tag {
        "AUCTION_SOLD_MAIL_SUBJECT" => true,
        "AUCTION_WON_MAIL_SUBJECT" => false,
        _ => return Ok(None),
    };
    let (player, amounts) = parse_invoice_body(&mail.body, seller)?;
    Ok(Some(invoice_view(item, player, amounts)))
}

fn parse_invoice_body(body: &str, seller: bool) -> Result<(&str, InvoiceAmounts), String> {
    let fields: Vec<&str> = body.split(':').collect();
    let [player, bid, buyout, deposit, cut, count] = fields.as_slice() else {
        return Err("Invalid auction invoice field count".into());
    };
    let parse = |value: &str| {
        value
            .parse::<u64>()
            .map_err(|error| format!("Invalid auction invoice number {value}: {error}"))
    };
    let bid = parse(bid)?;
    let buyout = parse(buyout)?;
    let deposit = parse(deposit)?;
    let cut = parse(cut)?;
    let count = parse(count)?;
    if count == 0 || cut > bid || bid.checked_add(deposit).is_none() {
        return Err("Invalid auction invoice amounts".into());
    }
    Ok((
        player,
        InvoiceAmounts {
            seller,
            bid,
            buyout,
            deposit,
            cut,
            count,
        },
    ))
}

struct InvoiceAmounts {
    seller: bool,
    bid: u64,
    buyout: u64,
    deposit: u64,
    cut: u64,
    count: u64,
}

fn invoice_view(item: &str, player: &str, amounts: InvoiceAmounts) -> AuctionInvoiceView {
    let InvoiceAmounts {
        seller,
        bid,
        buyout,
        deposit,
        cut,
        count,
    } = amounts;
    let item = invoice_item_name(item, count);
    let (item_tag, player_tag, amount_tag, multiple_tag) = invoice_tags(seller);
    let player = if player.is_empty() {
        text(multiple_tag)
    } else {
        player
    };
    AuctionInvoiceView {
        item_label: format!("{} {item}", text(item_tag)),
        player_label: format!("{} {player}", text(player_tag)),
        amount_label: text(amount_tag).into(),
        amount: if seller { bid + deposit - cut } else { bid },
        sale_price: seller.then_some(bid),
        buyout,
        deposit,
        house_cut: cut,
        count,
    }
}

fn invoice_item_name(item: &str, count: u64) -> String {
    if count > 1 {
        text("AUCTION_MAIL_ITEM_STACK")
            .replacen("%s", item, 1)
            .replace("%d", &count.to_string())
    } else {
        item.to_string()
    }
}

fn invoice_tags(seller: bool) -> (&'static str, &'static str, &'static str, &'static str) {
    if seller {
        (
            "ITEM_SOLD_COLON",
            "PURCHASED_BY_COLON",
            "AMOUNT_RECEIVED_COLON",
            "AUCTION_HOUSE_MAIL_MULTIPLE_BUYERS",
        )
    } else {
        (
            "ITEM_PURCHASED_COLON",
            "SOLD_BY_COLON",
            "AMOUNT_PAID_COLON",
            "AUCTION_HOUSE_MAIL_MULTIPLE_SELLERS",
        )
    }
}
