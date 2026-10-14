//! RewardTrackFrameMixin: source timing, fractional offset and directional release snap.
use super::Journeys;

pub(super) const LEVEL_SPACING: f32 = 53.;

#[derive(Clone, Debug, PartialEq)]
pub struct Scroll {
    elapsed: f32,
    offset: f32,
    direction: i32,
    selected: u32,
    moving: bool,
}

impl Journeys {
    pub fn is_scrolling(&self) -> bool {
        self.scroll.is_some()
    }

    pub fn track_selected_index(&self) -> u32 {
        self.scroll
            .as_ref()
            .map_or(self.level, |scroll| scroll.selected)
    }

    pub fn track_offset(&self) -> f32 {
        self.scroll.as_ref().map_or(
            self.level.saturating_sub(1) as f32 * LEVEL_SPACING,
            |scroll| scroll.offset,
        )
    }

    pub fn start_scroll(&mut self, direction: i32) {
        assert!(matches!(direction, -1 | 1), "Renown arrow direction");
        self.scroll = Some(Scroll {
            elapsed: 0.,
            offset: self.track_offset(),
            direction,
            selected: self.level,
            moving: false,
        });
    }

    pub fn advance_scroll(&mut self, elapsed: f32, max: u32) {
        let Some(scroll) = &mut self.scroll else {
            return;
        };
        scroll.elapsed += elapsed;
        let speed = match scroll.elapsed {
            time if time >= 2.6 => 4.,
            time if time >= 1.6 => 3.,
            time if time >= 0.6 => 2.,
            _ => 0.,
        };
        scroll.moving = speed > 0.;
        if !scroll.moving {
            return;
        }
        let maximum = max.saturating_sub(1) as f32 * LEVEL_SPACING;
        // Retail adds speed * total held time per update, not speed * elapsed.
        scroll.offset =
            (scroll.offset + speed * scroll.direction as f32 * scroll.elapsed).clamp(0., maximum);
        self.level = (scroll.offset / LEVEL_SPACING).floor() as u32 + 1;
        let boundary = scroll.offset == 0. || scroll.offset == maximum;
        if boundary {
            self.stop_scroll(max);
        }
    }

    pub fn stop_scroll(&mut self, max: u32) {
        let Some(scroll) = self.scroll.take() else {
            return;
        };
        self.level = if !scroll.moving {
            (scroll.selected as i64 + scroll.direction as i64).clamp(1, max as i64) as u32
        } else {
            let center = (scroll.offset / LEVEL_SPACING).floor() as u32 + 1;
            let remainder = scroll.offset % LEVEL_SPACING;
            let next = remainder >= 1. && scroll.direction == 1;
            (center + u32::from(next)).min(max)
        };
    }
}
