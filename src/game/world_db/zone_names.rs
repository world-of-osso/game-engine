use std::collections::HashMap;
use std::sync::OnceLock;

/// Retail GetZoneText: the top-level AreaTable ancestor of the area under the player
/// (Northshire Valley 9 → Northshire 6170 → Elwynn Forest 12).
pub fn zone_of_area(area_id: u32) -> u32 {
    root_area(area_parents(), area_id)
}

fn area_parents() -> &'static HashMap<u32, u32> {
    static PARENTS: OnceLock<HashMap<u32, u32>> = OnceLock::new();
    PARENTS.get_or_init(|| {
        game_engine::world_db::load_area_parents().unwrap_or_else(|err| {
            bevy::log::error!("AreaTable parents unavailable: {err}");
            HashMap::new()
        })
    })
}

/// Follows parents to the root; AreaTable has no cycles, the bound only stops bad data.
fn root_area(parents: &HashMap<u32, u32>, area_id: u32) -> u32 {
    let mut id = area_id;
    for _ in 0..16 {
        match parents.get(&id) {
            Some(&parent) => id = parent,
            None => break,
        }
    }
    id
}

pub fn zone_id_to_name(id: u32) -> String {
    game_engine::world_db::load_zone_name(id)
        .ok()
        .flatten()
        .unwrap_or_else(|| "Unknown".to_string())
}

#[cfg(test)]
mod tests {
    use super::{zone_id_to_name, zone_of_area};

    #[test]
    fn zone_id_to_name_known() {
        game_engine::world_db::import_zone_name_cache().expect("import zone name cache");
        assert_eq!(zone_id_to_name(12), "Elwynn Forest");
        assert_eq!(zone_id_to_name(1519), "Stormwind City");
    }

    #[test]
    fn northshire_areas_belong_to_elwynn_forest() {
        assert_eq!(zone_of_area(9), 12, "Northshire Valley");
        assert_eq!(zone_of_area(24), 12, "Northshire Abbey");
        assert_eq!(zone_of_area(87), 12, "Goldshire");
        assert_eq!(zone_of_area(12), 12, "Elwynn Forest");
    }

    #[test]
    fn zone_id_to_name_unknown() {
        assert_eq!(zone_id_to_name(99999), "Unknown");
    }
}
