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
    for line in reader.lines() {
        let line = line.map_err(|err| format!("read {} row: {err}", path.display()))?;
        let fields = parse_csv_line_trimmed(&line);
        let field = |col: usize| fields.get(col).and_then(|value| value.parse::<u32>().ok());
        if let (Some(id), Some(parent)) = (field(id_col), field(parent_col))
            && parent != 0
        {
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
