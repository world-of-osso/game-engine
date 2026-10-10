use game_engine_core::spell_catalog::{CatalogSpell, SpellCatalogData};
use game_engine_ui_model::spell_overrides::resolve_action;
use shared::{
    components::{AuraOverride, AuraView},
    protocol::ActionRef,
};

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
