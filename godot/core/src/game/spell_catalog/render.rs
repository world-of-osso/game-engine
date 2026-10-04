//! Retail `$token` substitution for spell descriptions; grammar notes and
//! sources in `docs/reference/spell-description-tokens.md`.
//!
//! Resolved from the local DB2 rows and the viewing player's [`SpellTextContext`]:
//! effect tokens (`$s1`, `$m1`, `$w1`, `$t1`, `$a1`, `$o1`, `$x1`, upper-case
//! variants; spell and attack power scaled points with the player's replicated
//! powers), `$d`, `$u`, `$n`, `$h`, `$r`, `$SP`, `$AP`, each optionally prefixed by a
//! spell id;
//! `$?cond[..][..]` chains over known spells (`s`), player auras (`a`), the spec
//! index (`c`) and numeric comparisons; `${expr}.N`; `$/N;tok` and `$*N;tok`;
//! `$lsingular:plural;`; `$@spelldesc`/`$@spelltooltip`/`$@spellaura`/
//! `$@auradesc`/`$@spellname` references.
//!
//! `$<name>` reads the spell's `SpellDescriptionVariables` definition `$name=...`:
//! rendered as text in the description, evaluated as a number inside `${...}`.
//!
//! Anything else (other caster stats such as `$pri`, level scaled effect points,
//! power scaled points before the powers arrive, `$g` gender forms, inline icons) renders as the visible marker `{?<token>}` and logs one
//! warning per spell and token.

use std::collections::HashSet;
use std::sync::{Mutex, OnceLock};

use log::warn;

use super::render_eval::{Expr, Operand, eval_condition, parse_expr, variable_name};
use super::{CatalogSpell, SpellCatalogData, SpellTextContext};

/// `$@spelldesc` and `$<name>` nesting beyond this renders a marker (reference cycles
/// exist). Crusader Strike's `$<damage>` alone nests three variables deep.
const MAX_REFERENCE_DEPTH: u8 = 8;

pub(super) fn render_spell_text(
    text: &str,
    spell: &CatalogSpell,
    catalog: &SpellCatalogData,
    ctx: &SpellTextContext,
) -> String {
    let renderer = Renderer {
        catalog,
        ctx,
        depth: 0,
    };
    renderer.render(text, spell)
}

pub(super) struct Renderer<'a> {
    pub catalog: &'a SpellCatalogData,
    pub ctx: &'a SpellTextContext,
    depth: u8,
}

/// A token whose value the local data cannot supply.
#[derive(Debug, PartialEq)]
pub(super) struct Unresolved;

/// Output under construction; `last_number` feeds `$l` plural forms.
struct Out {
    text: String,
    last_number: Option<f64>,
}

impl Renderer<'_> {
    fn render(&self, text: &str, spell: &CatalogSpell) -> String {
        let mut out = Out {
            text: String::with_capacity(text.len()),
            last_number: None,
        };
        self.render_into(text, spell, &mut out);
        out.text
    }

    fn render_into(&self, text: &str, spell: &CatalogSpell, out: &mut Out) {
        let mut rest = text;
        while let Some(pos) = rest.find('$') {
            out.text.push_str(&rest[..pos]);
            rest = &rest[pos..];
            let consumed = self.render_token(rest, spell, out);
            rest = &rest[consumed..];
        }
        out.text.push_str(rest);
    }

    /// `text` starts with `$`. Appends the rendering and returns the bytes consumed.
    fn render_token(&self, text: &str, spell: &CatalogSpell, out: &mut Out) -> usize {
        let body = &text[1..];
        match body.chars().next() {
            Some('?') => self.render_conditional(text, spell, out),
            Some('{') => self.render_expression(text, spell, out),
            Some('@') => self.render_reference(text, spell, out),
            Some('<') => self.render_variable(text, spell, out),
            Some(op @ ('/' | '*')) => self.render_scaled(text, op, spell, out),
            _ => self.render_simple(text, spell, out),
        }
    }

    fn render_simple(&self, text: &str, spell: &CatalogSpell, out: &mut Out) -> usize {
        let Some(token) = parse_value_token(&text[1..]) else {
            out.text.push('$');
            return 1;
        };
        let len = 1 + token.len;
        let raw = &text[..len];
        if let Some(form_len) = word_form_len(&text[1..], &token) {
            let len = 1 + form_len;
            self.render_word_form(&text[..len], spell, out);
            return len;
        }
        if let Some(target) = token.reference_target() {
            self.render_description_of(target, raw, false, spell, out);
            return len;
        }
        if token.name.eq_ignore_ascii_case("d") {
            match self.source(&token, spell).and_then(duration_text) {
                Some(value) => out.text.push_str(&value),
                None => self.unresolved(spell, raw, out),
            }
            return len;
        }
        match self.value(&token, spell) {
            Ok(value) => push_number(out, value, None),
            Err(Unresolved) => self.unresolved(spell, raw, out),
        }
        len
    }

    /// `$/1000;s1` and `$*2;s1`: the token scaled by a constant.
    fn render_scaled(&self, text: &str, op: char, spell: &CatalogSpell, out: &mut Out) -> usize {
        let after_op = &text[2..];
        let parsed = after_op.split_once(';').and_then(|(factor, token_text)| {
            let factor: f64 = factor.parse().ok()?;
            let token = parse_value_token(token_text)?;
            let len = factor_len(after_op) + token.len;
            Some((factor, token, len))
        });
        let Some((factor, token, len)) = parsed else {
            out.text.push('$');
            return 1;
        };
        let len = 2 + len;
        match self.value(&token, spell) {
            Ok(value) if op == '/' && factor != 0.0 => push_number(out, value / factor, None),
            Ok(value) if op == '*' => push_number(out, value * factor, None),
            _ => self.unresolved(spell, &text[..len], out),
        }
        len
    }

    fn render_expression(&self, text: &str, spell: &CatalogSpell, out: &mut Out) -> usize {
        let braced = braced_len(&text[1..], '{', '}');
        let mut len = 1 + braced;
        let precision = decimal_suffix(&text[len..]);
        if precision.is_some() {
            len += 2;
        }
        let inner = text.get(2..braced).unwrap_or_default();
        let value = parse_expr(inner).and_then(|expr| self.evaluate(&expr, spell));
        match value {
            Ok(value) => push_number(out, value, Some(precision.unwrap_or(0))),
            Err(Unresolved) => self.unresolved(spell, &text[..len], out),
        }
        len
    }

    /// `$?c1[a]?c2[b][else]`, whitespace allowed between a condition and its branch.
    fn render_conditional(&self, text: &str, spell: &CatalogSpell, out: &mut Out) -> usize {
        let chain = parse_conditional_chain(text);
        if chain.arms.is_empty() {
            self.unresolved(spell, "$?", out);
            return 2;
        }
        match self.choose_branch(&chain, spell) {
            Ok(Some(branch)) => self.render_into(branch, spell, out),
            Ok(None) => {}
            Err(condition) => {
                let raw = format!("$?{}", condition.trim());
                self.unresolved(spell, &raw, out);
            }
        }
        chain.len
    }

    /// The branch the first true condition picks, else the `[else]` branch; `Err` holds
    /// the first condition that cannot be evaluated.
    fn choose_branch<'t>(
        &self,
        chain: &ConditionalChain<'t>,
        spell: &CatalogSpell,
    ) -> Result<Option<&'t str>, &'t str> {
        for arm in &chain.arms {
            match eval_condition(arm.condition, self, spell) {
                Ok(true) => return Ok(Some(arm.branch)),
                Ok(false) => {}
                Err(Unresolved) => return Err(arm.condition),
            }
        }
        Ok(chain.otherwise)
    }

    /// `$<name>` in description text: the definition rendered in place.
    fn render_variable(&self, text: &str, spell: &CatalogSpell, out: &mut Out) -> usize {
        let Some(name) = variable_name(&text[2..]) else {
            out.text.push('$');
            return 1;
        };
        let len = name.len() + 3;
        match (
            self.nested(),
            variable_definition(&spell.description_variables, name),
        ) {
            (Some(nested), Some(definition)) => nested.render_into(definition, spell, out),
            _ => self.unresolved(spell, &text[..len], out),
        }
        len
    }

    /// `$<name>` inside `${...}` or a condition: the definition's number.
    fn variable_value(&self, name: &str, spell: &CatalogSpell) -> Result<f64, Unresolved> {
        let definition =
            variable_definition(&spell.description_variables, name).ok_or(Unresolved)?;
        self.nested()
            .ok_or(Unresolved)?
            .definition_value(definition, spell)
    }

    /// A definition is one number form: `${expr}`, a `$?` chain whose branches are
    /// definitions, or a bare expression (`$s1`). A trailing unmatched `}` is ignored:
    /// Retail data has them (Crusader Strike's `$pvp=$?a134735[${1.3}][${1}]}`).
    fn definition_value(&self, text: &str, spell: &CatalogSpell) -> Result<f64, Unresolved> {
        let text = text.trim();
        let (value, rest) = if text.starts_with("$?") {
            let chain = parse_conditional_chain(text);
            let branch = self.choose_branch(&chain, spell).map_err(|_| Unresolved)?;
            let value = self.definition_value(branch.ok_or(Unresolved)?, spell)?;
            (value, &text[chain.len..])
        } else if text.starts_with("${") {
            let len = 1 + braced_len(&text[1..], '{', '}');
            let inner = text.get(2..len - 1).ok_or(Unresolved)?;
            let value = self.evaluate(&parse_expr(inner)?, spell)?;
            let precision = usize::from(decimal_suffix(&text[len..]).is_some()) * 2;
            (value, &text[len + precision..])
        } else {
            (self.evaluate(&parse_expr(text)?, spell)?, "")
        };
        match rest.trim_end_matches('}').trim() {
            "" => Ok(value),
            _ => Err(Unresolved),
        }
    }

    fn nested(&self) -> Option<Renderer<'_>> {
        (self.depth < MAX_REFERENCE_DEPTH).then(|| Renderer {
            catalog: self.catalog,
            ctx: self.ctx,
            depth: self.depth + 1,
        })
    }

    fn render_reference(&self, text: &str, spell: &CatalogSpell, out: &mut Out) -> usize {
        let body = &text[2..];
        let name_len = body.bytes().take_while(u8::is_ascii_alphabetic).count();
        let digits = body[name_len..]
            .bytes()
            .take_while(u8::is_ascii_digit)
            .count();
        let len = 2 + name_len + digits;
        let raw = &text[..len];
        let target = body[name_len..name_len + digits].parse::<u32>().ok();
        match (&body[..name_len], target) {
            ("spelldesc" | "spelltooltip", Some(id)) => {
                self.render_description_of(id, raw, false, spell, out)
            }
            ("spellaura" | "auradesc", Some(id)) => {
                self.render_description_of(id, raw, true, spell, out)
            }
            ("spellname", Some(id)) => match self.catalog.get(id) {
                Some(target) => out.text.push_str(&target.name),
                None => self.unresolved(spell, raw, out),
            },
            _ => self.unresolved(spell, raw, out),
        }
        len
    }

    fn render_description_of(
        &self,
        id: u32,
        raw: &str,
        aura: bool,
        spell: &CatalogSpell,
        out: &mut Out,
    ) {
        let (Some(target), Some(nested)) = (self.catalog.get(id), self.nested()) else {
            self.unresolved(spell, raw, out);
            return;
        };
        let text = if aura {
            &target.aura_description
        } else {
            &target.description
        };
        out.text.push_str(&nested.render(text, target));
    }

    /// `$lpoint:points;` picks by the last number shown.
    fn render_word_form(&self, raw: &str, spell: &CatalogSpell, out: &mut Out) {
        let is_plural_form = raw[1..].starts_with(['l', 'L']);
        let forms = &raw[2..raw.len() - 1];
        match (is_plural_form, out.last_number, forms.split_once(':')) {
            (true, Some(number), Some((singular, plural))) => {
                let word = if number == 1.0 { singular } else { plural };
                out.text.push_str(word);
            }
            _ => self.unresolved(spell, raw, out),
        }
    }

    fn source<'s>(
        &'s self,
        token: &ValueToken,
        spell: &'s CatalogSpell,
    ) -> Option<&'s CatalogSpell> {
        match token.spell_id {
            Some(id) => self.catalog.get(id),
            None => Some(spell),
        }
    }

    /// Numeric value of a letter token (`$d` in seconds).
    pub(super) fn value(
        &self,
        token: &ValueToken,
        spell: &CatalogSpell,
    ) -> Result<f64, Unresolved> {
        let source = self.source(token, spell).ok_or(Unresolved)?;
        letter_value(source, token, self.ctx.caster_power).ok_or(Unresolved)
    }

    pub(super) fn evaluate(&self, expr: &Expr, spell: &CatalogSpell) -> Result<f64, Unresolved> {
        expr.eval(&|operand| match operand {
            Operand::Token(token) => self.value(token, spell),
            Operand::Variable(name) => self.variable_value(name, spell),
        })
    }

    fn unresolved(&self, spell: &CatalogSpell, raw: &str, out: &mut Out) {
        warn_unresolved(spell.id, raw);
        out.text.push_str("{?");
        out.text.push_str(raw.trim());
        out.text.push('}');
        out.last_number = None;
    }
}

fn warn_unresolved(spell_id: u32, raw: &str) {
    static WARNED: OnceLock<Mutex<HashSet<(u32, String)>>> = OnceLock::new();
    let warned = WARNED.get_or_init(Default::default);
    let first = warned
        .lock()
        .map(|mut set| set.insert((spell_id, raw.to_string())))
        .unwrap_or(true);
    if first {
        warn!("spell {spell_id} description: unresolved token {raw:?}");
    }
}

fn push_number(out: &mut Out, value: f64, decimals: Option<usize>) {
    out.text.push_str(&match decimals {
        Some(decimals) => format_fixed(value, decimals),
        None => format_number(value),
    });
    out.last_number = Some(value);
}

/// `[digits]name[index]` after a `$`.
#[derive(Clone, Debug, PartialEq)]
pub(super) struct ValueToken {
    pub spell_id: Option<u32>,
    pub name: String,
    /// Zero-based effect index.
    pub index: Option<u8>,
    pub len: usize,
}

impl ValueToken {
    /// `$spelldesc123` without the `@`.
    fn reference_target(&self) -> Option<u32> {
        self.name.strip_prefix("spelldesc")?.parse().ok()
    }
}

pub(super) fn parse_value_token(body: &str) -> Option<ValueToken> {
    let digits = body.bytes().take_while(u8::is_ascii_digit).count();
    let spell_id = match digits {
        0 => None,
        _ => Some(body[..digits].parse().ok()?),
    };
    let letters = body[digits..]
        .bytes()
        .take_while(u8::is_ascii_alphabetic)
        .count();
    if letters == 0 {
        return None;
    }
    let mut name = body[digits..digits + letters].to_string();
    let mut len = digits + letters;
    if name == "spelldesc" {
        let id_len = body[len..].bytes().take_while(u8::is_ascii_digit).count();
        name.push_str(&body[len..len + id_len]);
        len += id_len;
        return Some(ValueToken {
            spell_id,
            name,
            index: None,
            len,
        });
    }
    let index = body[len..]
        .bytes()
        .next()
        .filter(|byte| (b'1'..=b'9').contains(byte))
        .map(|byte| byte - b'1');
    len += usize::from(index.is_some());
    Some(ValueToken {
        spell_id,
        name,
        index,
        len,
    })
}

/// Length after `$` of `$lsingular:plural;` / `$gmale:female;`, if `token` starts one.
fn word_form_len(body: &str, token: &ValueToken) -> Option<usize> {
    if token.spell_id.is_some() || !token.name.starts_with(['l', 'L', 'g', 'G']) {
        return None;
    }
    let end = body.find(';')?;
    let form = &body[1..end];
    let plain = !form.contains(['$', '\n', '[', ']']);
    (plain && form.contains(':')).then_some(end + 1)
}

fn factor_len(after_op: &str) -> usize {
    after_op.find(';').map_or(0, |pos| pos + 1)
}

fn letter_value(
    spell: &CatalogSpell,
    token: &ValueToken,
    power: Option<super::CasterPower>,
) -> Option<f64> {
    let effect = || spell.effect(token.index.unwrap_or(0));
    let points = || effect_points(effect()?, power);
    let value = match token.name.as_str() {
        "s" | "S" | "w" | "W" => points()?.abs(),
        "m" | "M" => points()?,
        "t" | "T" => positive(effect()?.aura_period_ms)? as f64 / 1000.0,
        "a" | "A" => Some(f64::from(effect()?.radius_yd)).filter(|radius| *radius > 0.0)?,
        "o" | "O" => periodic_total(spell, effect()?, points()?)?,
        "x" | "X" => positive(effect()?.chain_targets)? as f64,
        "d" | "D" => positive(spell.duration_ms)? as f64 / 1000.0,
        "u" | "U" => positive(spell.max_stacks)? as f64,
        "n" | "N" => positive(spell.proc_charges)? as f64,
        "h" | "H" => positive(spell.proc_chance)? as f64,
        "r" | "R" => max_range(spell)?,
        "SP" | "sp" => f64::from(power?.spell_power),
        "AP" | "ap" => f64::from(power?.attack_power),
        _ => return None,
    };
    Some(value)
}

/// The effect's points as the caster deals them before crit and percentage modifiers:
/// base points plus attack power and spell power times their coefficients, each bonus
/// truncated to an integer (`DoneTotal += int32(...)` in `Unit::SpellDamageBonusDone`,
/// TrinityCore a352b1fa Unit.cpp:6868-6905; the game server's `scaled_amount`). `None`
/// for level scaled points, and for power scaled points without the player's powers.
fn effect_points(effect: &super::CatalogEffect, power: Option<super::CasterPower>) -> Option<f64> {
    if effect.level_scaled {
        return None;
    }
    let base = f64::from(effect.base_points);
    if effect.spell_power_coefficient == 0.0 && effect.attack_power_coefficient == 0.0 {
        return Some(base);
    }
    let power = power?;
    // f32 like the server and TrinityCore: 1.4f × 100 is 140, in f64 139.99999.
    let bonus = |coefficient: f32, power: f32| f64::from((coefficient * power).trunc());
    Some(
        base + bonus(effect.attack_power_coefficient, power.attack_power)
            + bonus(effect.spell_power_coefficient, power.spell_power),
    )
}

fn positive<T: PartialOrd + Default>(value: T) -> Option<T> {
    (value > T::default()).then_some(value)
}

fn max_range(spell: &CatalogSpell) -> Option<f64> {
    let [hostile, friendly] = spell.range.max_yd;
    let range = if hostile > 0.0 { hostile } else { friendly };
    (range > 0.0).then_some(f64::from(range))
}

/// `$o`: the points of every tick over the duration.
fn periodic_total(spell: &CatalogSpell, effect: &super::CatalogEffect, points: f64) -> Option<f64> {
    let period_ms = positive(effect.aura_period_ms)?;
    let duration_ms = positive(spell.duration_ms)?;
    let ticks = duration_ms as u32 / period_ms;
    Some(points.abs() * f64::from(ticks))
}

fn duration_text(spell: &CatalogSpell) -> Option<String> {
    positive(spell.duration_ms).map(format_duration)
}

/// The text of definition `$name=...` in `SpellDescriptionVariables.Variables`. A
/// definition runs until the next line that starts another one.
fn variable_definition<'v>(variables: &'v str, name: &str) -> Option<&'v str> {
    let start = definition_starts(variables)
        .find(|(_, defined)| *defined == name)?
        .0;
    let body_start = start + name.len() + 2;
    let end = definition_starts(variables)
        .map(|(offset, _)| offset)
        .find(|offset| *offset > start)
        .unwrap_or(variables.len());
    Some(variables[body_start..end].trim())
}

/// `(offset of the `$`, name)` of each line that starts a `$name=` definition.
fn definition_starts(variables: &str) -> impl Iterator<Item = (usize, &str)> {
    let mut offset = 0;
    variables.split_inclusive('\n').filter_map(move |line| {
        let line_start = offset;
        offset += line.len();
        let trimmed = line.trim_start();
        let body = trimmed.strip_prefix('$')?;
        let name_len = body
            .bytes()
            .take_while(|byte| byte.is_ascii_alphanumeric() || *byte == b'_')
            .count();
        body[name_len..]
            .starts_with('=')
            .then(|| {
                let dollar = line_start + line.len() - trimmed.len();
                (dollar, &body[..name_len])
            })
            .filter(|(_, name)| !name.is_empty())
    })
}

/// `{`...`}` length including nested groups; unterminated groups run to the end.
pub(super) fn braced_len(body: &str, open: char, close: char) -> usize {
    let mut depth = 0usize;
    for (pos, ch) in body.char_indices() {
        if ch == open {
            depth += 1;
        } else if ch == close {
            depth = depth.saturating_sub(1);
            if depth == 0 {
                return pos + ch.len_utf8();
            }
        }
    }
    body.len()
}

/// `.N` precision right after `${...}`.
fn decimal_suffix(after: &str) -> Option<usize> {
    let mut chars = after.chars();
    (chars.next() == Some('.'))
        .then(|| chars.next()?.to_digit(10))
        .flatten()
        .map(|digits| digits as usize)
}

struct ConditionalArm<'t> {
    condition: &'t str,
    branch: &'t str,
}

struct ConditionalChain<'t> {
    arms: Vec<ConditionalArm<'t>>,
    otherwise: Option<&'t str>,
    len: usize,
}

/// `text` starts with `$?`.
fn parse_conditional_chain(text: &str) -> ConditionalChain<'_> {
    let mut arms = Vec::new();
    let mut pos = 1;
    while text[pos..].starts_with('?') {
        let condition_start = pos + 1;
        let Some(open) = condition_end(&text[condition_start..]) else {
            break;
        };
        let condition = &text[condition_start..condition_start + open];
        let branch_start = condition_start + open;
        let (branch, len) = bracket_group(&text[branch_start..]);
        arms.push(ConditionalArm { condition, branch });
        pos = branch_start + len;
    }
    let otherwise = text[pos..].starts_with('[').then(|| {
        let (branch, len) = bracket_group(&text[pos..]);
        pos += len;
        branch
    });
    ConditionalChain {
        arms,
        otherwise,
        len: pos,
    }
}

/// Offset of the `[` that opens the branch; parentheses group sub-conditions.
/// `None` when a character outside the condition grammar comes first.
fn condition_end(text: &str) -> Option<usize> {
    let mut depth = 0usize;
    for (pos, ch) in text.char_indices() {
        match ch {
            '(' => depth += 1,
            ')' => depth = depth.checked_sub(1)?,
            '[' if depth == 0 => return Some(pos),
            ch if ch.is_ascii_alphanumeric() || ch.is_ascii_whitespace() => {}
            '$' | '!' | '&' | '|' | '<' | '>' | '=' | '.' | '-' => {}
            _ => return None,
        }
    }
    None
}

/// `text` starts with `[`: the inner text and the group length.
fn bracket_group(text: &str) -> (&str, usize) {
    let len = braced_len(text, '[', ']');
    let inner_end = if text[..len].ends_with(']') {
        len - 1
    } else {
        len
    };
    (&text[1..inner_end], len)
}

/// Up to two decimals, trailing zeros trimmed.
pub(super) fn format_number(value: f64) -> String {
    let text = format!("{value:.2}");
    let text = text.trim_end_matches('0').trim_end_matches('.');
    match text {
        "-0" => "0".to_string(),
        _ => text.to_string(),
    }
}

fn format_fixed(value: f64, decimals: usize) -> String {
    let text = format!("{value:.decimals$}");
    match text.trim_start_matches('-').trim_matches(['0', '.']) {
        "" => format!("{:.decimals$}", 0.0),
        _ => text,
    }
}

fn format_duration(ms: i32) -> String {
    let secs = f64::from(ms) / 1000.0;
    let (amount, singular, plural) = match secs {
        s if s < 60.0 => (s, "sec", "sec"),
        s if s < 3600.0 => (s / 60.0, "min", "min"),
        s if s < 86400.0 => (s / 3600.0, "hour", "hours"),
        s => (s / 86400.0, "day", "days"),
    };
    let unit = if amount == 1.0 { singular } else { plural };
    format!("{} {unit}", format_number(amount))
}
