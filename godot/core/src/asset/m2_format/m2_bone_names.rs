/// Map WoW M2 key_bone_id to a human-readable name.
pub fn key_bone_name(id: i32) -> &'static str {
    upper_body_key_bone_name(id)
        .or_else(|| hand_key_bone_name(id))
        .or_else(|| misc_key_bone_name(id))
        .unwrap_or("bone")
}

fn upper_body_key_bone_name(id: i32) -> Option<&'static str> {
    match id {
        0 => Some("ArmL"),
        1 => Some("ArmR"),
        2 => Some("ShoulderL"),
        3 => Some("ShoulderR"),
        4 => Some("SpineLow"),
        5 => Some("Waist"),
        6 => Some("Head"),
        7 => Some("Jaw"),
        _ => None,
    }
}

fn hand_key_bone_name(id: i32) -> Option<&'static str> {
    match id {
        8 => Some("IndexFingerR"),
        9 => Some("MiddleFingerR"),
        10 => Some("PinkyFingerR"),
        11 => Some("RingFingerR"),
        12 => Some("ThumbR"),
        13 => Some("IndexFingerL"),
        14 => Some("MiddleFingerL"),
        15 => Some("PinkyFingerL"),
        16 => Some("RingFingerL"),
        17 => Some("ThumbL"),
        _ => None,
    }
}

fn misc_key_bone_name(id: i32) -> Option<&'static str> {
    match id {
        18 => Some("BTH"),
        19 => Some("CSR"),
        20 => Some("CSL"),
        21 => Some("Breath"),
        22 => Some("Root"),
        23 => Some("Knee"),
        24 => Some("FootL"),
        25 => Some("FootR"),
        26 => Some("ElbowL"),
        27 => Some("ElbowR"),
        28 => Some("KneeR"),
        29 => Some("KneeL"),
        30 => Some("WHL"),
        31 => Some("WHR"),
        _ => None,
    }
}

/// Display name for a bone: known key_bone_id names, key-based ID for unknown
/// positive IDs, or "Bone[index]" for bones with no key (-1).
pub fn bone_display_name(key_bone_id: i32, index: usize) -> String {
    let name = key_bone_name(key_bone_id);
    if name != "bone" {
        return name.to_string();
    }
    if key_bone_id >= 0 {
        format!("KeyBone{key_bone_id}")
    } else {
        format!("Bone[{index}]")
    }
}

#[cfg(test)]
#[path = "../../../tests/unit/asset/m2_bone_names_tests.rs"]
mod tests;
