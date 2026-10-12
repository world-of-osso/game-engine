use crate::GameClient;
use godot::prelude::*;

#[godot_api(secondary)]
impl GameClient {
    /// Read-only journal authority/preview observation for private live fixtures.
    #[func]
    fn mount_journal_state(&self) -> VarDictionary {
        let view = &self.mount_journal.view;
        let mut state = VarDictionary::new();
        state.set("catalog_count", view.catalog.len() as i64);
        state.set("open", view.visible);
        state.set("error", view.error.as_str());
        state.set("selected", view.selected.map_or(0, i64::from));
        state.set("preview_ready", self.mount_journal.preview_ready());
        state.set("yaw", self.mount_journal.preview_yaw());
        let mut collected = Array::<VarDictionary>::new();
        for mount in view.catalog.values().filter(|mount| mount.collected) {
            let mut row = VarDictionary::new();
            row.set("mount_id", i64::from(mount.mount_id));
            row.set("spell_id", i64::from(mount.spell_id));
            row.set("name", mount.name.as_str());
            row.set("favourite", mount.favourite);
            row.set("active", mount.active);
            row.set("usable", mount.usable);
            collected.push(&row);
        }
        state.set("collected", &collected);
        state
    }
}
