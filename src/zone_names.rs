pub fn zone_id_to_name(id: u32) -> String {
    game_engine::world_db::load_zone_name(id)
        .ok()
        .flatten()
        .unwrap_or_else(|| "Unknown".to_string())
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
