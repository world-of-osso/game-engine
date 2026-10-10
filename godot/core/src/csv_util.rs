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
    #[test]
    fn numeric_rows_reads_local_pet_species_with_multiline_description_fields() {
        let path = Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../../data/db2/12.1.0.69933/BattlePetSpecies.csv");
        let mut icons = std::collections::BTreeMap::new();
        let result = read_numeric_rows(&path, ["ID", "IconFileDataID"], |[id, icon]| {
            icons.insert(id, icon);
        });
        assert!(result.is_ok(), "local pet CSV failed: {result:?}");
        assert_eq!(icons.len(), 2994);
        assert_eq!(icons[&39], 656559);
        assert!(icons.contains_key(&1530));
    }
}

/// Call `row` with the integer values of `columns` for each record of the CSV export at
/// `path`; other columns may hold quoted text.
pub fn read_numeric_rows<const N: usize>(
    path: &Path,
    columns: [&str; N],
    mut row: impl FnMut([i64; N]),
) -> Result<(), String> {
    let text = std::fs::read_to_string(path)
        .map_err(|error| format!("read {}: {error}", path.display()))?;
    let mut records = parse_csv_records(&text).into_iter();
    let header = records
        .next()
        .ok_or_else(|| format!("{} has no header", path.display()))?;
    let mut indexes = [0; N];
    for (index, column) in indexes.iter_mut().zip(columns) {
        *index = header
            .iter()
            .position(|name| *name == column)
            .ok_or_else(|| format!("{} has no column {column}", path.display()))?;
    }
    for fields in records.filter(|row| row != &[String::new()]) {
        let mut values = [0; N];
        for (value, index) in values.iter_mut().zip(indexes) {
            let field = fields
                .get(index)
                .ok_or_else(|| format!("{}: short row {fields:?}", path.display()))?;
            *value = field
                .parse()
                .map_err(|error| format!("{}: bad value {field:?}: {error}", path.display()))?;
        }
        row(values);
    }
    Ok(())
}
