use rusqlite::{Connection, OpenFlags, backup::Backup};
use std::{fs, path::Path, time::Duration};

pub fn backup_database(source: &Path, target: &Path) -> Result<(), String> {
    let source_connection =
        Connection::open_with_flags(source, OpenFlags::SQLITE_OPEN_READ_ONLY)
            .map_err(|error| format!("Open backup source {}: {error}", source.display()))?;
    let mut target_connection = Connection::open(target)
        .map_err(|error| format!("Open backup target {}: {error}", target.display()))?;
    let backup = Backup::new(&source_connection, &mut target_connection)
        .map_err(|error| format!("Start backup {}: {error}", source.display()))?;
    backup
        .run_to_completion(100, Duration::from_millis(10), None)
        .map_err(|error| {
            format!(
                "Backup {} to {}: {error}",
                source.display(),
                target.display()
            )
        })
}

/// Stage every authored root CSV, preserving private fixture-generated catalogs.
pub fn stage_authored_csvs(source: &Path, target: &Path) -> Result<(), String> {
    let entries = fs::read_dir(source)
        .map_err(|error| format!("List authored CSVs {}: {error}", source.display()))?;
    for entry in entries {
        let entry = entry.map_err(|error| format!("Read authored CSV entry: {error}"))?;
        let path = entry.path();
        let destination = target.join(entry.file_name());
        if path.extension().is_some_and(|extension| extension == "csv") && !destination.exists() {
            std::os::unix::fs::symlink(&path, &destination)
                .map_err(|error| format!("Stage CSV {}: {error}", path.display()))?;
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn backup_includes_committed_wal_rows_and_isolates_writes() {
        let root = std::env::temp_dir().join(format!("fixture-backup-{}", std::process::id()));
        fs::create_dir_all(&root).unwrap();
        let source = root.join("source.sqlite");
        let target = root.join("target.sqlite");
        let writer = Connection::open(&source).unwrap();
        writer.execute_batch("PRAGMA journal_mode=WAL; PRAGMA wal_autocheckpoint=0; CREATE TABLE entries (id INTEGER); INSERT INTO entries VALUES (42);").unwrap();
        backup_database(&source, &target).unwrap();
        let copy = Connection::open(&target).unwrap();
        copy.execute_batch("INSERT INTO entries VALUES (99);")
            .unwrap();
        let rows: Vec<i32> = copy
            .prepare("SELECT id FROM entries ORDER BY id")
            .unwrap()
            .query_map([], |row| row.get(0))
            .unwrap()
            .map(Result::unwrap)
            .collect();
        assert_eq!(rows, vec![42, 99]);
        let count: i32 = writer
            .query_row("SELECT count(*) FROM entries", [], |row| row.get(0))
            .unwrap();
        assert_eq!(count, 1);
        drop(copy);
        drop(writer);
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn stages_equipment_csv_and_preserves_generated_catalog() {
        let root = std::env::temp_dir().join(format!("fixture-csv-{}", std::process::id()));
        let source = root.join("source");
        let target = root.join("target");
        fs::create_dir_all(&source).unwrap();
        fs::create_dir_all(&target).unwrap();
        fs::write(
            source.join("ItemDisplayInfoModelMatRes.csv"),
            "ID,MaterialResourcesID\n25,123\n",
        )
        .unwrap();
        fs::write(source.join("ChrRaces.csv"), "authored").unwrap();
        fs::write(target.join("ChrRaces.csv"), "generated").unwrap();
        stage_authored_csvs(&source, &target).unwrap();
        assert_eq!(
            fs::read_to_string(target.join("ItemDisplayInfoModelMatRes.csv")).unwrap(),
            "ID,MaterialResourcesID\n25,123\n"
        );
        assert_eq!(
            fs::read_to_string(target.join("ChrRaces.csv")).unwrap(),
            "generated"
        );
        fs::remove_dir_all(root).unwrap();
    }
}
