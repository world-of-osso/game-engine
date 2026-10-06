//! Retail death dialogs over the realm's death snapshot. See docs/specs/death-flow.md.
use crate::popup::{PopupOutcome, PopupResult, PopupSpec, PopupStack};
use shared::protocol::{
    DeathPositionSnapshot, DeathSnapshot, DeathStateSnapshot, DeathStateUpdate,
};

pub const DEATH_POPUP: &str = "DEATH";
pub const CORPSE_POPUP: &str = "RECOVER_CORPSE";
pub const HEALER_POPUP: &str = "XP_LOSS";
const DEATH_KEYS: [&str; 3] = [DEATH_POPUP, CORPSE_POPUP, HEALER_POPUP];

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DeathRequest {
    Release,
    Corpse,
    SpiritHealer,
}

#[derive(Default)]
pub struct DeathFlow {
    pub snapshot: Option<DeathSnapshot>,
    active: Option<&'static str>,
    pending: bool,
    dismissed_healer: bool,
}
impl DeathFlow {
    /// Error-only replies retain the last snapshot and unlock retry.
    pub fn receive(&mut self, update: DeathStateUpdate) -> Option<String> {
        if let Some(snapshot) = update.snapshot {
            self.snapshot = Some(snapshot);
        }
        self.pending = false;
        update.error
    }

    pub fn is_ghost(&self) -> bool {
        self.snapshot
            .as_ref()
            .is_some_and(|snapshot| snapshot.state == DeathStateSnapshot::Ghost)
    }

    pub fn corpse(&self) -> Option<&DeathPositionSnapshot> {
        self.snapshot
            .as_ref()
            .filter(|_| self.is_ghost())?
            .corpse
            .as_ref()
    }

    /// Server range booleans are only sampled on requests, not on movement. Use the
    /// supplied positions and shared server range constants; server validates acceptance.
    fn popup_key(&self, position: Option<&DeathPositionSnapshot>) -> Option<&'static str> {
        let snapshot = self.snapshot.as_ref()?;
        match snapshot.state {
            DeathStateSnapshot::Dead => Some(DEATH_POPUP),
            DeathStateSnapshot::Ghost => {
                let position = position?;
                if in_range(
                    position,
                    snapshot.corpse.as_ref(),
                    shared::death::CORPSE_RESURRECT_RANGE,
                ) {
                    Some(CORPSE_POPUP)
                } else if in_range(
                    position,
                    snapshot.graveyard.as_ref(),
                    shared::death::SPIRIT_HEALER_RANGE,
                ) {
                    Some(HEALER_POPUP)
                } else {
                    None
                }
            }
            DeathStateSnapshot::Alive | DeathStateSnapshot::Resurrecting => None,
        }
    }

    pub fn sync_popups(
        &mut self,
        popups: &mut PopupStack,
        position: Option<&DeathPositionSnapshot>,
        level: u8,
    ) {
        let key = self.popup_key(position);
        if key != Some(HEALER_POPUP) {
            self.dismissed_healer = false;
        }
        self.active = key;
        for stale in DEATH_KEYS
            .into_iter()
            .filter(|candidate| Some(*candidate) != key)
        {
            popups.hide(stale);
        }
        let Some(key) = key else { return };
        if self.pending || (key == HEALER_POPUP && self.dismissed_healer) {
            popups.hide(key);
        } else if !popups.contains(key) {
            popups.push(popup_spec(key, level));
        }
    }

    pub fn popup_results(&mut self, results: &[PopupResult]) -> Option<DeathRequest> {
        if self.pending {
            return None;
        }
        let key = self.active?;
        let result = results.iter().find(|result| result.key == key)?;
        if result.outcome != PopupOutcome::Accepted {
            self.dismissed_healer = key == HEALER_POPUP;
            return None;
        }
        self.pending = true;
        match key {
            DEATH_POPUP => Some(DeathRequest::Release),
            CORPSE_POPUP => Some(DeathRequest::Corpse),
            HEALER_POPUP => Some(DeathRequest::SpiritHealer),
            _ => unreachable!("death flow selects only death popup keys"),
        }
    }
}

fn in_range(
    position: &DeathPositionSnapshot,
    destination: Option<&DeathPositionSnapshot>,
    radius: f32,
) -> bool {
    let Some(destination) = destination else {
        return false;
    };
    let distance_squared = (position.x - destination.x).powi(2)
        + (position.y - destination.y).powi(2)
        + (position.z - destination.z).powi(2);
    position.map_id == destination.map_id && distance_squared <= radius * radius
}

fn popup_spec(key: &str, level: u8) -> PopupSpec {
    // GlobalStrings.csv:4876,430,1912,169,171; Mainline/GameDialogDefs.lua:183,
    // GameDialogDefs.lua:2031 and Mainline/GameDialogDefs.lua:1050.
    let (text, accept, cancel) = match key {
        DEATH_POPUP => (
            "You have died. Release to the nearest graveyard?".into(),
            "Release Spirit",
            None,
        ),
        CORPSE_POPUP => ("Resurrect now?".into(), "Accept", None),
        HEALER_POPUP => (healer_text(level), "Accept", Some("Cancel")),
        _ => unreachable!("known death dialog"),
    };
    PopupSpec {
        key: key.into(),
        text,
        accept_label: accept.into(),
        cancel_label: cancel.map(str::to_owned),
        timeout: None,
        confirm_text: None,
    }
}

fn healer_text(level: u8) -> String {
    // Retail CONFIRM_XP_LOSS/NO_SICKNESS (GlobalStrings.csv:1910,4231), formatted
    // with pinned realm's 25% loss and ten-minute sickness, rather than promising 50%.
    if level < shared::death::RES_SICKNESS_MIN_LEVEL {
        "If you find your corpse, you can resurrect for no penalty.  If I resurrect you all of your items will take 25% durability damage (equipped and inventory).".into()
    } else {
        "If you find your corpse, you can resurrect for no penalty.  If I resurrect you all of your items will take 25% durability damage (equipped and inventory) and you will be afflicted by 10 minutes of |cff71d5ff|Hspell:15007|h[Resurrection Sickness]|h|r.".into()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::static_popup_component::{
        StaticPopupState, parse_popup_action, static_popup_screen,
    };
    use shared::protocol::DeathStateSnapshot;
    use ui_toolkit::{
        atlas::{ActiveSkin, set_thread_skin},
        registry::FrameRegistry,
        screen::{Screen, SharedContext},
    };

    fn at(x: f32) -> DeathPositionSnapshot {
        DeathPositionSnapshot {
            map_id: 0,
            x,
            y: 3.0,
            z: -7.0,
        }
    }
    fn update(state: DeathStateSnapshot) -> DeathStateUpdate {
        DeathStateUpdate {
            snapshot: Some(DeathSnapshot {
                state,
                corpse: Some(at(12.0)),
                graveyard: Some(at(200.0)),
                can_resurrect_at_corpse: false,
                spirit_healer_available: false,
            }),
            message: None,
            error: None,
        }
    }
    fn draw_and_accept(
        stack: &mut PopupStack,
        skin: ActiveSkin,
        text: &str,
        label: &str,
    ) -> Vec<PopupResult> {
        set_thread_skin(skin);
        let mut registry = FrameRegistry::new(1920.0, 1080.0);
        let mut shared = SharedContext::new();
        shared.insert(StaticPopupState {
            popups: stack.visible(),
        });
        Screen::new(static_popup_screen).sync(&shared, &mut registry);
        assert_eq!(
            crate::ui::screens::screen_test_helpers::fontstring_text(&registry, "StaticPopup1Text"),
            text
        );
        let button = registry
            .get(
                registry
                    .get_by_name("StaticPopup1Button1")
                    .expect("accept button"),
            )
            .unwrap();
        let action = button.onclick.as_deref().expect("click action");
        let (id, outcome) = parse_popup_action(action).expect("popup action");
        assert_eq!(stack.visible()[0].spec.accept_label, label);
        assert!(stack.resolve(id, outcome));
        stack.drain_results()
    }
    #[test]
    fn deathstate_dead_popup_release_both_skins() {
        for skin in [ActiveSkin::Modern, ActiveSkin::Forever] {
            let mut flow = DeathFlow::default();
            let mut stack = PopupStack::default();
            flow.receive(update(DeathStateSnapshot::Dead));
            flow.sync_popups(&mut stack, Some(&at(12.0)), 60);
            assert!(stack.contains("DEATH"));
            let results = draw_and_accept(
                &mut stack,
                skin,
                "You have died. Release to the nearest graveyard?",
                "Release Spirit",
            );
            assert_eq!(flow.popup_results(&results), Some(DeathRequest::Release));
            assert_eq!(flow.popup_results(&results), None, "one request per answer");
            flow.sync_popups(&mut stack, Some(&at(12.0)), 60);
            assert!(!stack.is_open(), "wait for authoritative reply");
        }
    }
    #[test]
    fn deathstate_ghost_corpse_range_recovery_both_skins() {
        for skin in [ActiveSkin::Modern, ActiveSkin::Forever] {
            let mut flow = DeathFlow::default();
            let mut stack = PopupStack::default();
            flow.receive(update(DeathStateSnapshot::Ghost));
            assert!(flow.is_ghost());
            flow.sync_popups(&mut stack, Some(&at(100.0)), 60);
            assert!(!stack.is_open());
            flow.sync_popups(&mut stack, Some(&at(15.0)), 60);
            assert!(stack.contains("RECOVER_CORPSE"));
            let results = draw_and_accept(&mut stack, skin, "Resurrect now?", "Accept");
            assert_eq!(flow.popup_results(&results), Some(DeathRequest::Corpse));
            flow.receive(update(DeathStateSnapshot::Alive));
            flow.sync_popups(&mut stack, Some(&at(15.0)), 60);
            assert!(!flow.is_ghost());
            assert!(!stack.is_open());
        }
    }
    #[test]
    fn deathstate_spirit_healer_accept_both_skins() {
        for skin in [ActiveSkin::Modern, ActiveSkin::Forever] {
            let mut flow = DeathFlow::default();
            let mut stack = PopupStack::default();
            flow.receive(update(DeathStateSnapshot::Ghost));
            flow.sync_popups(&mut stack, Some(&at(203.0)), 60);
            assert!(stack.contains("XP_LOSS"));
            let text = stack.visible()[0].spec.text.clone();
            let results = draw_and_accept(&mut stack, skin, &text, "Accept");
            assert_eq!(
                flow.popup_results(&results),
                Some(DeathRequest::SpiritHealer)
            );
        }
    }
}
