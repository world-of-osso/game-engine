//! CSV parsing for outfit DB2 tables.

use std::collections::HashMap;
use std::path::Path;

pub fn parse_race_prefix(path: &Path) -> Result<HashMap<u8, String>, String> {
    let (headers, rows) = read_csv(path)?;
    let id_col = col(&headers, "ID")?;
    let prefix_col = col(&headers, "ClientPrefix")?;

    let mut map = HashMap::new();
    for row in &rows {
        let id = field_u32(row, id_col) as u8;
        let prefix = row
            .get(prefix_col)
            .map(|s| s.trim().to_ascii_lowercase())
            .unwrap_or_default();
        if id != 0 && !prefix.is_empty() {
            map.insert(id, prefix);
        }
    }
    Ok(map)
}

fn read_csv(path: &Path) -> Result<(Vec<String>, Vec<Vec<String>>), String> {
    let data =
        std::fs::read_to_string(path).map_err(|e| format!("read {}: {e}", path.display()))?;
    let mut lines = data.lines();
    let header_line = lines.next().ok_or("empty CSV")?;
    let headers = parse_csv_line(header_line);
    let rows: Vec<_> = lines.map(parse_csv_line).collect();
    Ok((headers, rows))
}

fn parse_csv_line(line: &str) -> Vec<String> {
    let mut out = Vec::new();
    let mut cur = String::new();
    let mut in_quotes = false;
    let mut chars = line.chars().peekable();
    while let Some(ch) = chars.next() {
        match ch {
            '"' => {
                if in_quotes && chars.peek() == Some(&'"') {
                    cur.push('"');
                    chars.next();
                } else {
                    in_quotes = !in_quotes;
                }
            }
            ',' if !in_quotes => {
                out.push(cur.trim().to_string());
                cur.clear();
            }
            _ => cur.push(ch),
        }
    }
    out.push(cur.trim().to_string());
    out
}

fn col(headers: &[String], name: &str) -> Result<usize, String> {
    headers
        .iter()
        .position(|h| h == name)
        .ok_or_else(|| format!("missing column {name}"))
}

fn field_u32(row: &[String], col: usize) -> u32 {
    row.get(col)
        .and_then(|s| s.trim().parse::<u32>().ok())
        .unwrap_or(0)
}
