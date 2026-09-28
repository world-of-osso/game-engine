use rusqlite::{Connection, OpenFlags};

#[path = "npc_appearance_data.rs"]
mod npc_appearance_data;
pub use npc_appearance_data::{AuthoredNpcAppearance, query_authored_npc_appearance};

pub fn load_authored_npc_appearance(
    display_id: u32,
) -> Result<Option<AuthoredNpcAppearance>, String> {
    let path = game_engine::paths::resolve_data_path("cache/npc_appearance.sqlite");
    let connection =
        Connection::open_with_flags(&path, OpenFlags::SQLITE_OPEN_READ_ONLY).map_err(|error| {
            format!(
                "open authored NPC appearance cache {} for display {display_id}: {error}",
                path.display()
            )
        })?;
    query_authored_npc_appearance(&connection, display_id)
}
