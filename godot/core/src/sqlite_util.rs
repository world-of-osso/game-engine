pub fn is_missing_table_error(err: &rusqlite::Error) -> bool {
    matches!(
        err,
        rusqlite::Error::SqliteFailure(_, Some(message)) if message.contains("no such table")
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn detects_missing_table_error() {
        let err = rusqlite::Error::SqliteFailure(
            rusqlite::ffi::Error::new(1),
            Some("no such table: demo".to_string()),
        );
        assert!(is_missing_table_error(&err));
    }

    #[test]
    fn ignores_other_sqlite_errors() {
        let err = rusqlite::Error::SqliteFailure(
            rusqlite::ffi::Error::new(1),
            Some("syntax error".to_string()),
        );
        assert!(!is_missing_table_error(&err));
    }
}
