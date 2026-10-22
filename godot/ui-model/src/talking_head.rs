//! Private source conversation presentation state. Retail TalkingHeadUI.lua owns
//! local dismissal; unchanged authoritative sets must not replay dismissed heads.
use shared::protocol::{ActorObject, ConversationPayload, ConversationUpdate};
use std::collections::HashSet;

pub mod glows;
mod presentation;
pub use presentation::TalkingHeadPaint;

const CLOSE_DURATION_MS: f64 = 1000.0;
const VOICE_CLOSE_DELAY_MS: f64 = 1000.0;

#[derive(Clone, Debug, PartialEq)]
pub struct TalkingHeadView {
    pub conversation: u64,
    pub line: u32,
    pub display_id: u32,
    pub camera_id: u32,
    pub name: String,
    pub text: String,
    pub sound_kit: u32,
    pub elapsed_ms: u32,
    pub conversation_elapsed_ms: u32,
    pub texture_kit: u32,
    pub animation_kit: u32,
    pub start_animation: u16,
    pub end_animation: u16,
}

#[derive(Clone, Debug, PartialEq)]
struct Active {
    payload: ConversationPayload,
    elapsed_ms: f64,
}

#[derive(Clone, Debug, PartialEq)]
struct Closing {
    view: TalkingHeadView,
    elapsed_ms: f64,
    delay_ms: f64,
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct TalkingHeadState {
    active: Vec<Active>,
    dismissed: HashSet<u64>,
    closing: Option<Closing>,
}

impl TalkingHeadState {
    pub fn receive(&mut self, update: ConversationUpdate, owner: u64) -> Result<(), String> {
        let previous = self.current();
        if update
            .conversations
            .iter()
            .any(|conversation| conversation.private_owner != owner)
        {
            return Err("ConversationUpdate contains another player's private payload".into());
        }
        for conversation in &update.conversations {
            validate(conversation)?;
        }
        let next = update
            .conversations
            .into_iter()
            .map(|payload| {
                let prior = self
                    .active
                    .iter()
                    .find(|active| active.payload.id == payload.id);
                let elapsed_ms = prior.map_or(payload.elapsed_ms as f64, |active| {
                    active.elapsed_ms.max(payload.elapsed_ms as f64)
                });
                Active {
                    payload,
                    elapsed_ms,
                }
            })
            .collect();
        self.active = next;
        self.dismissed
            .retain(|id| self.active.iter().any(|active| active.payload.id == *id));
        self.sync_close(previous, 0.0);
        Ok(())
    }

    pub fn advance(&mut self, delta: f32) {
        let elapsed = f64::from(delta.max(0.0)) * 1000.0;
        let previous = self.current();
        let until_expiry = previous
            .as_ref()
            .and_then(|view| self.remaining_lifetime(view));
        if let Some(closing) = &mut self.closing {
            closing.elapsed_ms += elapsed;
        }
        for active in &mut self.active {
            active.elapsed_ms += elapsed;
        }
        self.active
            .retain(|active| active.elapsed_ms < active.payload.duration_ms as f64);
        self.sync_close(
            previous,
            until_expiry.map_or(0.0, |remaining| (elapsed - remaining).max(0.0)),
        );
        if self
            .closing
            .as_ref()
            .is_some_and(|closing| closing.elapsed_ms >= closing.delay_ms + CLOSE_DURATION_MS)
        {
            self.closing = None;
        }
    }

    pub fn dismiss(&mut self) {
        self.closing = None;
        if let Some(view) = self.current() {
            self.dismissed.insert(view.conversation);
        }
    }

    pub fn clear(&mut self) {
        self.active.clear();
        self.dismissed.clear();
        self.closing = None;
    }

    pub fn presentation(&self) -> Option<TalkingHeadPaint> {
        presentation::present(self)
    }

    pub fn is_closing(&self) -> bool {
        self.closing.is_some()
    }

    /// TalkingHeadUI.lua Close delays FadeoutFrames by1s when a VO handle exists.
    pub fn defer_close_for_voice(&mut self) {
        if let Some(closing) = &mut self.closing {
            closing.delay_ms = VOICE_CLOSE_DELAY_MS;
        }
    }

    fn remaining_lifetime(&self, view: &TalkingHeadView) -> Option<f64> {
        self.active
            .iter()
            .find(|active| active.payload.id == view.conversation)
            .map(|active| active.payload.duration_ms as f64 - active.elapsed_ms)
    }

    fn sync_close(&mut self, previous: Option<TalkingHeadView>, elapsed_ms: f64) {
        if self.current().is_some() {
            self.closing = None;
        } else if self.closing.is_none() {
            self.closing = previous.map(|view| Closing {
                view,
                elapsed_ms,
                delay_ms: 0.0,
            });
        }
    }

    pub fn current(&self) -> Option<TalkingHeadView> {
        self.active
            .iter()
            .filter(|active| !self.dismissed.contains(&active.payload.id))
            .filter(|active| active.elapsed_ms < active.payload.duration_ms as f64)
            .find_map(current_line)
    }
}

fn validate(conversation: &ConversationPayload) -> Result<(), String> {
    for line in &conversation.lines {
        let actor = conversation
            .actors
            .iter()
            .find(|actor| actor.index == line.actor_index)
            .ok_or_else(|| {
                format!(
                    "Conversation{} line{} lacks source actor{}",
                    conversation.entry, line.id, line.actor_index
                )
            })?;
        if matches!(actor.object, ActorObject::TalkingHead { .. })
            && (actor.name.is_none() || line.text.is_none())
        {
            return Err(format!(
                "Conversation{} line{} lacks source TalkingHead name/text",
                conversation.entry, line.id
            ));
        }
    }
    Ok(())
}

fn current_line(active: &Active) -> Option<TalkingHeadView> {
    let payload = &active.payload;
    let line = payload
        .lines
        .iter()
        .rev()
        .find(|line| line.start_ms as f64 <= active.elapsed_ms)?;
    let actor = payload
        .actors
        .iter()
        .find(|actor| actor.index == line.actor_index)?;
    let ActorObject::TalkingHead { display_id, .. } = actor.object else {
        return None;
    };
    Some(TalkingHeadView {
        conversation: payload.id,
        line: line.id,
        display_id,
        camera_id: line.ui_camera_id,
        name: actor.name.clone()?,
        text: line.text.as_ref()?.male_text.clone(),
        sound_kit: line.text.as_ref()?.sound_kit_id,
        elapsed_ms: (active.elapsed_ms - line.start_ms as f64) as u32,
        conversation_elapsed_ms: active.elapsed_ms as u32,
        texture_kit: payload.texture_kit_id,
        animation_kit: line.animation_kit_id,
        start_animation: line.start_animation,
        end_animation: line.end_animation,
    })
}
