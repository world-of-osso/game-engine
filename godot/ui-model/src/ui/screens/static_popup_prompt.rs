//! Word wrapping for GameDialog currency texture markup inside its290px text region.
use crate::{
    bank_art::{WHITE, label, texture},
    ui::widgets::font_string::GameFont,
};
use ui_toolkit::{text_measure::measure_text, widget_def::Element};

const FONT_SIZE: f32 = 12.;
const LINE_HEIGHT: f32 = 16.;
const TEXT_WIDTH: f32 = 290.;

#[derive(Clone, Copy)]
enum Part<'a> {
    Word(&'a str),
    Currency(u32),
}

pub(super) fn wrapped_currency_prompt(
    prefix: &str,
    (before, after): (&str, &str),
    icon: u32,
) -> (Element, f32) {
    let parts = before
        .split_whitespace()
        .map(Part::Word)
        .chain(std::iter::once(Part::Currency(icon)))
        .chain(after.split_whitespace().map(Part::Word));
    let space = measure_text(" ", GameFont::FrizQuadrata, FONT_SIZE)
        .expect("dialog font")
        .0;
    let lines = wrap_parts(parts, space);
    let height = lines.len() as f32 * LINE_HEIGHT;
    let contents = lines
        .into_iter()
        .enumerate()
        .flat_map(|(line, parts)| render_line(prefix, line, parts, space))
        .collect();
    (contents, height)
}

fn wrap_parts<'a>(parts: impl Iterator<Item = Part<'a>>, space: f32) -> Vec<Vec<(Part<'a>, f32)>> {
    let mut lines = vec![Vec::new()];
    let mut used = 0.;
    for part in parts {
        let width = match part {
            Part::Word(word) => {
                measure_text(word, GameFont::FrizQuadrata, FONT_SIZE)
                    .expect("dialog font")
                    .0
            }
            Part::Currency(_) => 16.,
        };
        if used > 0. && used + space + width > TEXT_WIDTH {
            lines.push(Vec::new());
            used = 0.;
        }
        if used > 0. {
            used += space;
        }
        lines.last_mut().expect("prompt line").push((part, width));
        used += width;
    }
    lines
}

fn render_line(prefix: &str, line: usize, parts: Vec<(Part<'_>, f32)>, space: f32) -> Element {
    let width = parts.iter().map(|(_, width)| width).sum::<f32>()
        + parts.len().saturating_sub(1) as f32 * space;
    let mut left = (320. - width) / 2.;
    let top = 16. + line as f32 * LINE_HEIGHT;
    parts
        .into_iter()
        .enumerate()
        .flat_map(|(index, (part, width))| {
            let rect = (left, top, width, LINE_HEIGHT);
            left += width + space;
            match part {
                Part::Word(word) => label(
                    format!("{prefix}TextLine{line}Word{index}"),
                    word,
                    rect,
                    (FONT_SIZE, WHITE, "LEFT"),
                ),
                Part::Currency(icon) => texture(format!("{prefix}CurrencyIcon"), icon, rect, WHITE),
            }
        })
        .collect()
}
