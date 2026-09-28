use crate::footstep_data::FootstepSurface;
use crate::ground_effect_data::{
    GroundEffectEntry, classify_surface_from_terrain_sound_name, parse_ground_effect_entries,
    parse_terrain_type_sounds,
};

fn wdc5(layout: u32, record_size: u32, records: &[u8], strings: &[u8], ids: &[u32]) -> Vec<u8> {
    let header = 136usize;
    let section = header + 68;
    let file = section + 40;
    let mut bytes = vec![0; file + records.len() + strings.len() + ids.len() * 4];
    bytes[..4].copy_from_slice(b"WDC5");
    bytes[header..header + 4].copy_from_slice(&(ids.len() as u32).to_le_bytes());
    bytes[header + 8..header + 12].copy_from_slice(&record_size.to_le_bytes());
    bytes[header + 12..header + 16].copy_from_slice(&(strings.len() as u32).to_le_bytes());
    bytes[header + 20..header + 24].copy_from_slice(&layout.to_le_bytes());
    bytes[section + 8..section + 12].copy_from_slice(&(file as u32).to_le_bytes());
    bytes[section + 16..section + 20].copy_from_slice(&(strings.len() as u32).to_le_bytes());
    bytes[file..file + records.len()].copy_from_slice(records);
    bytes[file + records.len()..file + records.len() + strings.len()].copy_from_slice(strings);
    for (index, id) in ids.iter().enumerate() {
        let offset = file + records.len() + strings.len() + index * 4;
        bytes[offset..offset + 4].copy_from_slice(&id.to_le_bytes());
    }
    bytes
}

#[test]
fn ground_effect_accepts_both_layouts_and_reads_rows_by_id() {
    for layout in [0xD93D_5678, 0x3DEC_72D8] {
        let bytes = wdc5(layout, 5, &[7, 0, 0, 0, 3, 9, 0, 0, 0, 6], &[], &[42, 100]);
        let rows = parse_ground_effect_entries(&bytes).unwrap();
        assert_eq!(rows.len(), 2);
        assert_eq!(
            rows[&42],
            GroundEffectEntry {
                effect_id: 42,
                density: 7,
                terrain_sound_id: 3
            }
        );
        assert_eq!(rows[&100].density, 9);
        assert_eq!(rows[&100].terrain_sound_id, 6);
    }
}

#[test]
fn terrain_sounds_accepts_all_layouts_and_skips_empty_strings() {
    for layout in [0xB99F_5777, 0x5462_668A, 0x3AF6_B1EA] {
        let bytes = wdc5(layout, 4, &[0; 8], b"Stone\0\0Mud / Lava\0", &[2, 5]);
        let names = parse_terrain_type_sounds(&bytes).unwrap();
        assert_eq!(names[&2], "Stone");
        assert_eq!(names[&5], "Mud / Lava");
    }
}

#[test]
fn parser_rejects_invalid_layout_record_size_and_string_data() {
    let wrong_layout = wdc5(0, 5, &[7, 0, 0, 0, 3], &[], &[42]);
    assert!(
        parse_ground_effect_entries(&wrong_layout)
            .unwrap_err()
            .contains("layout hash")
    );
    let wrong_size = wdc5(0xD93D_5678, 4, &[0; 4], &[], &[42]);
    assert!(
        parse_ground_effect_entries(&wrong_size)
            .unwrap_err()
            .contains("record size")
    );
    let bad_utf8 = wdc5(0xB99F_5777, 4, &[0; 4], &[0xff, 0], &[2]);
    assert!(
        parse_terrain_type_sounds(&bad_utf8)
            .unwrap_err()
            .contains("invalid UTF-8")
    );
    let missing_name = wdc5(0xB99F_5777, 4, &[0; 4], b"\0", &[2]);
    assert!(
        parse_terrain_type_sounds(&missing_name)
            .unwrap_err()
            .contains("string count")
    );
    let large_id = wdc5(0xB99F_5777, 4, &[0; 4], b"Stone\0", &[256]);
    assert!(
        parse_terrain_type_sounds(&large_id)
            .unwrap_err()
            .contains("does not fit in u8")
    );
}

#[test]
fn classifier_uses_ordered_case_insensitive_keywords() {
    assert_eq!(
        classify_surface_from_terrain_sound_name("METAL wood"),
        Some(FootstepSurface::Metal)
    );
    assert_eq!(
        classify_surface_from_terrain_sound_name("Stone sand"),
        Some(FootstepSurface::Stone)
    );
    assert_eq!(
        classify_surface_from_terrain_sound_name("Twiggy"),
        Some(FootstepSurface::Grass)
    );
    assert_eq!(
        classify_surface_from_terrain_sound_name("Mud / Lava"),
        Some(FootstepSurface::Mud)
    );
    assert_eq!(classify_surface_from_terrain_sound_name("Unknown"), None);
}
