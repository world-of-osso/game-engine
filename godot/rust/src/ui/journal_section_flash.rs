//! EncounterInfoTemplate.flashAnim (Retail EJ XML764–768), started by FocusSection.
use std::time::Instant;

use godot::classes::Control;
use godot::prelude::*;

use super::RegistryUi;

// Source order/durations; completion restores the Glow frame's authored zero alpha.
const PHASES: &[(f64, f32, f32)] = &[
    (0.6, 0.0, 1.0),
    (0.6, 1.0, 0.0),
    (0.6, 0.0, 1.0),
    (0.9, 1.0, 0.0),
];

fn section_flash_alpha(elapsed: f64) -> Option<f32> {
    let mut remaining = elapsed.max(0.0);
    for &(duration, from, to) in PHASES {
        if remaining < duration {
            return Some(from + (to - from) * (remaining / duration) as f32);
        }
        remaining -= duration;
    }
    None
}

impl RegistryUi {
    pub(super) fn start_journal_section_flash(&mut self, section: u32) -> Result<(), String> {
        if let Some((previous, _)) = self.section_flash.take()
            && let Some(mut glow) = self.find_journal_section_glow(previous)
        {
            glow.set_modulate(Color::from_rgba(1.0, 1.0, 1.0, 0.0));
        }
        let mut glow = self
            .find_journal_section_glow(section)
            .ok_or_else(|| format!("Journal focus target {section} has no Glow frame"))?;
        glow.set_modulate(Color::from_rgba(1.0, 1.0, 1.0, 0.0));
        self.section_flash = Some((section, Instant::now()));
        Ok(())
    }

    pub(super) fn advance_journal_section_flash(&mut self) {
        let Some((section, start)) = self.section_flash else {
            return;
        };
        let Some(mut glow) = self.find_journal_section_glow(section) else {
            // Changing page/encounter removes the old used header and its animation.
            self.section_flash = None;
            return;
        };
        let alpha = section_flash_alpha(start.elapsed().as_secs_f64());
        glow.set_modulate(Color::from_rgba(1.0, 1.0, 1.0, alpha.unwrap_or(0.0)));
        if alpha.is_none() {
            self.section_flash = None;
        }
    }

    fn find_journal_section_glow(&self, section: u32) -> Option<Gd<Control>> {
        let id = self
            .model
            .as_ref()?
            .registry
            .get_by_name(&format!("JournalSection{section}Glow"))?;
        self.projection.as_ref()?.node(id)
    }
}

#[cfg(test)]
mod tests {
    use super::section_flash_alpha;

    #[test]
    fn adventure_section_flash_source_phases_and_completion() {
        for (elapsed, expected) in [
            (0.0, 0.0),
            (0.3, 0.5),
            (0.6, 1.0),
            (0.9, 0.5),
            (1.2, 0.0),
            (1.5, 0.5),
            (1.8, 1.0),
            (2.25, 0.5),
        ] {
            let actual = section_flash_alpha(elapsed).unwrap();
            assert!((actual - expected).abs() < 0.00001, "{elapsed}: {actual}");
        }
        assert_eq!(section_flash_alpha(2.7), None);
        assert_eq!(section_flash_alpha(3.5), None);
    }
}
