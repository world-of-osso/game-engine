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

pub fn parse_csv_line_trimmed(line: &str) -> Vec<String> {
    parse_csv_line(line)
        .into_iter()
        .map(|field| field.trim().to_string())
        .collect()
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
        let fields = parse_csv_line(r#""a """"quoted"""" value",x"#);
        assert_eq!(fields, vec![r#"a "quoted" value"#, "x"]);
    }

    #[test]
    fn parse_csv_line_trimmed_trims_unquoted_whitespace() {
        let fields = parse_csv_line_trimmed(" a , \" b \" ,c ");
        assert_eq!(fields, vec!["a", "b", "c"]);
    }
}
