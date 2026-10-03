//! RFC 4180 record reader. Spell descriptions contain quoted `\r\n`, so the
//! line-based `csv_util` parser cannot be used here.

use std::borrow::Cow;
use std::path::{Path, PathBuf};

pub(crate) struct CsvTable {
    path: PathBuf,
    headers: Vec<String>,
    body: String,
}

impl CsvTable {
    pub(crate) fn read(path: &Path) -> Result<Self, String> {
        let text = std::fs::read_to_string(path)
            .map_err(|err| format!("read {}: {err}", path.display()))?;
        Self::parse(path, &text)
    }

    /// `text` as the contents of the table at `path`.
    pub(crate) fn parse(path: &Path, text: &str) -> Result<Self, String> {
        let mut records = Records { rest: text };
        let headers = records
            .next()
            .ok_or_else(|| format!("{} has no header", path.display()))?
            .into_iter()
            .map(Cow::into_owned)
            .collect::<Vec<_>>();
        let body = records.rest.to_string();
        Ok(Self {
            path: path.to_path_buf(),
            headers,
            body,
        })
    }

    pub(crate) fn column(&self, name: &str) -> Result<usize, String> {
        crate::csv_util::header_index(&self.headers, name, &self.path)
    }

    pub(crate) fn records(&self) -> Records<'_> {
        Records { rest: &self.body }
    }

    pub(crate) fn path(&self) -> &Path {
        &self.path
    }
}

pub(crate) struct Records<'a> {
    rest: &'a str,
}

impl<'a> Iterator for Records<'a> {
    type Item = Vec<Cow<'a, str>>;

    fn next(&mut self) -> Option<Self::Item> {
        if self.rest.is_empty() {
            return None;
        }
        let mut fields = Vec::new();
        loop {
            let (field, terminator) = self.take_field();
            fields.push(field);
            if terminator != Some(',') {
                return Some(fields);
            }
        }
    }
}

impl<'a> Records<'a> {
    /// Returns the field and the byte that ended it (`,`, `\n`, or `None` at EOF).
    fn take_field(&mut self) -> (Cow<'a, str>, Option<char>) {
        if let Some(quoted) = self.rest.strip_prefix('"') {
            self.rest = quoted;
            let field = self.take_quoted();
            return (field, self.take_terminator());
        }
        let end = self.rest.find([',', '\n']).unwrap_or(self.rest.len());
        let field = self.rest[..end].trim_end_matches('\r');
        self.rest = &self.rest[end..];
        (Cow::Borrowed(field), self.take_terminator())
    }

    fn take_quoted(&mut self) -> Cow<'a, str> {
        let mut owned: Option<String> = None;
        let mut start = 0;
        let bytes = self.rest.as_bytes();
        let mut i = 0;
        while i < bytes.len() {
            if bytes[i] != b'"' {
                i += 1;
                continue;
            }
            if bytes.get(i + 1) == Some(&b'"') {
                owned
                    .get_or_insert_with(String::new)
                    .push_str(&self.rest[start..=i]);
                i += 2;
                start = i;
                continue;
            }
            break;
        }
        let tail = &self.rest[start..i];
        let field = match owned {
            Some(mut text) => {
                text.push_str(tail);
                Cow::Owned(text)
            }
            None => Cow::Borrowed(tail),
        };
        self.rest = self.rest.get(i + 1..).unwrap_or("");
        let skip = self.rest.find([',', '\n']).unwrap_or(self.rest.len());
        self.rest = &self.rest[skip..];
        field
    }

    fn take_terminator(&mut self) -> Option<char> {
        let terminator = self.rest.chars().next()?;
        self.rest = &self.rest[1..];
        Some(terminator)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn parse(text: &str) -> Vec<Vec<String>> {
        Records { rest: text }
            .map(|record| record.into_iter().map(Cow::into_owned).collect())
            .collect()
    }

    #[test]
    fn quoted_fields_keep_embedded_newlines_and_escaped_quotes() {
        let records = parse("1,\"a,\r\n\"\"b\"\"\",x\r\n2,,\"\"\n");
        assert_eq!(
            records,
            vec![vec!["1", "a,\r\n\"b\"", "x"], vec!["2", "", ""]]
        );
    }

    #[test]
    fn last_record_without_newline_is_returned() {
        assert_eq!(parse("7,Fire"), vec![vec!["7", "Fire"]]);
    }
}
