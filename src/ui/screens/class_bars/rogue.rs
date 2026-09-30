//! `RogueComboPointTemplate` / `RogueComboPointBarFrame` (RogueComboPointBar.xml:5-249,
//! RogueComboPointBar.lua), atlas 2302 `uiroguecombpoints.blp` FDID 4902605.

use super::super::inworld_unit_frames_art::{AlphaKey, AtlasArt, FlipBook, art, key};
use super::anim::{AnimGroup, PointTemplate, PointVisual, group, layer};
use super::{BarLogic, ClassBarView, Row};
use crate::status::{ClassBar, ClassBarResource};

const fn rogue(rect: (f32, f32, f32, f32)) -> AtlasArt {
    art(4_902_605, (512.0, 512.0), rect)
}

const BG_ACTIVE: usize = 1;
const BG_INACTIVE: usize = 2;
const BG_GLOW: usize = 3;
const ICON: usize = 4;
const ICON_CHARGED: usize = 5;
const FX: usize = 6;
const FX_CHARGED: usize = 7;
const CHARGED_INACTIVE: usize = 8;
const CHARGED_ACTIVE: usize = 9;
const CHARGED_GLOW: usize = 10;
const FRAME_GLOW: usize = 11;
const SLASH: usize = 12;
const SLASH_CHARGED: usize = 13;
/// `fxTextures`: every texture but `BGShadow`.
const FX_TEXTURES: std::ops::Range<usize> = 1..14;

/// `<FlipBook duration=".57" flipBookRows="3" flipBookColumns="6" flipBookFrames="17">`.
const SLASH_BOOK: FlipBook = FlipBook {
    columns: 6,
    rows: 3,
    frames: 17,
    duration: 0.57,
};

/// A slash: shown at once, then its flipbook.
const fn slash(layer: usize) -> (usize, AlphaKey) {
    (layer, key(0.0, 1.0, 0.0, 0.0))
}

static TEMPLATE: PointTemplate = PointTemplate {
    layers: &[
        // BACKGROUND: uf-roguecp-bg-shadow (19990) y -4, uf-roguecp-bg (19991),
        // uf-roguecp-bg-dis (19989), and BGGlow on uf-roguecp-bg again.
        layer("BGShadow", rogue((261.0, 286.0, 39.0, 64.0)), (0.0, -4.0)),
        layer("BGActive", rogue((299.0, 319.0, 1.0, 21.0)), (0.0, 0.0)),
        layer("BGInactive", rogue((299.0, 319.0, 67.0, 87.0)), (0.0, 0.0)),
        layer("BGGlow", rogue((299.0, 319.0, 1.0, 21.0)), (0.0, 0.0)),
        // ARTWORK 1: uf-roguecp-icon-red (19993), -icon-blue (19992).
        layer(
            "IconUncharged",
            rogue((299.0, 313.0, 106.0, 121.0)),
            (0.0, 0.0),
        ),
        layer(
            "IconCharged",
            rogue((299.0, 313.0, 89.0, 104.0)),
            (0.0, 0.0),
        ),
        // ARTWORK 2: uf-roguecp-fx-red (20007), -fx-blue (20006).
        layer("FXUncharged", rogue((321.0, 335.0, 17.0, 31.0)), (0.0, 0.0)),
        layer("FXCharged", rogue((321.0, 335.0, 1.0, 15.0)), (0.0, 0.0)),
        // ARTWORK 3: uf-roguecp-bg-anima-dis (19986), -bg-anima (19987), -bg-animaglow
        // (19988).
        layer(
            "ChargedFrameInactive",
            rogue((299.0, 319.0, 45.0, 65.0)),
            (0.0, 0.0),
        ),
        layer(
            "ChargedFrameActive",
            rogue((299.0, 319.0, 23.0, 43.0)),
            (0.0, 0.0),
        ),
        layer(
            "ChargedFrameGlow",
            rogue((261.0, 285.0, 66.0, 90.0)),
            (0.0, 0.0),
        ),
        // OVERLAY: uf-roguecp-frame-glow (20005), 43×43 uf-roguecp-slash-red (19995) and
        // -slash-blue (19994).
        layer("FrameGlow", rogue((261.0, 284.0, 92.0, 115.0)), (0.0, 0.0)),
        layer(
            "SlashFBUncharged",
            rogue((1.0, 259.0, 132.0, 261.0)),
            (0.0, 0.0),
        )
        .sized((43.0, 43.0)),
        layer(
            "SlashFBCharged",
            rogue((1.0, 259.0, 1.0, 130.0)),
            (0.0, 0.0),
        )
        .sized((43.0, 43.0)),
    ],
    groups: &GROUPS,
};

/// `transitionAnims` (RogueComboPointBar.xml:93-224), indexed as [`transition`] names them.
static GROUPS: [AnimGroup; 13] = [
    // 0 unchargedEmpty
    group(&[(BG_INACTIVE, key(0.0, 1.0, 0.0, 0.1))], &[]),
    // 1 unchargedEmptyToUnchargedFull
    group(
        &[
            slash(SLASH),
            (ICON, key(0.0, 0.5, 0.0, 0.1)),
            (ICON, key(0.5, 1.0, 0.27, 0.27)),
            (BG_ACTIVE, key(0.0, 0.0, 0.0, 0.2)),
            (BG_ACTIVE, key(0.0, 1.0, 0.2, 0.17)),
            (BG_INACTIVE, key(1.0, 1.0, 0.0, 0.37)),
            (BG_INACTIVE, key(1.0, 0.0, 0.37, 0.1)),
            (BG_GLOW, key(0.0, 1.0, 0.0, 0.17)),
            (BG_GLOW, key(1.0, 0.0, 0.17, 0.4)),
        ],
        &[(SLASH, SLASH_BOOK)],
    ),
    // 2 unchargedEmptyToChargedFull
    group(
        &[
            slash(SLASH_CHARGED),
            (CHARGED_GLOW, key(1.0, 1.0, 0.0, 0.17)),
            (CHARGED_GLOW, key(1.0, 0.0, 0.17, 0.33)),
            (CHARGED_ACTIVE, key(0.0, 0.0, 0.0, 0.2)),
            (CHARGED_ACTIVE, key(0.0, 1.0, 0.2, 0.17)),
            (ICON_CHARGED, key(0.0, 0.5, 0.0, 0.1)),
            (ICON_CHARGED, key(0.5, 1.0, 0.27, 0.27)),
            (CHARGED_INACTIVE, key(0.0, 0.5, 0.0, 0.1)),
            (CHARGED_INACTIVE, key(0.5, 0.0, 0.37, 0.13)),
            (BG_GLOW, key(1.0, 0.0, 0.0, 0.47)),
        ],
        &[(SLASH_CHARGED, SLASH_BOOK)],
    ),
    // 3 unchargedEmptyToChargedEmpty
    group(
        &[
            (CHARGED_GLOW, key(1.0, 1.0, 0.0, 0.17)),
            (CHARGED_GLOW, key(1.0, 0.0, 0.17, 0.33)),
            (BG_GLOW, key(1.0, 0.0, 0.0, 0.47)),
            (CHARGED_INACTIVE, key(1.0, 1.0, 0.0, 0.17)),
        ],
        &[],
    ),
    // 4 chargedEmptyToChargedFull
    group(
        &[
            slash(SLASH_CHARGED),
            (CHARGED_ACTIVE, key(0.0, 0.0, 0.0, 0.2)),
            (CHARGED_ACTIVE, key(0.0, 1.0, 0.2, 0.17)),
            (ICON_CHARGED, key(0.0, 0.5, 0.0, 0.1)),
            (ICON_CHARGED, key(0.5, 1.0, 0.27, 0.27)),
            (CHARGED_INACTIVE, key(1.0, 0.5, 0.0, 0.1)),
            (CHARGED_INACTIVE, key(0.5, 0.0, 0.37, 0.13)),
            (BG_GLOW, key(1.0, 0.0, 0.0, 0.47)),
        ],
        &[(SLASH_CHARGED, SLASH_BOOK)],
    ),
    // 5 chargedEmptyToUnchargedFull
    group(
        &[
            slash(SLASH),
            (ICON, key(0.0, 0.5, 0.0, 0.1)),
            (ICON, key(0.5, 1.0, 0.27, 0.27)),
            (BG_ACTIVE, key(0.0, 0.0, 0.0, 0.2)),
            (BG_ACTIVE, key(0.0, 1.0, 0.2, 0.17)),
            (BG_GLOW, key(0.0, 1.0, 0.0, 0.17)),
            (BG_GLOW, key(1.0, 0.0, 0.17, 0.4)),
            (CHARGED_INACTIVE, key(1.0, 0.5, 0.0, 0.1)),
            (CHARGED_INACTIVE, key(0.5, 0.0, 0.17, 0.33)),
        ],
        &[(SLASH, SLASH_BOOK)],
    ),
    // 6 chargedEmptyToUnchargedEmpty
    group(
        &[
            (CHARGED_GLOW, key(1.0, 1.0, 0.0, 0.17)),
            (CHARGED_GLOW, key(1.0, 0.0, 0.17, 0.33)),
            (BG_GLOW, key(1.0, 0.0, 0.0, 0.47)),
            (CHARGED_INACTIVE, key(1.0, 1.0, 0.0, 0.17)),
            (CHARGED_INACTIVE, key(1.0, 0.0, 0.17, 0.33)),
            (BG_INACTIVE, key(0.0, 0.0, 0.0, 0.37)),
            (BG_INACTIVE, key(0.0, 1.0, 0.37, 0.1)),
        ],
        &[],
    ),
    // 7 unchargedFullToUnchargedEmpty
    group(
        &[
            (FRAME_GLOW, key(1.0, 0.0, 0.0, 0.5)),
            (ICON, key(1.0, 0.0, 0.0, 0.17)),
            (FX, key(1.0, 0.0, 0.0, 0.4)),
            (BG_ACTIVE, key(1.0, 1.0, 0.0, 0.2)),
            (BG_ACTIVE, key(1.0, 0.0, 0.2, 0.17)),
            (BG_INACTIVE, key(0.0, 0.0, 0.0, 0.37)),
            (BG_INACTIVE, key(0.0, 1.0, 0.37, 0.1)),
        ],
        &[],
    ),
    // 8 unchargedFullToChargedFull
    group(
        &[
            (CHARGED_GLOW, key(1.0, 1.0, 0.0, 0.17)),
            (CHARGED_GLOW, key(1.0, 0.0, 0.17, 0.33)),
            (BG_GLOW, key(1.0, 0.0, 0.0, 0.47)),
            (CHARGED_ACTIVE, key(0.0, 0.0, 0.0, 0.2)),
            (CHARGED_ACTIVE, key(0.0, 1.0, 0.2, 0.17)),
            (BG_ACTIVE, key(1.0, 1.0, 0.0, 0.2)),
            (BG_ACTIVE, key(1.0, 0.0, 0.2, 0.17)),
            (ICON_CHARGED, key(0.0, 0.0, 0.0, 0.27)),
            (ICON_CHARGED, key(0.0, 1.0, 0.27, 0.27)),
            (ICON, key(1.0, 1.0, 0.0, 0.27)),
            (ICON, key(1.0, 0.0, 0.27, 0.27)),
        ],
        &[],
    ),
    // 9 unchargedFullToChargedEmpty
    group(
        &[
            (CHARGED_GLOW, key(1.0, 0.0, 0.0, 0.5)),
            (ICON, key(1.0, 0.0, 0.0, 0.17)),
            (FX, key(1.0, 0.0, 0.0, 0.4)),
            (BG_ACTIVE, key(1.0, 1.0, 0.0, 0.2)),
            (BG_ACTIVE, key(1.0, 0.0, 0.2, 0.17)),
            (CHARGED_INACTIVE, key(0.0, 0.0, 0.0, 0.17)),
            (CHARGED_INACTIVE, key(0.0, 1.0, 0.17, 0.33)),
        ],
        &[],
    ),
    // 10 chargedFullToChargedEmpty
    group(
        &[
            (CHARGED_GLOW, key(1.0, 0.0, 0.0, 0.5)),
            (ICON_CHARGED, key(1.0, 0.0, 0.0, 0.17)),
            (FX_CHARGED, key(1.0, 0.0, 0.0, 0.4)),
            (CHARGED_INACTIVE, key(0.0, 0.0, 0.0, 0.17)),
            (CHARGED_INACTIVE, key(0.0, 1.0, 0.17, 0.33)),
        ],
        &[],
    ),
    // 11 chargedFullToUnchargedEmpty
    group(
        &[
            (CHARGED_GLOW, key(1.0, 0.0, 0.0, 0.5)),
            (ICON_CHARGED, key(1.0, 0.0, 0.0, 0.17)),
            (FX_CHARGED, key(1.0, 0.0, 0.0, 0.4)),
            (CHARGED_INACTIVE, key(0.0, 0.0, 0.0, 0.17)),
            (BG_INACTIVE, key(0.0, 0.0, 0.0, 0.37)),
            (BG_INACTIVE, key(0.0, 1.0, 0.37, 0.1)),
        ],
        &[],
    ),
    // 12 chargedFullToUnchargedFull
    group(
        &[
            (CHARGED_GLOW, key(1.0, 1.0, 0.0, 0.17)),
            (CHARGED_GLOW, key(1.0, 0.0, 0.17, 0.33)),
            (BG_GLOW, key(1.0, 0.0, 0.0, 0.47)),
            (CHARGED_ACTIVE, key(1.0, 0.0, 0.0, 0.2)),
            (BG_ACTIVE, key(0.0, 0.0, 0.0, 0.2)),
            (BG_ACTIVE, key(0.0, 1.0, 0.2, 0.17)),
            (ICON_CHARGED, key(1.0, 1.0, 0.0, 0.27)),
            (ICON_CHARGED, key(1.0, 0.0, 0.27, 0.27)),
            (ICON, key(0.0, 0.0, 0.0, 0.27)),
            (ICON, key(0.0, 1.0, 0.27, 0.27)),
        ],
        &[],
    ),
];

/// `(isCharged, isFull)` of a combo point.
type PointState = (bool, bool);

/// `RogueComboPointTransitions` (RogueComboPointBar.lua:79-120): the group for a change.
fn transition(from: PointState, to: PointState) -> Option<usize> {
    let (uncharged, charged, empty, full) = (false, true, false, true);
    let table = [
        ((uncharged, empty), (uncharged, empty), 0),
        ((uncharged, empty), (uncharged, full), 1),
        ((uncharged, empty), (charged, full), 2),
        ((uncharged, empty), (charged, empty), 3),
        ((charged, empty), (charged, full), 4),
        ((charged, empty), (uncharged, full), 5),
        ((charged, empty), (uncharged, empty), 6),
        ((uncharged, full), (uncharged, empty), 7),
        ((uncharged, full), (charged, full), 8),
        ((uncharged, full), (charged, empty), 9),
        ((charged, full), (charged, empty), 10),
        ((charged, full), (uncharged, empty), 11),
        ((charged, full), (uncharged, full), 12),
    ];
    table
        .iter()
        .find(|(f, t, _)| *f == from && *t == to)
        .map(|(_, _, group)| *group)
}

fn reset(visual: &mut PointVisual, now: f64) {
    visual.stop_all(now);
    for layer in FX_TEXTURES {
        visual.set_alpha(layer, 0.0);
    }
}

/// 20×20 points 4 apart, `topPadding` 10 (RogueComboPointBar.xml:6,237,245).
fn row(max: u8) -> Row {
    Row {
        cell: (20.0, 20.0),
        spacing: 4.0,
        padding: (10.0, left_padding(max)),
        scale: 1.0,
    }
}

/// `RogueComboPointBarMixin:UpdateMaxPower`: 0 up to 5 points, then -20 per extra point
/// (RogueComboPointBar.lua:1-33).
fn left_padding(max: u8) -> f32 {
    -20.0 * f32::from(max.saturating_sub(5))
}

#[derive(Debug, Default)]
pub struct Bar {
    /// Each point and its `(isCharged, isFull)`, `None` after `Setup`.
    points: Vec<(PointVisual, Option<PointState>)>,
}

impl BarLogic for Bar {
    /// `RogueComboPointBarMixin:UpdatePower`: `Update(i <= comboPoints, isCharged)`.
    /// Replicated charged indices match `GetUnitChargedPowerPoints` (1-based).
    fn power(&mut self, resource: &ClassBarResource, now: f64) {
        if self.points.len() != usize::from(resource.max) {
            self.points = (0..resource.max)
                .map(|_| {
                    let mut visual = PointVisual::new(&TEMPLATE);
                    reset(&mut visual, now);
                    (visual, None)
                })
                .collect();
        }
        for (index, (visual, state)) in self.points.iter_mut().enumerate() {
            let charged = resource
                .dynamics
                .charged_points
                .contains(&((index + 1) as u8));
            let next = (charged, index < usize::from(resource.current));
            if *state == Some(next) {
                continue;
            }
            let from = state.unwrap_or((false, false));
            *state = Some(next);
            reset(visual, now);
            if let Some(group) = transition(from, next) {
                visual.restart(group, now);
            }
        }
    }

    fn tick(&mut self, now: f64) {
        for (visual, _) in &mut self.points {
            visual.finish(now);
        }
    }

    fn view(&self, resource: &ClassBarResource, now: f64) -> ClassBarView {
        let visuals = self.points.iter().map(|(visual, _)| visual);
        row(resource.max).view(ClassBar::RogueComboPoints, visuals, now)
    }
}
