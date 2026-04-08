use bevy::prelude::*;

/// Built-in chat channel type.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum ChatChannelType {
    Say,
    Yell,
    Party,
    Raid,
    Guild,
    Officer,
    Whisper,
    System,
    /// Numbered custom or zone channel (General, Trade, LookingForGroup, etc.).
    Custom,
}

impl ChatChannelType {
    /// Display color as RGBA for this channel type.
    pub fn color(self) -> [f32; 4] {
        match self {
            Self::Say => [1.0, 1.0, 1.0, 1.0],        // white
            Self::Yell => [1.0, 0.25, 0.25, 1.0],     // red
            Self::Party => [0.67, 0.67, 1.0, 1.0],    // light blue
            Self::Raid => [1.0, 0.5, 0.0, 1.0],       // orange
            Self::Guild => [0.25, 1.0, 0.25, 1.0],    // green
            Self::Officer => [0.25, 0.75, 0.25, 1.0], // dark green
            Self::Whisper => [1.0, 0.5, 1.0, 1.0],    // pink
            Self::System => [1.0, 1.0, 0.0, 1.0],     // yellow
            Self::Custom => [1.0, 0.75, 0.75, 1.0],   // light pink
        }
    }

    /// Prefix shown before messages (e.g. "[Party]").
    pub fn prefix(self) -> &'static str {
        match self {
            Self::Say => "[Say]",
            Self::Yell => "[Yell]",
            Self::Party => "[Party]",
            Self::Raid => "[Raid]",
            Self::Guild => "[Guild]",
            Self::Officer => "[Officer]",
            Self::Whisper => "[Whisper]",
            Self::System => "[System]",
            Self::Custom => "",
        }
    }
}

/// A joined chat channel with its display number and name.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct JoinedChannel {
    /// Channel display number (1-based, e.g. 1=General, 2=Trade).
    pub number: u32,
    pub name: String,
    pub channel_type: ChatChannelType,
}

/// A single chat message in the chat log.
#[derive(Clone, Debug, PartialEq)]
pub struct ChatMessage {
    pub channel_type: ChatChannelType,
    /// Channel name for custom channels, empty for system channels.
    pub channel_name: String,
    pub sender: String,
    pub text: String,
    pub timestamp: f64,
}

impl ChatMessage {
    /// Formatted display: "[Channel] Sender: text" or "Sender says: text".
    pub fn formatted(&self) -> String {
        let prefix = self.channel_type.prefix();
        if prefix.is_empty() && !self.channel_name.is_empty() {
            format!("[{}] {}: {}", self.channel_name, self.sender, self.text)
        } else if prefix.is_empty() {
            format!("{}: {}", self.sender, self.text)
        } else {
            format!("{} {}: {}", prefix, self.sender, self.text)
        }
    }
}

/// Runtime chat state.
#[derive(Resource, Clone, Debug, PartialEq, Default)]
pub struct ChatState {
    pub joined_channels: Vec<JoinedChannel>,
    pub messages: Vec<ChatMessage>,
    pub max_messages: usize,
}

impl ChatState {
    pub fn add_message(&mut self, msg: ChatMessage) {
        self.messages.push(msg);
        if self.max_messages > 0 && self.messages.len() > self.max_messages {
            self.messages.remove(0);
        }
    }

    /// Find a joined channel by number.
    pub fn channel_by_number(&self, number: u32) -> Option<&JoinedChannel> {
        self.joined_channels.iter().find(|c| c.number == number)
    }

    /// Find a joined channel by name (case-insensitive).
    pub fn channel_by_name(&self, name: &str) -> Option<&JoinedChannel> {
        let lower = name.to_lowercase();
        self.joined_channels
            .iter()
            .find(|c| c.name.to_lowercase() == lower)
    }

    /// Join a channel. Returns false if already joined.
    pub fn join(&mut self, channel: JoinedChannel) -> bool {
        if self.channel_by_name(&channel.name).is_some() {
            return false;
        }
        self.joined_channels.push(channel);
        true
    }

    /// Leave a channel by number. Returns the removed channel.
    pub fn leave(&mut self, number: u32) -> Option<JoinedChannel> {
        let idx = self
            .joined_channels
            .iter()
            .position(|c| c.number == number)?;
        Some(self.joined_channels.remove(idx))
    }

    /// Next available channel number (first unused starting from 1).
    pub fn next_channel_number(&self) -> u32 {
        let mut n = 1;
        while self.joined_channels.iter().any(|c| c.number == n) {
            n += 1;
        }
        n
    }
}

// --- Client → server intents ---

/// A pending chat action to send to the server.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ChatIntent {
    /// Join a channel by name.
    JoinChannel { name: String },
    /// Leave a channel by number.
    LeaveChannel { number: u32 },
    /// Send a message to a specific channel type.
    SendMessage {
        channel_type: ChatChannelType,
        /// Target for whisper (player name) or custom channel name.
        target: String,
        text: String,
    },
}

/// Queue of chat intents waiting to be sent to the server.
#[derive(Resource, Default)]
pub struct ChatIntentQueue {
    pub pending: Vec<ChatIntent>,
}

impl ChatIntentQueue {
    pub fn join_channel(&mut self, name: String) {
        self.pending.push(ChatIntent::JoinChannel { name });
    }

    pub fn leave_channel(&mut self, number: u32) {
        self.pending.push(ChatIntent::LeaveChannel { number });
    }

    pub fn send_message(&mut self, channel_type: ChatChannelType, target: String, text: String) {
        self.pending.push(ChatIntent::SendMessage {
            channel_type,
            target,
            text,
        });
    }

    pub fn drain(&mut self) -> Vec<ChatIntent> {
        std::mem::take(&mut self.pending)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn general_channel() -> JoinedChannel {
        JoinedChannel {
            number: 1,
            name: "General".into(),
            channel_type: ChatChannelType::Custom,
        }
    }

    fn trade_channel() -> JoinedChannel {
        JoinedChannel {
            number: 2,
            name: "Trade".into(),
            channel_type: ChatChannelType::Custom,
        }
    }

    fn msg(channel: ChatChannelType, sender: &str, text: &str) -> ChatMessage {
        ChatMessage {
            channel_type: channel,
            channel_name: String::new(),
            sender: sender.into(),
            text: text.into(),
            timestamp: 0.0,
        }
    }

    // --- ChatChannelType ---

    #[test]
    fn channel_colors_distinct() {
        assert_ne!(ChatChannelType::Say.color(), ChatChannelType::Yell.color());
        assert_ne!(
            ChatChannelType::Guild.color(),
            ChatChannelType::Party.color()
        );
    }

    #[test]
    fn channel_prefixes() {
        assert_eq!(ChatChannelType::Party.prefix(), "[Party]");
        assert_eq!(ChatChannelType::Guild.prefix(), "[Guild]");
        assert_eq!(ChatChannelType::Custom.prefix(), "");
    }

    // --- ChatMessage ---

    #[test]
    fn message_formatted_system_channel() {
        let m = msg(ChatChannelType::Party, "Alice", "Hello");
        assert_eq!(m.formatted(), "[Party] Alice: Hello");
    }

    #[test]
    fn message_formatted_custom_channel() {
        let m = ChatMessage {
            channel_type: ChatChannelType::Custom,
            channel_name: "Trade".into(),
            sender: "Bob".into(),
            text: "WTS Ore".into(),
            timestamp: 0.0,
        };
        assert_eq!(m.formatted(), "[Trade] Bob: WTS Ore");
    }

    // --- ChatState ---

    #[test]
    fn join_channel() {
        let mut state = ChatState::default();
        assert!(state.join(general_channel()));
        assert_eq!(state.joined_channels.len(), 1);
    }

    #[test]
    fn join_duplicate_rejected() {
        let mut state = ChatState::default();
        state.join(general_channel());
        assert!(!state.join(JoinedChannel {
            number: 5,
            name: "General".into(),
            channel_type: ChatChannelType::Custom,
        }));
        assert_eq!(state.joined_channels.len(), 1);
    }

    #[test]
    fn join_case_insensitive_duplicate() {
        let mut state = ChatState::default();
        state.join(general_channel());
        assert!(!state.join(JoinedChannel {
            number: 5,
            name: "general".into(),
            channel_type: ChatChannelType::Custom,
        }));
    }

    #[test]
    fn leave_channel() {
        let mut state = ChatState::default();
        state.join(general_channel());
        state.join(trade_channel());
        let removed = state.leave(1);
        assert_eq!(removed.unwrap().name, "General");
        assert_eq!(state.joined_channels.len(), 1);
    }

    #[test]
    fn leave_nonexistent() {
        let mut state = ChatState::default();
        assert!(state.leave(99).is_none());
    }

    #[test]
    fn channel_by_number() {
        let mut state = ChatState::default();
        state.join(general_channel());
        state.join(trade_channel());
        assert_eq!(state.channel_by_number(2).unwrap().name, "Trade");
        assert!(state.channel_by_number(99).is_none());
    }

    #[test]
    fn channel_by_name() {
        let mut state = ChatState::default();
        state.join(general_channel());
        assert!(state.channel_by_name("General").is_some());
        assert!(state.channel_by_name("general").is_some());
        assert!(state.channel_by_name("Unknown").is_none());
    }

    #[test]
    fn next_channel_number() {
        let mut state = ChatState::default();
        assert_eq!(state.next_channel_number(), 1);
        state.join(general_channel()); // number=1
        assert_eq!(state.next_channel_number(), 2);
        state.join(trade_channel()); // number=2
        assert_eq!(state.next_channel_number(), 3);
    }

    #[test]
    fn next_channel_number_fills_gap() {
        let mut state = ChatState::default();
        state.join(general_channel()); // number=1
        state.join(JoinedChannel {
            number: 3,
            name: "LFG".into(),
            channel_type: ChatChannelType::Custom,
        });
        assert_eq!(state.next_channel_number(), 2);
    }

    #[test]
    fn add_message_trims() {
        let mut state = ChatState {
            max_messages: 3,
            ..Default::default()
        };
        for i in 0..5 {
            state.add_message(msg(ChatChannelType::Say, "A", &format!("{i}")));
        }
        assert_eq!(state.messages.len(), 3);
        assert_eq!(state.messages[0].text, "2");
    }

    // --- ChatIntentQueue ---

    #[test]
    fn intent_join() {
        let mut queue = ChatIntentQueue::default();
        queue.join_channel("Trade".into());
        let drained = queue.drain();
        assert_eq!(
            drained[0],
            ChatIntent::JoinChannel {
                name: "Trade".into()
            }
        );
    }

    #[test]
    fn intent_leave() {
        let mut queue = ChatIntentQueue::default();
        queue.leave_channel(2);
        let drained = queue.drain();
        assert_eq!(drained[0], ChatIntent::LeaveChannel { number: 2 });
    }

    #[test]
    fn intent_send() {
        let mut queue = ChatIntentQueue::default();
        queue.send_message(ChatChannelType::Say, String::new(), "Hello".into());
        let drained = queue.drain();
        assert_eq!(
            drained[0],
            ChatIntent::SendMessage {
                channel_type: ChatChannelType::Say,
                target: String::new(),
                text: "Hello".into(),
            }
        );
    }

    #[test]
    fn intent_drain_clears() {
        let mut queue = ChatIntentQueue::default();
        queue.join_channel("A".into());
        queue.leave_channel(1);
        assert_eq!(queue.drain().len(), 2);
        assert!(queue.pending.is_empty());
    }
}
