use bevy::prelude::*;

pub mod textures {
    /// PvP queue frame sheet.
    pub const QUEUE_FRAME: u32 = 2123218;
    /// Alliance faction emblem.
    pub const ALLIANCE_EMBLEM: u32 = 136998;
    /// Horde faction emblem.
    pub const HORDE_EMBLEM: u32 = 137000;
    /// Alliance queue background.
    pub const QUEUE_BG_ALLIANCE: u32 = 1405824;
    /// Honor bar fill.
    pub const HONOR_BAR_FILL: u32 = 2131913;
}

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

    #[test]
    fn texture_fdids_are_nonzero() {
        assert_ne!(textures::QUEUE_FRAME, 0);
        assert_ne!(textures::ALLIANCE_EMBLEM, 0);
        assert_ne!(textures::HORDE_EMBLEM, 0);
        assert_ne!(textures::HONOR_BAR_FILL, 0);
    }

    // --- Bracket rating ---

    #[test]
    fn all_brackets_accessible() {
        let mut state = PVPState::default();
        state.brackets[0].rating = 1500;
        state.brackets[1].rating = 2100;
        state.brackets[2].rating = 1600;
        state.brackets[3].rating = 1900;
        assert_eq!(state.bracket(PVPBracket::Arena2v2).rating, 1500);
        assert_eq!(state.bracket(PVPBracket::Arena3v3).rating, 2100);
        assert_eq!(state.bracket(PVPBracket::RatedBG).rating, 1600);
        assert_eq!(state.bracket(PVPBracket::SoloShuffle).rating, 1900);
    }

    #[test]
    fn win_rate_all_wins() {
        let stats = BracketStats {
            season_wins: 50,
            season_losses: 0,
            ..Default::default()
        };
        assert!((stats.win_rate() - 1.0).abs() < 0.01);
    }

    #[test]
    fn win_rate_all_losses() {
        let stats = BracketStats {
            season_wins: 0,
            season_losses: 30,
            ..Default::default()
        };
        assert_eq!(stats.win_rate(), 0.0);
    }

    #[test]
    fn weekly_record_separate_from_season() {
        let stats = BracketStats {
            rating: 1800,
            season_wins: 100,
            season_losses: 50,
            weekly_wins: 10,
            weekly_losses: 5,
        };
        assert_eq!(stats.season_record(), "100 - 50");
        // Weekly is tracked independently
        assert_eq!(stats.weekly_wins, 10);
        assert_eq!(stats.weekly_losses, 5);
    }

    // --- Queue state transitions ---

    #[test]
    fn queue_state_transitions() {
        let mut state = PVPState::default();
        assert_eq!(state.queue, QueueState::Idle);
        assert!(!state.is_queued());

        state.queue = QueueState::Queued;
        assert!(state.is_queued());

        state.queue = QueueState::InProgress;
        assert!(!state.is_queued()); // in_progress is not "queued"
    }

    #[test]
    fn queue_in_progress_not_idle() {
        let state = PVPState {
            queue: QueueState::InProgress,
            ..Default::default()
        };
        assert!(!state.is_queued());
        assert!(!matches!(state.queue, QueueState::Idle));
    }

    // --- Currency tracking ---

    #[test]
    fn honor_at_cap() {
        let state = PVPState {
            honor: 15000,
            honor_max: 15000,
            ..Default::default()
        };
        assert_eq!(state.honor_text(), "15000/15000");
    }

    #[test]
    fn conquest_at_zero() {
        let state = PVPState {
            conquest: 0,
            conquest_max: 1800,
            ..Default::default()
        };
        assert_eq!(state.conquest_text(), "0/1800");
    }

    #[test]
    fn default_state_zeroed() {
        let state = PVPState::default();
        assert_eq!(state.honor, 0);
        assert_eq!(state.conquest, 0);
        assert_eq!(state.queue, QueueState::Idle);
        for b in &state.brackets {
            assert_eq!(b.rating, 0);
        }
    }
}
