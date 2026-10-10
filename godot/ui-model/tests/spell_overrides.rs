use game_engine_core::spell_catalog::{CatalogSpell, SpellCatalogData};
use game_engine_ui_model::spell_overrides::resolve_action;
use shared::{
    components::{AuraOverride, AuraView},
    protocol::ActionRef,
};

#[test]
fn toyfx2_override_set_replaces_empty_spell_and_item_slots_then_restores() {
    use game_engine_ui_model::spell_overrides::resolve_action_slot;
    // Actual Golden Hearthstone Card119211, spell176889, OverrideSpellData785.
    let aura = AuraView {
        overrides: vec![AuraOverride::SpellSet {
            id: 785,
            spells: vec![225656, 225658, 225660, 225657, 225659, 0, 0, 0, 0, 0],
        }],
        instance_id: 1,
        spell_id: 176889,
        caster: Some(42),
        stacks: 1,
        charges: 0,
        duration_ms: 300000,
        remaining_ms: 300000,
        harmful: false,
        dispel_type: 0,
        flags: 0,
    };
    for stored in [
        None,
        Some(ActionRef::Spell(133)),
        Some(ActionRef::Item(18660)),
    ] {
        assert_eq!(
            resolve_action_slot(0, stored, &[aura.clone()]),
            Some(ActionRef::Spell(225656))
        );
        assert_eq!(resolve_action_slot(0, stored, &[]), stored);
        assert_eq!(resolve_action_slot(5, stored, &[aura.clone()]), None);
        assert_eq!(resolve_action_slot(6, stored, &[aura.clone()]), None);
        assert_eq!(resolve_action_slot(12, stored, &[aura.clone()]), stored);
    }
}

#[test]
fn override_applies_and_restores_action_button_without_rebinding() {
    let base = ActionRef::Spell(85288);
    let catalog = SpellCatalogData::from_parts(
        vec![
            CatalogSpell {
                id: 85288,
                name: "Raging Blow".into(),
                icon_fdid: 132352,
                ..Default::default()
            },
            CatalogSpell {
                id: 335097,
                name: "Crushing Blow".into(),
                icon_fdid: 236317,
                ..Default::default()
            },
        ],
        Default::default(),
    );
    let aura = AuraView {
        overrides: vec![AuraOverride::ActionBar {
            spell_id: 85288,
            replacement: 335097,
        }],
        instance_id: 1,
        spell_id: 1719,
        caster: Some(42),
        stacks: 1,
        charges: 0,
        duration_ms: 12000,
        remaining_ms: 12000,
        harmful: false,
        dispel_type: 0,
        flags: 0,
    };
    for (auras, expected, name, icon) in [
        (vec![], 85288, "Raging Blow", 132352),
        (vec![aura], 335097, "Crushing Blow", 236317),
        (vec![], 85288, "Raging Blow", 132352),
    ] {
        let ActionRef::Spell(id) = resolve_action(base, &auras) else {
            panic!("spell action")
        };
        let spell = catalog.get(id).unwrap();
        assert_eq!(
            (id, spell.name.as_ref(), spell.icon_fdid),
            (expected, name, icon)
        );
        assert_eq!(base, ActionRef::Spell(85288));
    }
}

#[test]
fn animation_override_and_item_action_do_not_replace_spells() {
    let aura = AuraView {
        overrides: vec![AuraOverride::Animation(1013)],
        instance_id: 1,
        spell_id: 406732,
        caster: Some(42),
        stacks: 1,
        charges: 0,
        duration_ms: 10000,
        remaining_ms: 10000,
        harmful: false,
        dispel_type: 0,
        flags: 0,
    };
    assert_eq!(
        resolve_action(ActionRef::Spell(85288), &[aura.clone()]),
        ActionRef::Spell(85288)
    );
    assert_eq!(
        resolve_action(ActionRef::Item(6948), &[aura]),
        ActionRef::Item(6948)
    );
}

#[test]
fn spellbook_replacement_preserves_base_binding_and_presents_its_cooldown() {
    use game_engine_ui_model::{
        spell_overrides::update_spellbook_item, spellbook_frame_component::SpellbookItemView,
    };
    let base = SpellbookItemView {
        spell_id: 85288,
        name: "Raging Blow".into(),
        subtext: String::new(),
        icon_fdid: 132352,
        passive: false,
        available_at: None,
        cooldown_fraction: 0.0,
    };
    let replacement = CatalogSpell {
        id: 335097,
        name: "Crushing Blow".into(),
        icon_fdid: 236317,
        ..Default::default()
    };
    let mut active = base.clone();
    update_spellbook_item(&mut active, Some(&replacement), 0.75);
    assert_eq!(active.spell_id, 85288);
    assert_eq!(active.name, "Crushing Blow");
    assert_eq!(active.icon_fdid, 236317);
    assert_eq!(active.cooldown_fraction, 0.75);
    let mut restored = base.clone();
    update_spellbook_item(&mut restored, None, 0.0);
    assert_eq!(restored, base);
    let mut future = base;
    future.available_at = Some(10);
    update_spellbook_item(&mut future, Some(&replacement), 0.75);
    assert_eq!(future.name, "Raging Blow");
    assert_eq!(future.cooldown_fraction, 0.0);
}
