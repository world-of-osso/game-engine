use game_engine_core::adventure_guide::Catalog;
use game_engine_ui_model::adventure_guide::{Page, Tab, View};
use std::sync::Arc;
fn journal() -> View {
    let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../data/cache/adventure_guide.sqlite");
    View {
        catalog: Arc::new(
            Catalog::load(&path).expect("Import pinned local journal cache before testing"),
        ),
        ..Default::default()
    }
}
#[test]
fn authentic_dungeon_sections_and_loot_are_selectable() {
    let mut view = journal();
    view.apply("journal:instance:67").unwrap();
    view.apply("journal:encounter:112").unwrap();
    assert_eq!(view.tab, Tab::Dungeons);
    assert_eq!(view.catalog.encounter(112).unwrap().name, "Ozruk");
    view.apply("journal:page:abilities").unwrap();
    assert_eq!(view.page, Page::Abilities);
    let sections = view.sections().unwrap();
    assert!(!sections.is_empty());
    assert!(sections.iter().any(|(s, _)| s.title.contains("Shatter")));
    let section = sections[0].0.id;
    view.apply(&format!("journal:section:{section}")).unwrap();
    assert!(view.collapsed.contains(&section));
    view.apply("journal:page:loot").unwrap();
    let loot = view.loot();
    assert!(!loot.is_empty());
    assert!(loot.iter().all(|i| !i.name.is_empty()));
    let slot = loot[0].slot;
    view.apply(&format!("journal:slot:{slot}")).unwrap();
    assert!(view.loot().iter().all(|i| i.slot == slot));
}
#[test]
fn raid_selection_search_and_back_preserve_authored_identity() {
    let mut view = journal();
    view.apply("journal:tab:raids").unwrap();
    view.apply("journal:tier:68").unwrap();
    view.search = "Molten".into();
    assert_eq!(
        view.instances().iter().map(|i| i.id).collect::<Vec<_>>(),
        vec![741]
    );
    view.apply("journal:encounter:1519").unwrap();
    assert_eq!(view.instance, Some(741));
    assert_eq!(view.catalog.display(1519), Some(13031));
    view.apply("journal:back").unwrap();
    assert_eq!(view.encounter, None);
    assert_eq!(view.search, "");
}
