//! Retail EncounterTimeline / EncounterWarnings presentation state.
use shared::protocol::{EncounterSeverity, EncounterTimelineEvent, EncounterTimer};

#[derive(Clone, Debug, PartialEq)]
pub struct TimerView {
    pub caster: u64,
    pub timer: EncounterTimer,
    pub remaining: f32,
    pub name: String,
    pub icon: u32,
}
#[derive(Clone, Debug, PartialEq)]
pub struct WarningView {
    pub spell_id: u32,
    pub severity: EncounterSeverity,
    pub name: String,
    pub icon: u32,
    pub remaining: f32,
}
#[derive(Clone, Debug, Default, PartialEq)]
pub struct EncounterTimeline {
    pub timers: Vec<TimerView>,
    pub warnings: Vec<WarningView>,
    pub phase: Option<u32>,
}
impl EncounterTimeline {
    pub fn receive(&mut self, event: EncounterTimelineEvent, spell: impl Fn(u32) -> (String, u32)) {
        match event {
            EncounterTimelineEvent::Timers { caster, timers } => {
                self.timers.retain(|view| view.caster != caster);
                self.timers.extend(timers.into_iter().map(|timer| {
                    let (name, icon) = spell(timer.spell_id);
                    TimerView {
                        caster,
                        remaining: timer.remaining_ms as f32 / 1000.0,
                        timer,
                        name,
                        icon,
                    }
                }));
                self.timers
                    .sort_by(|a, b| a.remaining.total_cmp(&b.remaining));
            }
            EncounterTimelineEvent::Cast {
                spell_id, severity, ..
            } => {
                let (name, icon) = spell(spell_id);
                self.warnings.retain(|warning| warning.severity != severity);
                self.warnings.push(WarningView {
                    spell_id,
                    severity,
                    name,
                    icon,
                    remaining: 3.0,
                });
            }
            EncounterTimelineEvent::Phase { phase, .. } => {
                self.phase = Some(phase);
                self.warnings
                    .retain(|warning| warning.severity != EncounterSeverity::Low);
                self.warnings.push(WarningView {
                    spell_id: 0,
                    severity: EncounterSeverity::Low,
                    name: format!("Phase {phase}"),
                    icon: 0,
                    remaining: 3.0,
                });
            }
        }
    }
    pub fn advance(&mut self, delta: f32) {
        let delta = delta.max(0.0);
        for timer in &mut self.timers {
            timer.remaining = (timer.remaining - delta).max(0.0);
        }
        for warning in &mut self.warnings {
            warning.remaining -= delta;
        }
        self.warnings.retain(|warning| warning.remaining > 0.0);
    }
    pub fn clear(&mut self) {
        *self = Self::default();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn timeline_countdown_replacement_real_cast_warning_and_reset() {
        let mut view = EncounterTimeline::default();
        let timer = EncounterTimer {
            event_id: 2,
            spell_id: 20805,
            remaining_ms: 7312,
            duration_ms: 7312,
            conditional: true,
        };
        let spell = |_: u32| ("Lightning Bolt".into(), 136048);
        view.receive(
            EncounterTimelineEvent::Timers {
                caster: 3669,
                timers: vec![timer.clone()],
            },
            spell,
        );
        assert_eq!(view.timers.len(), 1);
        view.advance(2.0);
        assert!((view.timers[0].remaining - 5.312).abs() < 0.001);
        assert!(view.warnings.is_empty(), "a countdown never invents a cast");
        view.receive(
            EncounterTimelineEvent::Timers {
                caster: 3674,
                timers: vec![EncounterTimer {
                    spell_id: 6254,
                    ..timer.clone()
                }],
            },
            spell,
        );
        view.receive(
            EncounterTimelineEvent::Timers {
                caster: 3669,
                timers: vec![],
            },
            spell,
        );
        assert_eq!(view.timers.len(), 1);
        assert_eq!(view.timers[0].caster, 3674);
        view.receive(
            EncounterTimelineEvent::Cast {
                caster: 3669,
                spell_id: 20805,
                severity: EncounterSeverity::Low,
            },
            spell,
        );
        assert_eq!(view.warnings[0].name, "Lightning Bolt");
        assert_eq!(view.warnings[0].icon, 136048);
        view.receive(
            EncounterTimelineEvent::Phase {
                caster: 3669,
                phase: 2,
            },
            spell,
        );
        assert_eq!(view.phase, Some(2));
        assert_eq!(view.warnings.last().unwrap().name, "Phase 2");
        view.advance(10.0);
        assert!(view.warnings.is_empty());
        view.clear();
        assert!(view.timers.is_empty());
        assert_eq!(view.phase, None);
    }
}
