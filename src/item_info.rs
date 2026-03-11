use serde::{Deserialize, Serialize};
use std::io::BufRead;
use std::path::Path;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ItemInfoQuery {
    pub item_id: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ItemStaticInfo {
    pub item_id: u32,
    pub name: String,
    pub quality: u8,
    pub item_level: u16,
    pub required_level: u16,
    pub inventory_type: u8,
    pub sell_price: u32,
    pub stackable: u32,
    pub bonding: u8,
    pub expansion_id: u8,
}

pub fn lookup_item_info(item_id: u32) -> Result<Option<ItemStaticInfo>, String> {
    let path = Path::new("/syncthing/Sync/Projects/wow/wow-ui-sim/data/items.rs");
    let file =
        std::fs::File::open(path).map_err(|e| format!("failed to open {}: {e}", path.display()))?;
    let reader = std::io::BufReader::new(file);
    lookup_item_info_in_reader(reader, item_id)
}

fn lookup_item_info_in_reader<R: BufRead>(
    reader: R,
    item_id: u32,
) -> Result<Option<ItemStaticInfo>, String> {
    for line in reader.lines() {
        let line = line.map_err(|e| format!("failed to read item db line: {e}"))?;
        if let Some(item) = parse_item_line(&line)
            && item.item_id == item_id
        {
            return Ok(Some(item));
        }
    }
    Ok(None)
}

fn parse_item_line(line: &str) -> Option<ItemStaticInfo> {
    let trimmed = line.trim();
    if !trimmed.starts_with('(') || !trimmed.contains("ItemInfo {") {
        return None;
    }

    let item_id_end = trimmed.find(',')?;
    let item_id = trimmed[1..item_id_end].parse().ok()?;
    let name = extract_string_field(trimmed, "name")?;
    let quality = extract_number_field(trimmed, "quality")?;
    let item_level = extract_number_field(trimmed, "item_level")?;
    let required_level = extract_number_field(trimmed, "required_level")?;
    let inventory_type = extract_number_field(trimmed, "inventory_type")?;
    let sell_price = extract_number_field(trimmed, "sell_price")?;
    let stackable = extract_number_field(trimmed, "stackable")?;
    let bonding = extract_number_field(trimmed, "bonding")?;
    let expansion_id = extract_number_field(trimmed, "expansion_id")?;

    Some(ItemStaticInfo {
        item_id,
        name,
        quality,
        item_level,
        required_level,
        inventory_type,
        sell_price,
        stackable,
        bonding,
        expansion_id,
    })
}

fn extract_string_field(line: &str, field: &str) -> Option<String> {
    let needle = format!("{field}: \"");
    let start = line.find(&needle)? + needle.len();
    let rest = &line[start..];
    let end = rest.find('"')?;
    Some(rest[..end].to_string())
}

fn extract_number_field<T: std::str::FromStr>(line: &str, field: &str) -> Option<T> {
    let needle = format!("{field}: ");
    let start = line.find(&needle)? + needle.len();
    let rest = &line[start..];
    let end = rest.find([',', '}'])?;
    rest[..end].trim().parse().ok()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_item_info_line() {
        let line = r#"        (2589, ItemInfo { name: "Linen Cloth", quality: 1, item_level: 5, required_level: 1, inventory_type: 0, sell_price: 13, stackable: 200, bonding: 0, expansion_id: 0 }),"#;

        let item = parse_item_line(line).expect("line should parse");

        assert_eq!(
            item,
            ItemStaticInfo {
                item_id: 2589,
                name: "Linen Cloth".into(),
                quality: 1,
                item_level: 5,
                required_level: 1,
                inventory_type: 0,
                sell_price: 13,
                stackable: 200,
                bonding: 0,
                expansion_id: 0,
            }
        );
    }

    #[test]
    fn lookup_item_info_finds_item_in_reader() {
        let data = br#"
        (2589, ItemInfo { name: "Linen Cloth", quality: 1, item_level: 5, required_level: 1, inventory_type: 0, sell_price: 13, stackable: 200, bonding: 0, expansion_id: 0 }),
        (2447, ItemInfo { name: "Peacebloom", quality: 1, item_level: 5, required_level: 1, inventory_type: 0, sell_price: 10, stackable: 200, bonding: 0, expansion_id: 0 }),
        "#;

        let item = lookup_item_info_in_reader(std::io::Cursor::new(data), 2447)
            .expect("lookup should succeed")
            .expect("item should exist");

        assert_eq!(item.name, "Peacebloom");
        assert_eq!(item.sell_price, 10);
    }
}
