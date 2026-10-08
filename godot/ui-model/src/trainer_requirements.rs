//! TrainerUI.lua:208-266 and GlobalStrings: only unmet numbers/abilities are red.
use super::{RED, TrainerView, WHITE};
use crate::bank_art::label;
use shared::protocol::{TrainerService, TrainerServiceState};
use ui_toolkit::{text_measure::measure_text, widget_def::Element, widgets::font_string::GameFont};

const WIDTH: f32 = 240.0;
const FONT_SIZE: f32 = 10.0;
#[derive(Clone)]
struct Run {
    text: String,
    color: &'static str,
}
fn run(text: impl Into<String>, met: bool) -> Run {
    Run {
        text: text.into(),
        color: if met { WHITE } else { RED },
    }
}

fn level_runs(service: &TrainerService, view: &TrainerView) -> Vec<Run> {
    if service.req_level <= 1 {
        return vec![];
    }
    vec![
        run("Level ", true),
        run(
            service.req_level.to_string(),
            view.player_level >= service.req_level,
        ),
    ]
}
fn skill_runs(service: &TrainerService, view: &TrainerView) -> Vec<Run> {
    if service.req_skill_line == 0 {
        return vec![];
    }
    let name = view
        .display
        .skills
        .get(&service.req_skill_line)
        .cloned()
        .unwrap_or_else(|| format!("Skill {}", service.req_skill_line));
    let met = view.ranks.iter().any(|line| {
        line.skill_line == service.req_skill_line && line.rank >= service.req_skill_rank
    });
    vec![
        run(format!("{name} ("), true),
        run(service.req_skill_rank.to_string(), met),
        run(")", true),
    ]
}
fn requirement_runs(service: &TrainerService, view: &TrainerView) -> Vec<Run> {
    let mut groups = vec![level_runs(service, view), skill_runs(service, view)];
    groups.extend(service.req_abilities.iter().map(|id| {
        vec![run(
            view.display.spell_name(*id),
            view.known_spells.contains(id),
        )]
    }));
    groups
        .into_iter()
        .filter(|group| !group.is_empty())
        .enumerate()
        .flat_map(|(index, group)| {
            std::iter::once(run(if index == 0 { "Requires: " } else { ", " }, true)).chain(group)
        })
        .collect()
}

struct PlacedRun {
    run: Run,
    x: f32,
    line: usize,
    width: f32,
}
fn wrap_runs(runs: Vec<Run>) -> Vec<PlacedRun> {
    let mut x = 0.0;
    let mut line = 0;
    let mut placed = vec![];
    for run in runs {
        for word in run.text.split_inclusive(' ') {
            let width = measure_text(word, GameFont::FrizQuadrata, FONT_SIZE)
                .expect("Trainer requirement font")
                .0;
            if x > 0.0 && x + width > WIDTH {
                x = 0.0;
                line += 1;
            }
            placed.push(PlacedRun {
                run: Run {
                    text: word.into(),
                    color: run.color,
                },
                x,
                line,
                width,
            });
            x += width;
        }
    }
    placed
}

pub(super) fn subtext(prefix: &str, service: &TrainerService, view: &TrainerView) -> Element {
    if service.state == TrainerServiceState::Known {
        return label(
            format!("{prefix}Requirements"),
            "Already known",
            (48.0, 26.5, WIDTH, FONT_SIZE),
            (FONT_SIZE, WHITE, "LEFT"),
        );
    }
    let runs = wrap_runs(requirement_runs(service, view));
    let lines = runs.last().map_or(0, |run| run.line + 1);
    let top = 16.5 + (30.0 - lines as f32 * FONT_SIZE) / 2.0;
    runs.into_iter()
        .enumerate()
        .flat_map(|(index, placed)| {
            label(
                format!("{prefix}Requirements{index}"),
                &placed.run.text,
                (
                    48.0 + placed.x,
                    top + placed.line as f32 * FONT_SIZE,
                    placed.width + 1.0,
                    FONT_SIZE,
                ),
                (FONT_SIZE, placed.run.color, "LEFT"),
            )
        })
        .collect()
}
