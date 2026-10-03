use std::path::Path;

pub(super) fn resolve_cached_outfit_display_ids(
    data_dir: &Path,
    race: u8,
    class: u8,
    sex: u8,
) -> Result<Vec<u32>, String> {
    let cache_path = super::imported_outfit_links_cache_path(data_dir)?;
    let conn = super::open_read_only(&cache_path)?;
    let mut stmt = conn
        .prepare(
            "SELECT iam.display_info_id
             FROM starter_outfits so
             JOIN item_modified_appearance_map ima ON ima.item_id = so.item_id
             JOIN item_appearance_map iam ON iam.appearance_id = ima.appearance_id
             WHERE so.race_id = ?1 AND so.class_id = ?2 AND so.sex_id = ?3
             ORDER BY so.item_order",
        )
        .map_err(|err| format!("prepare starter outfit display lookup: {err}"))?;
    let rows = stmt
        .query_map((race, class, sex), |row| row.get::<_, u32>(0))
        .map_err(|err| format!("query starter outfit display ids: {err}"))?;
    let mut display_ids = Vec::new();
    for row in rows {
        display_ids.push(row.map_err(|err| format!("read starter outfit display row: {err}"))?);
    }
    Ok(display_ids)
}

#[cfg(test)]
mod tests {
    use std::path::Path;

    #[test]
    fn resolve_outfit_display_ids_loads_from_cache() {
        let data_dir = Path::new("data");
        super::super::import_outfit_links_cache(data_dir).expect("import outfit links cache");
        let display_ids = super::resolve_cached_outfit_display_ids(data_dir, 1, 1, 0)
            .expect("resolve outfit displays");
        assert!(
            !display_ids.is_empty(),
            "starter outfit display lookup should not be empty"
        );
    }
}
