//! Retail70448 TalkingHeadUI.lua callbacks and XML alpha groups. Voice continues
//! to read the authoritative line; delayed name/text replacement is paint only.
use super::{CLOSE_DURATION_MS, TalkingHeadState, TalkingHeadView};
use shared::protocol::ActorObject;

#[derive(Clone, Debug, PartialEq)]
pub struct TalkingHeadPaint {
    pub view: TalkingHeadView,
    pub name_alpha: f32,
    pub text_alpha: f32,
    pub background_alpha: f32,
    pub portrait_alpha: f32,
    pub model_alpha: f32,
    pub close_alpha: f32,
    pub main_elapsed_ms: f64,
}

pub(super) fn present(state: &TalkingHeadState) -> Option<TalkingHeadPaint> {
    if let Some(closing) = &state.closing {
        return Some(close_paint(closing.view.clone(), closing.elapsed_ms));
    }
    let mut view = state.current()?;
    let active = state
        .active
        .iter()
        .find(|active| active.payload.id == view.conversation)?;
    let lines: Vec<_> = active
        .payload
        .lines
        .iter()
        .filter_map(|line| {
            let actor = active
                .payload
                .actors
                .iter()
                .find(|actor| actor.index == line.actor_index)?;
            if !matches!(actor.object, ActorObject::TalkingHead { .. }) {
                return None;
            }
            Some((
                line.start_ms as f64,
                actor.name.as_deref()?,
                line.text.as_ref()?.male_text.as_str(),
            ))
        })
        .collect();
    let elapsed = active.elapsed_ms;
    let (name, name_alpha, name_changed) = paint_field(
        lines.iter().map(|(start, name, _)| (*start, *name)),
        elapsed,
        500.0,
    )?;
    let (text, text_alpha, _) = paint_field(
        lines.iter().map(|(start, _, text)| (*start, *text)),
        elapsed,
        750.0,
    )?;
    view.name = name;
    view.text = text;
    let main_elapsed_ms = elapsed - name_changed;
    Some(TalkingHeadPaint {
        view,
        name_alpha,
        text_alpha,
        background_alpha: opening_alpha(elapsed, 400.0, 750.0),
        portrait_alpha: opening_alpha(elapsed, 0.0, 750.0),
        model_alpha: opening_alpha(main_elapsed_ms, 0.0, 750.0),
        close_alpha: opening_alpha(main_elapsed_ms, 750.0, 750.0),
        main_elapsed_ms,
    })
}

fn close_paint(view: TalkingHeadView, elapsed: f64) -> TalkingHeadPaint {
    // FadeoutFrames plays each source Close alpha group from1 to0 for1s.
    let alpha = (1.0 - elapsed / CLOSE_DURATION_MS).clamp(0.0, 1.0) as f32;
    let main_elapsed_ms = view.conversation_elapsed_ms as f64 + elapsed;
    TalkingHeadPaint {
        view,
        name_alpha: alpha,
        text_alpha: alpha,
        background_alpha: alpha,
        portrait_alpha: alpha,
        model_alpha: alpha,
        close_alpha: alpha,
        main_elapsed_ms,
    }
}

fn paint_field<'a>(
    values: impl IntoIterator<Item = (f64, &'a str)>,
    elapsed: f64,
    initial_fade: f64,
) -> Option<(String, f32, f64)> {
    let mut values = values.into_iter();
    let (_, mut requested) = values.next()?;
    let mut shown = requested;
    let mut changed = 0.0;
    let mut alpha = opening_alpha(elapsed, initial_fade, 250.0);
    let mut latest_fade = if elapsed >= initial_fade {
        initial_fade
    } else {
        f64::NEG_INFINITY
    };
    for (start, value) in values.filter(|(start, _)| *start <= elapsed) {
        if value == requested {
            continue;
        }
        requested = value;
        changed = start;
        if elapsed >= start + 250.0 {
            shown = value;
        }
        if start >= latest_fade {
            alpha = (1.0 - (elapsed - start) / 250.0).clamp(0.0, 1.0) as f32;
            latest_fade = start;
        }
        if elapsed >= start + 500.0 && start + 500.0 >= latest_fade {
            alpha = opening_alpha(elapsed, start + 500.0, 250.0);
            latest_fade = start + 500.0;
        }
    }
    Some((shown.to_owned(), alpha, changed))
}

fn opening_alpha(elapsed: f64, delay: f64, duration: f64) -> f32 {
    if elapsed < delay {
        // Reset leaves regions whose source callback has not begun at0.01.
        return 0.01;
    }
    ((elapsed - delay) / duration).min(1.0) as f32
}
