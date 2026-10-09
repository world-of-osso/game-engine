use game_engine_core::{
    blp,
    spell_catalog::{SpellCatalogPaths, load_spell_catalog},
};
use game_engine_ui_model::{
    spellbook_frame_component::PlayerSpellsTab, spellbook_preview::load_class_preview_state,
};
use std::collections::BTreeMap;

// Exceptions must name concrete spells and a witnessed local-CASC/DB2 blocker.
const ICON_EXCEPTIONS: &[(u32, &str)] = &[];

#[test]
fn every_class_spellbook_entry_has_nonempty_real_icon() {
    let data = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../data");
    let catalog = load_spell_catalog(&SpellCatalogPaths::for_data_dir(&data)).unwrap();
    // The user's concrete missing icon must resolve independently of the chosen build.
    let avenging = catalog.get(31884).unwrap();
    assert_eq!(avenging.icon_fdid, 135875);
    let avenging_bytes = std::fs::read(data.join("textures/135875.blp")).unwrap();
    let avenging_icon = blp::decode_rgba(&avenging_bytes).unwrap();
    assert!(avenging_icon.width > 0 && avenging_icon.height > 0);
    assert!(
        avenging_icon
            .pixels
            .chunks_exact(4)
            .any(|pixel| pixel[3] != 0)
    );
    let mut decoded = BTreeMap::<u32, Result<(), String>>::new();
    let mut missing = Vec::new();
    let mut total = 0;
    for class in 1..=13 {
        let (&spec, _) = catalog
            .tabs
            .specs
            .iter()
            .filter(|(_, s)| s.class_id == class && !s.initial)
            .min_by_key(|(id, s)| (s.order_index, **id))
            .unwrap();
        let state =
            load_class_preview_state(&data, PlayerSpellsTab::Spellbook, class, spec).unwrap();
        for item in state
            .categories
            .iter()
            .flat_map(|c| &c.groups)
            .flat_map(|g| &g.items)
        {
            total += 1;
            println!(
                "CLASSBOOK_ICON_ENTRY\t{class}\t{spec}\t{}\t{}\t{}\t{}",
                item.spell_id, item.icon_fdid, item.name, item.subtext
            );
            let result = decoded.entry(item.icon_fdid).or_insert_with(|| {
                let path = data.join(format!("textures/{}.blp", item.icon_fdid));
                let bytes = std::fs::read(&path).map_err(|e| format!("{}: {e}", path.display()))?;
                let image = blp::decode_rgba(&bytes)?;
                if image.width == 0
                    || image.height == 0
                    || !image
                        .pixels
                        .chunks_exact(4)
                        .any(|p| p[3] != 0 && p[..3].iter().any(|&v| v != 0))
                {
                    return Err(format!("FDID {} decoded an empty icon", item.icon_fdid));
                }
                Ok(())
            });
            if let Err(error) = result {
                if let Some((_, reason)) =
                    ICON_EXCEPTIONS.iter().find(|(id, _)| *id == item.spell_id)
                {
                    assert!(!reason.is_empty());
                    println!(
                        "CLASSBOOK_ICON_EXCEPTION\t{class}\t{}\t{reason}",
                        item.spell_id
                    );
                } else {
                    missing.push((class, item.spell_id, item.icon_fdid, error.clone()));
                }
            }
        }
    }
    println!("CLASSBOOK_ICON_TOTAL\t{total}\t{}", missing.len());
    assert!(
        missing.is_empty(),
        "{} unresolved spellbook icons, first cases: {:?}",
        missing.len(),
        &missing[..missing.len().min(12)]
    );
}
