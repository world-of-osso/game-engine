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
fn soft_interact_is_on_by_default_and_for_files_saved_before_it() {
    // User decision 2026-10-02: Enable Interact Key on, unlike Retail (gamepad only).
    let file = ClientOptionsFile::default();
    assert!(file.hud.soft_target_interact);
    let saved = ron::ser::to_string(&file)
        .unwrap()
        .replace("softTargetInteract:true,", "");
    assert!(!saved.contains("softTargetInteract"));
    let restored: ClientOptionsFile = ron::de::from_str(&saved).unwrap();
    assert!(restored.hud.soft_target_interact);
}
