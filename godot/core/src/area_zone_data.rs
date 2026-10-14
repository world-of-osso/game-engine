use std::collections::HashMap;
use std::io::BufRead;
use std::path::Path;

use crate::csv_util::{header_index, parse_csv_line_trimmed};

/// AreaTable `ID` → `ParentAreaID` for every area with a nonzero parent.
pub fn load_area_parents<R: BufRead>(reader: R, path: &Path) -> Result<HashMap<u32, u32>, String> {
    let mut reader = reader;
    let mut header = String::new();
    reader
        .read_line(&mut header)
        .map_err(|err| format!("read {} header: {err}", path.display()))?;
    let headers = parse_csv_line_trimmed(header.trim_end_matches(['\r', '\n']));
    let id_col = header_index(&headers, "ID", path)?;
    let parent_col = header_index(&headers, "ParentAreaID", path)?;
    let mut parents = HashMap::new();
    for (line_number, line) in reader.lines().enumerate() {
        let line = line.map_err(|err| format!("read {} row: {err}", path.display()))?;
        let fields = parse_csv_line_trimmed(&line);
        let field = |column| {
            crate::csv_util::parse_field::<u32>(&fields, column)
                .map_err(|error| format!("{}:{}: {error}", path.display(), line_number + 2))
        };
        let id = field(id_col)?;
        let parent = field(parent_col)?;
        if parent != 0 {
            parents.insert(id, parent);
        }
    }
    Ok(parents)
}

/// Follows parents to the root; the bound stops traversal of bad data.
pub fn root_area(parents: &HashMap<u32, u32>, area_id: u32) -> u32 {
    let mut id = area_id;
    for _ in 0..16 {
        match parents.get(&id) {
            Some(&parent) => id = parent,
            None => break,
        }
    }
    id
}

#[cfg(test)]
mod db2_header_tests {
    use super::*;
    #[test]
    fn db2_header_area_parents_reject_invalid_canonical_row() {
        let fixture = crate::db2_test_support::ExportFixture::copy("AreaTable", Some("ID"));
        let error = load_area_parents(
            std::io::Cursor::new(std::fs::read(&fixture.0).unwrap()),
            &fixture.0,
        )
        .unwrap_err();
        assert!(error.contains("AreaTable.csv:2:"), "{error}");
    }
}
