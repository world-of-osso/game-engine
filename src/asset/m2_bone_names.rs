/// Map WoW M2 key_bone_id to a human-readable name.
pub fn key_bone_name(id: i32) -> &'static str {
    match id {
        0 => "ArmL",
        1 => "ArmR",
        2 => "ShoulderL",
        3 => "ShoulderR",
        4 => "SpineLow",
        5 => "Waist",
        6 => "Head",
        7 => "Jaw",
        8 => "IndexFingerR",
        9 => "MiddleFingerR",
        10 => "PinkyFingerR",
        11 => "RingFingerR",
        12 => "ThumbR",
        13 => "IndexFingerL",
        14 => "MiddleFingerL",
        15 => "PinkyFingerL",
        16 => "RingFingerL",
        17 => "ThumbL",
        18 => "BTH",
        19 => "CSR",
        20 => "CSL",
        21 => "Breath",
        22 => "Root",
        23 => "Knee",
        24 => "FootL",
        25 => "FootR",
        26 => "ElbowL",
        27 => "ElbowR",
        28 => "KneeR",
        29 => "KneeL",
        30 => "WHL",
        31 => "WHR",
        _ => "bone",
    }
}

/// Display name for a bone: known key_bone_id names, or "Bone[index]" for unknown.
pub fn bone_display_name(key_bone_id: i32, index: usize) -> String {
    let name = key_bone_name(key_bone_id);
    if name == "bone" {
        format!("Bone[{index}]")
    } else {
        name.to_string()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn known_key_bone_ids() {
        assert_eq!(key_bone_name(22), "Root");
        assert_eq!(key_bone_name(6), "Head");
        assert_eq!(key_bone_name(0), "ArmL");
    }

    #[test]
    fn unknown_key_bone_id() {
        assert_eq!(key_bone_name(-1), "bone");
        assert_eq!(key_bone_name(999), "bone");
    }

    #[test]
    fn bone_display_name_known() {
        assert_eq!(bone_display_name(6, 3), "Head");
        assert_eq!(bone_display_name(22, 0), "Root");
    }

    #[test]
    fn bone_display_name_unknown() {
        assert_eq!(bone_display_name(-1, 5), "Bone[5]");
        assert_eq!(bone_display_name(-1, 0), "Bone[0]");
    }
}
