use bevy::prelude::*;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PVPBracket {
    Arena2v2,
    Arena3v3,
    RatedBG,
    SoloShuffle,
}

impl PVPBracket {
    pub fn label(self) -> &'static str {
        match self {
            Self::Arena2v2 => "2v2",
            Self::Arena3v3 => "3v3",
            Self::RatedBG => "Rated BG",
            Self::SoloShuffle => "Solo Shuffle",
        }
    }

    pub const ALL: [PVPBracket; 4] = [
        Self::Arena2v2,
        Self::Arena3v3,
        Self::RatedBG,
        Self::SoloShuffle,
    ];
}

#[derive(Clone, Debug, PartialEq, Default)]
pub struct BracketStats {
    pub rating: u32,
    pub season_wins: u32,
    pub season_losses: u32,
    pub weekly_wins: u32,
    pub weekly_losses: u32,
}

impl BracketStats {
    pub fn season_record(&self) -> String {
        format!("{} - {}", self.season_wins, self.season_losses)
    }

    pub fn win_rate(&self) -> f32 {
        let total = self.season_wins + self.season_losses;
        if total == 0 {
            return 0.0;
        }
        self.season_wins as f32 / total as f32
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub enum QueueState {
    #[default]
    Idle,
    Queued,
    InProgress,
}

/// Runtime PvP state.
#[derive(Resource, Clone, Debug, PartialEq, Default)]
pub struct PVPState {
    pub honor: u32,
    pub honor_max: u32,
    pub conquest: u32,
    pub conquest_max: u32,
    pub brackets: [BracketStats; 4],
    pub queue: QueueState,
}

impl PVPState {
    pub fn bracket(&self, b: PVPBracket) -> &BracketStats {
        &self.brackets[PVPBracket::ALL.iter().position(|&x| x == b).unwrap()]
    }

    pub fn is_queued(&self) -> bool {
        matches!(self.queue, QueueState::Queued)
    }

    pub fn honor_text(&self) -> String {
        format!("{}/{}", self.honor, self.honor_max)
    }

    pub fn conquest_text(&self) -> String {
        format!("{}/{}", self.conquest, self.conquest_max)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn bracket_labels() {
        assert_eq!(PVPBracket::Arena2v2.label(), "2v2");
        assert_eq!(PVPBracket::SoloShuffle.label(), "Solo Shuffle");
    }

    #[test]
    fn season_record_and_win_rate() {
        let stats = BracketStats {
            rating: 1800,
            season_wins: 30,
            season_losses: 20,
            ..Default::default()
        };
        assert_eq!(stats.season_record(), "30 - 20");
        assert!((stats.win_rate() - 0.6).abs() < 0.01);
    }

    #[test]
    fn win_rate_zero_games() {
        let stats = BracketStats::default();
        assert_eq!(stats.win_rate(), 0.0);
    }

    #[test]
    fn bracket_access() {
        let mut state = PVPState::default();
        state.brackets[1].rating = 2100;
        assert_eq!(state.bracket(PVPBracket::Arena3v3).rating, 2100);
    }

    #[test]
    fn queue_state() {
        let mut state = PVPState::default();
        assert!(!state.is_queued());
        state.queue = QueueState::Queued;
        assert!(state.is_queued());
    }

    #[test]
    fn currency_text() {
        let state = PVPState {
            honor: 500,
            honor_max: 15000,
            conquest: 800,
            conquest_max: 1800,
            ..Default::default()
        };
        assert_eq!(state.honor_text(), "500/15000");
        assert_eq!(state.conquest_text(), "800/1800");
    }
}
