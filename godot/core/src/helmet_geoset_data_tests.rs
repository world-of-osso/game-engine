use std::collections::HashMap;
use std::path::Path;

use crate::helmet_geoset_data::{HelmetGeosetRule, load_helmet_geoset_rules};

/// FNV-1a over every visibility ID and its rules in authored row order.
fn fingerprint(rules: &HashMap<u32, Vec<HelmetGeosetRule>>) -> u64 {
    let mut ids: Vec<_> = rules.keys().copied().collect();
    ids.sort_unstable();
    let mut hash = 0xcbf2_9ce4_8422_2325u64;
    let mut mix = |value: u32| {
        for byte in value.to_le_bytes() {
            hash = (hash ^ u64::from(byte)).wrapping_mul(0x100_0000_01b3);
        }
    };
    for id in ids {
        mix(id);
        for rule in &rules[&id] {
            mix(rule.race_id.into());
            mix(rule.race_bit_selection);
            mix(rule.hide_geoset_group.into());
        }
    }
    hash
}

#[test]
fn authored_helmet_rules_group_every_row_by_visibility_id() {
    let data_root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../data");
    let started = std::time::Instant::now();
    let rules = load_helmet_geoset_rules(&data_root).unwrap();
    // Loaded for every character preview; the quadratic parser took ~4 s. The bound
    // catches that regression with room for a loaded machine running the whole suite in
    // parallel (a linear parse measured 545 ms there against a 500 ms bound).
    let elapsed = started.elapsed();
    assert!(elapsed.as_millis() < 2000, "helmet rules took {elapsed:?}");
    assert_eq!(rules.len(), 122);
    assert_eq!(rules.values().map(Vec::len).sum::<usize>(), 18302);
    assert_eq!(
        rules[&245],
        [
            HelmetGeosetRule {
                race_id: 75,
                race_bit_selection: 0,
                hide_geoset_group: 2
            },
            HelmetGeosetRule {
                race_id: 75,
                race_bit_selection: 0,
                hide_geoset_group: 44
            },
        ]
    );
    assert_eq!(fingerprint(&rules), 0xcfd6_8687_0f5a_f4d8);
}
