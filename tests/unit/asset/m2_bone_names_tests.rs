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
fn bone_display_name_unknown_negative() {
    assert_eq!(bone_display_name(-1, 5), "Bone[5]");
    assert_eq!(bone_display_name(-1, 0), "Bone[0]");
}

#[test]
fn bone_display_name_unknown_positive_uses_key_id() {
    assert_eq!(bone_display_name(48, 9), "KeyBone48");
    assert_eq!(bone_display_name(55, 5), "KeyBone55");
}
