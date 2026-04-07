use bevy::prelude::*;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TrackingType {
    None,
    Herbs,
    Minerals,
    Fish,
    Beasts,
    Humanoids,
    Undead,
}

impl TrackingType {
    pub fn label(self) -> &'static str {
        match self {
            Self::None => "None",
            Self::Herbs => "Find Herbs",
            Self::Minerals => "Find Minerals",
            Self::Fish => "Find Fish",
            Self::Beasts => "Track Beasts",
            Self::Humanoids => "Track Humanoids",
            Self::Undead => "Track Undead",
        }
    }
}

impl Default for TrackingType {
    fn default() -> Self {
        Self::None
    }
}

/// Minimap notification flags for button indicators.
#[derive(Clone, Debug, PartialEq, Default)]
pub struct MinimapNotifications {
    pub has_mail: bool,
    pub calendar_event: bool,
    pub lfg_proposal: bool,
}

/// Runtime minimap UI state (beyond the render texture itself).
#[derive(Resource, Clone, Debug, PartialEq, Default)]
pub struct MinimapUIState {
    pub zone_name: String,
    pub zone_id: u32,
    pub player_x: f32,
    pub player_y: f32,
    pub tracking: TrackingType,
    pub notifications: MinimapNotifications,
    pub zoom_level: f32,
}

impl MinimapUIState {
    pub fn coords_text(&self) -> String {
        format!("{:.1}, {:.1}", self.player_x, self.player_y)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tracking_labels() {
        assert_eq!(TrackingType::None.label(), "None");
        assert_eq!(TrackingType::Herbs.label(), "Find Herbs");
        assert_eq!(TrackingType::Minerals.label(), "Find Minerals");
    }

    #[test]
    fn coords_text_format() {
        let state = MinimapUIState {
            player_x: 42.5,
            player_y: 63.2,
            ..Default::default()
        };
        assert_eq!(state.coords_text(), "42.5, 63.2");
    }

    #[test]
    fn default_state() {
        let state = MinimapUIState::default();
        assert!(state.zone_name.is_empty());
        assert_eq!(state.tracking, TrackingType::None);
        assert!(!state.notifications.has_mail);
    }

    #[test]
    fn notifications() {
        let n = MinimapNotifications {
            has_mail: true,
            calendar_event: false,
            lfg_proposal: true,
        };
        assert!(n.has_mail);
        assert!(n.lfg_proposal);
        assert!(!n.calendar_event);
    }
}
