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
    healer_requested: bool,
}
impl DeathFlow {
    /// Error-only replies retain the last snapshot and unlock retry.
    pub fn receive(&mut self, update: DeathStateUpdate) -> Option<String> {
        if let Some(snapshot) = update.snapshot {
            if snapshot.state != DeathStateSnapshot::Ghost {
                self.healer_requested = false;
            }
            self.snapshot = Some(snapshot);
        }
        self.pending = false;
        update.error
    }

    /// An explicit interaction with the healer opens XP_LOSS, not arrival at the graveyard.
    pub fn request_spirit_healer(&mut self, position: &DeathPositionSnapshot) -> bool {
        let graveyard = self
            .snapshot
            .as_ref()
            .and_then(|snapshot| snapshot.graveyard.as_ref());
        let available =
            self.is_ghost() && in_range(position, graveyard, shared::death::SPIRIT_HEALER_RANGE);
        if available {
            self.healer_requested = true;
        }
        available
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
            DeathStateSnapshot::Ghost => self.ghost_popup_key(snapshot, position?),
            DeathStateSnapshot::Alive | DeathStateSnapshot::Resurrecting => None,
        }
    }

    fn ghost_popup_key(
        &self,
        snapshot: &DeathSnapshot,
        position: &DeathPositionSnapshot,
    ) -> Option<&'static str> {
        let healer_in_range = in_range(
            position,
            snapshot.graveyard.as_ref(),
            shared::death::SPIRIT_HEALER_RANGE,
        );
        if self.healer_requested && healer_in_range {
            Some(HEALER_POPUP)
        } else if in_range(
            position,
            snapshot.corpse.as_ref(),
            shared::death::CORPSE_RESURRECT_RANGE,
        ) {
            Some(CORPSE_POPUP)
        } else {
            None
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
            self.healer_requested = false;
        }
        self.active = key;
        for stale in DEATH_KEYS
            .into_iter()
            .filter(|candidate| Some(*candidate) != key)
        {
            popups.hide(stale);
        }
        let Some(key) = key else { return };
        if self.pending {
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
            if key == HEALER_POPUP {
                self.healer_requested = false;
            }
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

/// Own corpse marker; a distant corpse stays on the map edge in its true direction.
pub fn corpse_minimap_blip(
    flow: &DeathFlow,
    view: &game_engine_core::minimap_data::MinimapView,
    map_id: u32,
) -> Option<crate::minimap::MinimapBlip> {
    use game_engine_core::minimap_data::MapMask;
    let corpse = flow
        .corpse()
        .filter(|corpse| u32::from(corpse.map_id) == map_id)?;
    let right = (corpse.z - view.center[1]) / view.diameter;
    let down = (view.center[0] - corpse.x) / view.diameter;
    // Keep the entire arrow inside both skins' map masks.
    const EDGE_OFFSET: f32 = 0.44;
    let distance = match view.mask {
        MapMask::Round => right.hypot(down),
        MapMask::Square => right.abs().max(down.abs()),
    };
    let scale = EDGE_OFFSET / distance.max(EDGE_OFFSET);
    Some(crate::minimap::MinimapBlip {
        unit: 0,
        kind: crate::minimap::BlipKind::Corpse,
        offset: [right * scale, down * scale],
    })
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
        "If you find your corpse, you can resurrect for no penalty.  If I resurrect you all of your equipped items will take 25% durability damage.".into()
    } else {
        "If you find your corpse, you can resurrect for no penalty.  If I resurrect you all of your equipped items will take 25% durability damage and you will be afflicted by 10 minutes of |cff71d5ff|Hspell:15007|h[Resurrection Sickness]|h|r.".into()
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
        assert!(
            matches!(button.widget_data.as_ref(), Some(ui_toolkit::frame::WidgetData::Button(data)) if data.text == label)
        );
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
    fn deathstate_corpse_marker_and_edge_arrow_both_skins() {
        use crate::minimap::{MinimapClusterState, cluster_style, minimap_cluster_screen};
        use game_engine_core::minimap_data::MinimapView;
        for skin in [ActiveSkin::Modern, ActiveSkin::Forever] {
            set_thread_skin(skin);
            let mut flow = DeathFlow::default();
            flow.receive(update(DeathStateSnapshot::Ghost));
            let view = MinimapView::new([12.0, -7.0], 0).masked(cluster_style(skin).mask);
            let marker = corpse_minimap_blip(&flow, &view, 0).expect("own corpse marker");
            assert_eq!(marker.offset, [0.0, 0.0]);
            let distant = MinimapView::new([-2000.0, -7.0], 0).masked(cluster_style(skin).mask);
            let edge = corpse_minimap_blip(&flow, &distant, 0).expect("off-map corpse arrow");
            assert!(edge.offset[1] < -0.4 && edge.offset[1] > -0.5);
            assert!(corpse_minimap_blip(&flow, &view, 1).is_none(), "wrong map");
            let mut shared = SharedContext::new();
            let state = MinimapClusterState {
                blips: vec![edge],
                ..Default::default()
            };
            shared.insert(state.clone());
            let mut registry = FrameRegistry::new(1920.0, 1080.0);
            Screen::new(minimap_cluster_screen).sync(&shared, &mut registry);
            crate::minimap::apply_minimap_postsetup(&state, &mut registry);
            let frame = registry
                .get(
                    registry
                        .get_by_name("MinimapCorpse0")
                        .expect("visible corpse arrow"),
                )
                .unwrap();
            let Some(ui_toolkit::frame::WidgetData::Texture(texture)) = frame.widget_data.as_ref()
            else {
                panic!("corpse arrow texture");
            };
            assert_eq!(
                texture.source,
                ui_toolkit::widgets::texture::TextureSource::FileDataId(
                    crate::minimap::CORPSE_ARROW_FDID
                )
            );
            assert_eq!(texture.rotation, 0.0, "north-pointing edge arrow");
            assert!(
                crate::minimap::minimap_texture_fdids(&state, skin)
                    .contains(&crate::minimap::CORPSE_ARROW_FDID)
            );
            let diagonal = MinimapView::new([1012.0, -1007.0], 0).masked(cluster_style(skin).mask);
            let arrow = corpse_minimap_blip(&flow, &diagonal, 0).unwrap();
            let expected = if skin == ActiveSkin::Modern {
                0.31112698
            } else {
                0.44
            };
            for offset in arrow.offset {
                assert!((offset - expected).abs() < 0.00001);
            }
            flow.receive(update(DeathStateSnapshot::Alive));
            assert!(corpse_minimap_blip(&flow, &view, 0).is_none());
        }
    }

    #[test]
    fn deathstate_wrong_map_and_leaving_range_hide_stale_corpse() {
        let mut flow = DeathFlow::default();
        let mut stack = PopupStack::default();
        flow.receive(update(DeathStateSnapshot::Ghost));
        flow.sync_popups(&mut stack, Some(&at(15.0)), 60);
        let old_popup = stack.visible()[0].clone();
        let mut wrong_map = at(15.0);
        wrong_map.map_id = 1;
        flow.sync_popups(&mut stack, Some(&wrong_map), 60);
        assert!(!stack.is_open());
        assert_eq!(
            flow.popup_results(&[PopupResult {
                id: old_popup.id,
                key: old_popup.spec.key,
                outcome: crate::popup::PopupOutcome::Accepted
            }]),
            None
        );
        flow.sync_popups(&mut stack, Some(&at(15.0)), 60);
        assert!(stack.contains(CORPSE_POPUP));
        flow.sync_popups(&mut stack, Some(&at(100.0)), 60);
        assert!(!stack.is_open());
    }

    #[test]
    fn deathstate_refusal_preserves_snapshot_and_unlocks_retry() {
        let mut flow = DeathFlow::default();
        let mut stack = PopupStack::default();
        flow.receive(update(DeathStateSnapshot::Dead));
        flow.sync_popups(&mut stack, Some(&at(12.0)), 60);
        stack.accept_top();
        assert_eq!(
            flow.popup_results(&stack.drain_results()),
            Some(DeathRequest::Release)
        );
        let error = flow.receive(DeathStateUpdate {
            snapshot: None,
            message: None,
            error: Some("unable to release spirit here".into()),
        });
        assert_eq!(error.as_deref(), Some("unable to release spirit here"));
        flow.sync_popups(&mut stack, Some(&at(12.0)), 60);
        assert!(stack.contains(DEATH_POPUP));
        stack.accept_top();
        assert_eq!(
            flow.popup_results(&stack.drain_results()),
            Some(DeathRequest::Release)
        );
    }

    #[test]
    fn deathstate_healer_cancel_reopen_and_range() {
        let mut flow = DeathFlow::default();
        let mut stack = PopupStack::default();
        flow.receive(update(DeathStateSnapshot::Ghost));
        assert!(!flow.request_spirit_healer(&at(100.0)));
        let mut other_map = at(203.0);
        other_map.map_id = 1;
        assert!(!flow.request_spirit_healer(&other_map));
        assert!(flow.request_spirit_healer(&at(203.0)));
        flow.sync_popups(&mut stack, Some(&at(203.0)), 60);
        stack.cancel_top();
        assert_eq!(flow.popup_results(&stack.drain_results()), None);
        flow.sync_popups(&mut stack, Some(&at(203.0)), 60);
        assert!(!stack.is_open());
        assert!(flow.request_spirit_healer(&at(203.0)));
        flow.sync_popups(&mut stack, Some(&at(203.0)), 60);
        assert!(stack.contains(HEALER_POPUP));
        flow.sync_popups(&mut stack, Some(&at(100.0)), 60);
        assert!(!stack.is_open(), "moving away closes confirmation");
    }

    #[test]
    fn deathstate_healer_requires_interaction() {
        let mut flow = DeathFlow::default();
        let mut stack = PopupStack::default();
        flow.receive(update(DeathStateSnapshot::Ghost));
        flow.sync_popups(&mut stack, Some(&at(203.0)), 60);
        assert!(
            !stack.is_open(),
            "Retail healer confirmation starts on interaction, not release"
        );
    }

    #[test]
    fn deathstate_spirit_healer_accept_both_skins() {
        for skin in [ActiveSkin::Modern, ActiveSkin::Forever] {
            let mut flow = DeathFlow::default();
            let mut stack = PopupStack::default();
            flow.receive(update(DeathStateSnapshot::Ghost));
            flow.sync_popups(&mut stack, Some(&at(203.0)), 60);
            assert!(flow.request_spirit_healer(&at(203.0)));
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
