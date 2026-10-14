//! Canonical installed-export fixtures; never synthesize DB2 records.
use std::path::{Path, PathBuf};

pub(crate) struct ExportFixture(pub PathBuf);

impl ExportFixture {
    pub(crate) fn copy(table: &str, corrupt_column: Option<&str>) -> Self {
        let source = Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../../data")
            .join(format!("{table}.csv"));
        let text = std::fs::read_to_string(&source).unwrap();
        let name = Path::new(table).file_name().unwrap().to_str().unwrap();
        let dir = std::env::temp_dir().join(format!(
            "db2-{name}-{}-{:?}",
            std::process::id(),
            std::thread::current().id()
        ));
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join(format!("{name}.csv"));
        if let Some(column) = corrupt_column {
            let mut records = crate::csv_util::parse_csv_records(&text);
            let index = records[0].iter().position(|name| name == column).unwrap();
            records[1][index] = "invalid".into();
            let encode = |field: &String| format!("\"{}\"", field.replace('"', "\"\""));
            let csv = records
                .iter()
                .map(|row| row.iter().map(encode).collect::<Vec<_>>().join(","))
                .collect::<Vec<_>>()
                .join("\n");
            std::fs::write(&path, csv).unwrap();
        } else {
            std::fs::write(&path, text).unwrap();
        }
        Self(path)
    }
}

impl Drop for ExportFixture {
    fn drop(&mut self) {
        std::fs::remove_dir_all(self.0.parent().unwrap()).unwrap();
    }
}
