//! Private source conversation presentation state. Retail TalkingHeadUI.lua owns
//! local dismissal; unchanged authoritative sets must not replay dismissed heads.
use shared::protocol::ConversationUpdate;

#[derive(Clone, Debug, PartialEq)]
pub struct TalkingHeadView {
    pub conversation: u64,
    pub line: u32,
    pub display_id: u32,
    pub camera_id: u32,
    pub name: String,
    pub text: String,
    pub sound_kit: u32,
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct TalkingHeadState;

impl TalkingHeadState {
    pub fn receive(&mut self, _update: ConversationUpdate, _owner: u64) -> Result<(), String> {
        Ok(())
    }
    pub fn advance(&mut self, _delta: f32) {}
    pub fn dismiss(&mut self) {}
    pub fn current(&self) -> Option<TalkingHeadView> {
        None
    }
}
