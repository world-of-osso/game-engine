use crate::client_options_data::ClientOptionsFile;
use crate::input_bindings_data::{BindingKey, InputAction, InputBinding};
use crate::realm_preset_data::RealmPreset;

#[test]
fn full_options_round_trip_keeps_custom_values() {
    let mut file = ClientOptionsFile::default();
    file.accepted_eula = true;
    file.preferred_realm = RealmPreset::Prod;
    file.sound.master_volume = 0.23;
    file.camera.mouse_sensitivity = 0.008;
    file.camera.invert_y = true;
    file.graphics.anti_alias = crate::client_options_data::AntiAliasMode::Taa;
    file.graphics.ssao_enabled = true;
    file.hud.nameplate_style.health_colors.neutral = [0.2, 0.5, 0.8];
    file.hud.auto_loot = true;
    file.bindings.assign(
        InputAction::TargetNearest,
        InputBinding::Keyboard(BindingKey::F5),
    );
    file.modal_offset = Some([13.0, -9.0]);
    file.modal_position = Some([44.0, 55.0]);
    let serialized = ron::ser::to_string(&file).unwrap();
    let restored: ClientOptionsFile = ron::de::from_str(&serialized).unwrap();
    assert!(restored.accepted_eula);
    assert_eq!(restored.preferred_realm, RealmPreset::Prod);
    assert_eq!(restored.sound.master_volume, 0.23);
    assert_eq!(restored.camera.mouse_sensitivity, 0.008);
    assert!(restored.camera.invert_y);
    assert_eq!(restored.graphics.anti_alias, file.graphics.anti_alias);
    assert!(restored.graphics.ssao_enabled);
    assert_eq!(
        restored.hud.nameplate_style.health_colors.neutral,
        [0.2, 0.5, 0.8]
    );
    assert!(restored.hud.auto_loot);
    assert_eq!(
        restored.bindings.binding(InputAction::TargetNearest),
        Some(InputBinding::Keyboard(BindingKey::F5))
    );
    assert_eq!(restored.modal_offset, Some([13.0, -9.0]));
    assert_eq!(restored.modal_position, Some([44.0, 55.0]));
}

#[test]
fn sensitivity_clamps_without_erasing_unrelated_sections() {
    let mut file = ClientOptionsFile::default();
    file.camera.mouse_sensitivity = 0.8;
    file.hud.auto_loot = true;
    let effective = file.clamped();
    assert_eq!(effective.camera.mouse_sensitivity, 0.01);
    assert!(effective.hud.auto_loot);
}

#[test]
fn invalid_graphics_and_invalid_sound_fail() {
    let invalid: ClientOptionsFile =
        ron::de::from_str("(graphics:(antiAlias:Msaa4x,ssaoEnabled:true,))").unwrap();
    assert!(invalid.validate().unwrap_err().contains("SSAO"));
    assert!(ron::de::from_str::<ClientOptionsFile>("(sound:(master_volume:\"loud\",))").is_err());
}

#[test]
fn soft_interact_settings_default_to_retail_and_persist() {
    use crate::client_options_data::{InteractKeyIcons, SoftTargetArc};
    let file = ClientOptionsFile::default();
    assert!(!file.hud.soft_target_interact);
    let soft = file.hud.soft_target;
    assert_eq!(soft.interact_arc, SoftTargetArc::DirectlyInFront);
    assert_eq!(soft.interact_range, 10.0);
    assert_eq!(soft.interact_key_icons(), InteractKeyIcons::Default);
    // A file saved before these settings existed reads Retail's defaults.
    let old: ClientOptionsFile = ron::de::from_str("(hud:(show_minimap:true,show_action_bars:true,show_nameplates:true,show_health_bars:true,show_target_marker:true,show_fps_overlay:false))").unwrap();
    assert_eq!(old.hud.soft_target, soft);
    // The Kiosk gamepad preset values (Blizzard_Gamepad/Core.lua:20-33) round-trip.
    let mut file = file;
    file.hud.soft_target.interact_arc = SoftTargetArc::InFront;
    file.hud.soft_target.interact_range = 20.0;
    let saved = ron::ser::to_string(&file).unwrap();
    assert!(saved.contains("softTargetInteractArc:1"), "{saved}");
    let restored: ClientOptionsFile = ron::de::from_str(&saved).unwrap();
    assert_eq!(
        restored.hud.soft_target.interact_arc,
        SoftTargetArc::InFront
    );
    assert_eq!(restored.hud.soft_target.interact_range, 20.0);
    let bad = saved.replace("softTargetInteractArc:1", "softTargetInteractArc:3");
    assert!(ron::de::from_str::<ClientOptionsFile>(&bad).is_err());
    let mut negative = restored;
    negative.hud.soft_target.interact_range = -1.0;
    assert!(negative.validate().is_err());
}

#[test]
fn interact_key_icons_dropdown_sets_and_reads_the_icon_cvars() {
    use crate::client_options_data::{InteractKeyIcons, SoftTargetOptions};
    let mut soft = SoftTargetOptions::default();
    soft.set_interact_key_icons(InteractKeyIcons::ShowAll);
    assert!(
        soft.icon_enemy && soft.icon_interact && soft.icon_game_object && soft.low_priority_icons
    );
    assert_eq!(soft.interact_key_icons(), InteractKeyIcons::ShowAll);
    soft.set_interact_key_icons(InteractKeyIcons::ShowNone);
    assert!(
        !soft.icon_enemy
            && !soft.icon_interact
            && !soft.icon_game_object
            && !soft.low_priority_icons
    );
    assert_eq!(soft.interact_key_icons(), InteractKeyIcons::ShowNone);
    soft.set_interact_key_icons(InteractKeyIcons::Default);
    assert_eq!(soft, SoftTargetOptions::default());
    // Any other mix reads as the default choice (GetValue's else branch).
    soft.icon_game_object = true;
    assert_eq!(soft.interact_key_icons(), InteractKeyIcons::Default);
    assert_eq!(
        InteractKeyIcons::from_value(2),
        Some(InteractKeyIcons::ShowAll)
    );
    assert_eq!(InteractKeyIcons::from_value(4), None);
}
