//! Static `$token` substitution for spell descriptions.
//!
//! Supported: `$s`/`$w`/`$m`/`$t`/`$a`/`$o`/`$x` with a 1-9 effect index, `$d`,
//! `$u`, `$n`, `$h`, each optionally prefixed by a spell id (`$12345s1`).
//! Values come from the DB rows alone: no caster stats, so spell-power scaled
//! effects render their stored base points. Anything else (`${expr}`,
//! `$?cond`, `$<var>`, `$@name`, `$/n;s1`, `$l`/`$g` plurals) and tokens whose
//! data is absent stay verbatim.

use super::{CatalogSpell, SpellCatalogData};

pub(super) fn render_spell_text(
    text: &str,
    spell: &CatalogSpell,
    catalog: &SpellCatalogData,
) -> String {
    let mut out = String::with_capacity(text.len());
    let mut rest = text;
    while let Some(pos) = rest.find('$') {
        out.push_str(&rest[..pos]);
        rest = &rest[pos..];
        let consumed = render_token(rest, spell, catalog, &mut out);
        rest = &rest[consumed..];
    }
    out.push_str(rest);
    out
}

/// `text` starts with `$`. Appends the rendering and returns the bytes consumed.
fn render_token(
    text: &str,
    spell: &CatalogSpell,
    catalog: &SpellCatalogData,
    out: &mut String,
) -> usize {
    let body = &text[1..];
    if body.starts_with('{') {
        let len = 1 + braced_len(body);
        out.push_str(&text[..len]);
        return len;
    }
    let Some(token) = parse_token(body) else {
        out.push('$');
        return 1;
    };
    let len = 1 + token.len;
    let source = match token.spell_id {
        Some(id) => catalog.get(id),
        None => Some(spell),
    };
    match source.and_then(|source| token_value(source, token.letter, token.index)) {
        Some(value) => out.push_str(&value),
        None => out.push_str(&text[..len]),
    }
    len
}

/// Length of a `{...}` group including nested braces; unterminated groups run to the end.
fn braced_len(body: &str) -> usize {
    let mut depth = 0usize;
    for (pos, ch) in body.char_indices() {
        match ch {
            '{' => depth += 1,
            '}' => {
                depth -= 1;
                if depth == 0 {
                    return pos + 1;
                }
            }
            _ => {}
        }
    }
    body.len()
}

struct Token {
    spell_id: Option<u32>,
    letter: char,
    /// Zero-based effect index.
    index: Option<u8>,
    len: usize,
}

fn parse_token(body: &str) -> Option<Token> {
    let digits = body.bytes().take_while(u8::is_ascii_digit).count();
    let spell_id = match digits {
        0 => None,
        _ => Some(body[..digits].parse().ok()?),
    };
    let letter = body[digits..]
        .chars()
        .next()
        .filter(char::is_ascii_alphabetic)?
        .to_ascii_lowercase();
    let after_letter = digits + 1;
    let index = body[after_letter..]
        .bytes()
        .next()
        .filter(|byte| (b'1'..=b'9').contains(byte))
        .map(|byte| byte - b'1');
    Some(Token {
        spell_id,
        letter,
        index,
        len: after_letter + usize::from(index.is_some()),
    })
}

fn token_value(spell: &CatalogSpell, letter: char, index: Option<u8>) -> Option<String> {
    let effect = || index.and_then(|index| spell.effect(index));
    match letter {
        's' | 'w' => Some(format_number(effect()?.base_points.abs())),
        'm' => Some(format_number(effect()?.base_points)),
        't' => positive(effect()?.aura_period_ms).map(|ms| format_number(ms as f32 / 1000.0)),
        'a' => Some(effect()?.radius_yd)
            .filter(|radius| *radius > 0.0)
            .map(format_number),
        'o' => periodic_total(spell, effect()?.base_points, effect()?.aura_period_ms),
        'x' => positive(effect()?.chain_targets).map(|targets| targets.to_string()),
        'd' => positive(spell.duration_ms).map(format_duration),
        'u' => positive(spell.max_stacks).map(|stacks| stacks.to_string()),
        'n' => positive(spell.proc_charges).map(|charges| charges.to_string()),
        'h' => positive(spell.proc_chance).map(|chance| chance.to_string()),
        _ => None,
    }
}

fn positive<T: PartialOrd + Default>(value: T) -> Option<T> {
    (value > T::default()).then_some(value)
}

fn periodic_total(spell: &CatalogSpell, base_points: f32, period_ms: u32) -> Option<String> {
    let period_ms = positive(period_ms)?;
    let duration_ms = positive(spell.duration_ms)?;
    let ticks = duration_ms as u32 / period_ms;
    Some(format_number((base_points * ticks as f32).abs()))
}

/// Up to two decimals, trailing zeros trimmed.
fn format_number(value: f32) -> String {
    let text = format!("{value:.2}");
    let text = text.trim_end_matches('0').trim_end_matches('.');
    match text {
        "-0" => "0".to_string(),
        _ => text.to_string(),
    }
}

fn format_duration(ms: i32) -> String {
    let secs = ms as f32 / 1000.0;
    let (amount, singular, plural) = match secs {
        s if s < 60.0 => (s, "sec", "sec"),
        s if s < 3600.0 => (s / 60.0, "min", "min"),
        s if s < 86400.0 => (s / 3600.0, "hour", "hours"),
        s => (s / 86400.0, "day", "days"),
    };
    let unit = if amount == 1.0 { singular } else { plural };
    format!("{} {unit}", format_number(amount))
}
