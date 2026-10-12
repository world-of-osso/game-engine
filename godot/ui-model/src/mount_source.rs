//! Mount DB2 text escapes for the native plain-label projection.
//! Inline money art is represented by its gold/silver/copper unit, never raw texture tags.
pub(crate) fn readable_source(source: &str) -> String {
    let mut out = String::with_capacity(source.len());
    let mut rest = source;
    while !rest.is_empty() {
        if let Some((bytes, text)) = source_escape(rest) {
            out.push_str(text);
            rest = &rest[bytes..];
        } else {
            let character = rest.chars().next().expect("nonempty source");
            out.push(character);
            rest = &rest[character.len_utf8()..];
        }
    }
    out
}

fn source_escape(source: &str) -> Option<(usize, &'static str)> {
    if source.starts_with("|n") {
        return Some((2, "\n"));
    }
    if source.starts_with("|r") {
        return Some((2, ""));
    }
    if source.starts_with("||") {
        return Some((2, "|"));
    }
    let color = source.get(2..10);
    if source.starts_with("|c")
        && color.is_some_and(|color| color.bytes().all(|byte| byte.is_ascii_hexdigit()))
    {
        return Some((10, ""));
    }
    let texture = source.strip_prefix("|T")?;
    let end = texture.find("|t")?;
    let name = texture[..end].to_ascii_uppercase();
    let unit = if name.contains("UI-GOLDICON") {
        "g"
    } else if name.contains("UI-SILVERICON") {
        "s"
    } else if name.contains("UI-COPPERICON") {
        "c"
    } else {
        ""
    };
    Some((end + 4, unit))
}
