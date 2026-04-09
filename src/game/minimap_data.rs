use bevy::prelude::*;

pub mod textures {
    /// Minimap border ring.
    pub const BORDER: u32 = 136468;
    /// Zoom in button.
    pub const ZOOM_IN: u32 = 136480;
    /// Zoom out button.
    pub const ZOOM_OUT: u32 = 136483;
    /// LFG eye notification icon.
    pub const LFG_EYE: u32 = 136317;
    /// Player arrow (static).
    pub const ARROW: u32 = 136431;
    /// Player arrow (rotating).
    pub const ARROW_ROTATING: u32 = 136443;
}

#[derive(Component, Clone, Copy, Debug, PartialEq, Eq, Default)]
pub struct MinimapHerbNode;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub enum TrackingType {
    #[default]
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

    #[test]
    fn texture_fdids_are_nonzero() {
        assert_ne!(textures::BORDER, 0);
        assert_ne!(textures::ZOOM_IN, 0);
        assert_ne!(textures::ZOOM_OUT, 0);
        assert_ne!(textures::LFG_EYE, 0);
        assert_ne!(textures::ARROW, 0);
    }
}
