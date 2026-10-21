//! Private source conversation presentation state. Retail TalkingHeadUI.lua owns
//! local dismissal; unchanged authoritative sets must not replay dismissed heads.
use shared::protocol::{ActorObject, ConversationPayload, ConversationUpdate};
use std::collections::HashSet;

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

#[derive(Clone, Debug, Default, PartialEq)]
pub struct TalkingHeadState {
    active: Vec<Active>,
    dismissed: HashSet<u64>,
}

impl TalkingHeadState {
    pub fn receive(&mut self, update: ConversationUpdate, owner: u64) -> Result<(), String> {
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
        Ok(())
    }

    pub fn advance(&mut self, delta: f32) {
        for active in &mut self.active {
            active.elapsed_ms += f64::from(delta.max(0.0)) * 1000.0;
        }
        self.active
            .retain(|active| active.elapsed_ms < active.payload.duration_ms as f64);
    }

    pub fn dismiss(&mut self) {
        if let Some(view) = self.current() {
            self.dismissed.insert(view.conversation);
        }
    }

    pub fn clear(&mut self) {
        self.active.clear();
        self.dismissed.clear();
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
        texture_kit: payload.texture_kit_id,
        animation_kit: line.animation_kit_id,
        start_animation: line.start_animation,
        end_animation: line.end_animation,
    })
}
