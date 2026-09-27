use std::io::BufRead;
use std::path::Path;

pub fn parse_csv_line(line: &str) -> Vec<String> {
    let mut fields = Vec::new();
    let mut current = String::new();
    let mut in_quotes = false;
    let mut chars = line.chars().peekable();

    while let Some(ch) = chars.next() {
        match ch {
            '"' => {
                if in_quotes && chars.peek() == Some(&'"') {
                    current.push('"');
                    chars.next();
                } else {
                    in_quotes = !in_quotes;
                }
            }
            ',' if !in_quotes => {
                fields.push(current);
                current = String::new();
            }
            _ => current.push(ch),
        }
    }
    fields.push(current);
    fields
}

/// The records of CSV `text`, a quoted field may span lines (`Map.csv` descriptions).
pub fn parse_csv_records(text: &str) -> Vec<Vec<String>> {
    let mut records = Vec::new();
    let mut record = String::new();
    for line in text.lines() {
        if !record.is_empty() {
            record.push('\n');
        }
        record.push_str(line);
        if record.matches('"').count() % 2 == 0 {
            records.push(parse_csv_line(&record));
            record.clear();
        }
    }
    if !record.is_empty() {
        records.push(parse_csv_line(&record));
    }
    records
}

pub fn parse_csv_line_trimmed(line: &str) -> Vec<String> {
    parse_csv_line(line)
        .into_iter()
        .map(|field| field.trim().to_string())
        .collect()
}

pub fn header_index(headers: &[String], column: &str, path: &Path) -> Result<usize, String> {
    headers
        .iter()
        .position(|header| header == column)
        .ok_or_else(|| format!("{} missing {column} column", path.display()))
}

pub fn skip_csv_header<R: BufRead>(reader: &mut R, path: &Path) -> Result<(), String> {
    let mut header = String::new();
    reader
        .read_line(&mut header)
        .map_err(|err| format!("read {} header: {err}", path.display()))?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_csv_line_handles_quoted_commas() {
        let fields = parse_csv_line(r#""Hello, World",42,"test""#);
        assert_eq!(fields, vec!["Hello, World", "42", "test"]);
    }

    #[test]
    fn parse_csv_line_handles_escaped_quotes() {
        let fields = parse_csv_line(r#""a ""quoted"" value",x"#);
        assert_eq!(fields, vec![r#"a "quoted" value"#, "x"]);
    }

    #[test]
    fn parse_csv_records_keeps_quoted_newlines_in_one_record() {
        let records = parse_csv_records("ID,Name\n1,\"two\nlines\"\n2,x\n");
        assert_eq!(
            records,
            vec![vec!["ID", "Name"], vec!["1", "two\nlines"], vec!["2", "x"]]
        );
    }

    #[test]
    fn parse_csv_line_trimmed_trims_unquoted_whitespace() {
        let fields = parse_csv_line_trimmed(" a , \" b \" ,c ");
        assert_eq!(fields, vec!["a", "b", "c"]);
    }
}
