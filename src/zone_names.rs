pub fn zone_id_to_name(id: u32) -> &'static str {
    match id {
        10 => "Duskwood",
        12 => "Elwynn Forest",
        14 => "Durotar",
        17 => "The Barrens",
        38 => "Loch Modan",
        40 => "Westfall",
        44 => "Redridge Mountains",
        85 => "Tirisfal Glades",
        215 => "Mulgore",
        331 => "Ashenvale",
        1497 => "Undercity",
        1519 => "Stormwind City",
        1537 => "Ironforge",
        1637 => "Orgrimmar",
        1638 => "Thunder Bluff",
        1657 => "Darnassus",
        _ => "Unknown",
    }
}

#[cfg(test)]
mod tests {
    use super::zone_id_to_name;

    #[test]
    fn zone_id_to_name_known() {
        assert_eq!(zone_id_to_name(12), "Elwynn Forest");
        assert_eq!(zone_id_to_name(1519), "Stormwind City");
    }

    #[test]
    fn zone_id_to_name_unknown() {
        assert_eq!(zone_id_to_name(99999), "Unknown");
    }
}
