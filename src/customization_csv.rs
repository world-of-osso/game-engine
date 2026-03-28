use std::path::Path;

pub(super) fn read_csv(path: &Path) -> Result<(Vec<String>, Vec<Vec<String>>), String> {
    let data =
        std::fs::read_to_string(path).map_err(|e| format!("read {}: {e}", path.display()))?;
    let mut lines = data.lines();
    let header_line = lines.next().ok_or("empty CSV")?;
    let headers = parse_csv_line(header_line);
    let rows: Vec<_> = lines.map(parse_csv_line).collect();
    Ok((headers, rows))
}

pub(super) fn col_idx(headers: &[String], name: &str) -> Result<usize, String> {
    headers
        .iter()
        .position(|h| h == name)
        .ok_or_else(|| format!("Column '{name}' not found"))
}

pub(super) fn field_u32(row: &[String], col: usize) -> u32 {
    row.get(col)
        .and_then(|s| {
            s.parse::<u32>()
                .ok()
                .or_else(|| s.parse::<i32>().ok().map(|v| v as u32))
        })
        .unwrap_or(0)
}

pub(super) fn field_str(row: &[String], col: usize) -> String {
    row.get(col).cloned().unwrap_or_default()
}

pub(super) fn field_f32(row: &[String], col: usize) -> f32 {
    row.get(col)
        .and_then(|s| s.parse::<f32>().ok())
        .unwrap_or(0.0)
}

fn parse_csv_line(line: &str) -> Vec<String> {
    let mut fields = Vec::new();
    let mut current = String::new();
    let mut in_quotes = false;
    let mut chars = line.chars().peekable();
    while let Some(ch) = chars.next() {
        if in_quotes {
            if ch == '"' {
                if chars.peek() == Some(&'"') {
                    chars.next();
                    current.push('"');
                } else {
                    in_quotes = false;
                }
            } else {
                current.push(ch);
            }
        } else if ch == '"' {
            in_quotes = true;
        } else if ch == ',' {
            fields.push(std::mem::take(&mut current));
        } else {
            current.push(ch);
        }
    }
    fields.push(current);
    fields
}
