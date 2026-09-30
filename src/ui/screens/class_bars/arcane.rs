//! `ArcaneChargeTemplate` / `MageArcaneChargesFrame` (MageArcaneChargesBar.xml:5-136,
//! MageArcaneChargesBar.lua), atlas 2378 `uimagearcanecharge.blp` FDID 5045210.

use super::super::inworld_unit_frames_art::{AtlasArt, FlipBook, art, key};
use super::anim::{PointTemplate, group, layer};
use super::{ActivePoints, BarLogic, ClassBarView, Row, Set, SetActive};
use crate::status::{ClassBar, ClassBarResource};

const fn arcane(rect: (f32, f32, f32, f32)) -> AtlasArt {
    art(5_045_210, (512.0, 256.0), rect)
}

const CIRCLE: usize = 2;
const TRIANGLE: usize = 3;
const SQUARE: usize = 4;
const DIAMOND: usize = 5;
const ICON: usize = 6;
const FLARE: usize = 8;
const SHOCK: usize = 9;
const OUTER: usize = 10;
const GLOW: usize = 11;

static TEMPLATE: PointTemplate = PointTemplate {
    layers: &[
        // BACKGROUND: UF-Arcane-BGShadow (21048) CENTER y -2.5, UF-Arcane-BG (21047).
        layer(
            "ArcaneBGShadow",
            arcane((1.0, 28.0, 228.0, 255.0)),
            (0.0, -2.5),
        ),
        layer("ArcaneBG", arcane((56.0, 77.0, 228.0, 249.0)), (0.0, 0.0)),
        // BORDER: UF-Arcane-MagicCirc (21052), -MagicTriangle (21055), -MagicSquare
        // (21054), -MagicDiamond (21053).
        layer(
            "ArcaneCircle",
            arcane((343.0, 380.0, 1.0, 38.0)),
            (0.0, 0.0),
        ),
        layer(
            "ArcaneTriangle",
            arcane((303.0, 341.0, 1.0, 39.0)),
            (0.0, 0.0),
        ),
        layer(
            "ArcaneSquare",
            arcane((30.0, 54.0, 228.0, 252.0)),
            (0.0, 0.0),
        ),
        layer(
            "ArcaneDiamond",
            arcane((382.0, 418.0, 1.0, 37.0)),
            (0.0, 0.0),
        ),
        // ARTWORK: UF-Arcane-Icon (21051), UF-Arcane-Orb (21056).
        layer("ArcaneIcon", arcane((79.0, 98.0, 228.0, 247.0)), (0.0, 0.0)),
        layer("Orb", arcane((100.0, 118.0, 228.0, 246.0)), (0.0, 0.0)),
        // OVERLAY: UF-Arcane-Flare (21049), UF-Arcane-ShockFX (21058) 50×45 at y 1,
        // UF-Arcane-OuterFX (21057), UF-Arcane-FrameGlow (21050).
        layer("ArcaneFlare", arcane((420.0, 455.0, 1.0, 34.0)), (0.0, 0.0)),
        layer("FBArcaneFX", arcane((1.0, 301.0, 1.0, 226.0)), (0.0, 1.0)).sized((50.0, 45.0)),
        layer(
            "ArcaneOuterFX",
            arcane((120.0, 138.0, 228.0, 246.0)),
            (0.0, 0.0),
        ),
        layer("FrameGlow", arcane((457.0, 486.0, 1.0, 30.0)), (0.0, 0.0)),
    ],
    groups: &[
        // activateAnim (MageArcaneChargesBar.xml:79-102).
        group(
            &[
                (GLOW, key(0.0, 0.45, 0.0, 0.17)),
                (GLOW, key(0.45, 0.7, 0.17, 0.6)),
                (GLOW, key(0.7, 0.7, 0.77, 0.2)),
                (GLOW, key(0.7, 0.0, 0.97, 0.2)),
                (SHOCK, key(0.0, 1.0, 0.0, 0.0)),
                (ICON, key(0.0, 0.5, 0.0, 0.1)),
                (ICON, key(0.5, 1.0, 0.47, 0.13)),
                (FLARE, key(0.0, 0.0, 0.0, 0.23)),
                (FLARE, key(0.0, 1.0, 0.23, 0.23)),
                (FLARE, key(1.0, 0.65, 0.46, 0.27)),
                (FLARE, key(0.65, 0.0, 0.73, 0.27)),
                (DIAMOND, key(0.0, 1.0, 0.0, 0.33)),
                (DIAMOND, key(1.0, 0.0, 0.33, 0.57)),
                (SQUARE, key(0.0, 1.0, 0.0, 0.27)),
                (SQUARE, key(1.0, 0.0, 0.27, 0.63)),
                (CIRCLE, key(0.0, 1.0, 0.0, 0.17)),
                (CIRCLE, key(1.0, 1.0, 0.17, 0.33)),
                (CIRCLE, key(1.0, 0.0, 0.5, 0.5)),
                (TRIANGLE, key(0.0, 0.0, 0.0, 0.17)),
                (TRIANGLE, key(0.0, 1.0, 0.17, 0.1)),
                (TRIANGLE, key(1.0, 0.0, 0.27, 0.53)),
            ],
            &[(
                SHOCK,
                FlipBook {
                    columns: 6,
                    rows: 5,
                    frames: 28,
                    duration: 1.0,
                },
            )],
        ),
        // deactivateAnim (MageArcaneChargesBar.xml:103-111).
        group(
            &[
                (GLOW, key(0.0, 1.0, 0.0, 0.08)),
                (GLOW, key(1.0, 1.0, 0.08, 0.035)),
                (GLOW, key(1.0, 0.0, 0.12, 0.235)),
                (ICON, key(1.0, 0.0, 0.0, 0.15)),
                (FLARE, key(1.0, 0.0, 0.0, 0.25)),
                (OUTER, key(1.0, 1.0, 0.0, 0.15)),
                (OUTER, key(1.0, 0.0, 0.15, 0.235)),
            ],
            &[],
        ),
    ],
};

/// `ArcaneChargeMixin` (MageArcaneChargesBar.lua:11-47): `ResetVisuals` zeroes the
/// `fxTextures`; `SetActive` restarts `activateAnim` or `deactivateAnim`.
static MIXIN: SetActive = SetActive {
    template: &TEMPLATE,
    reset: &[
        Set::Alpha(CIRCLE, 0.0),
        Set::Alpha(TRIANGLE, 0.0),
        Set::Alpha(SQUARE, 0.0),
        Set::Alpha(DIAMOND, 0.0),
        Set::Alpha(ICON, 0.0),
        Set::Alpha(FLARE, 0.0),
        Set::Alpha(SHOCK, 0.0),
        Set::Alpha(OUTER, 0.0),
        Set::Alpha(GLOW, 0.0),
    ],
    activate: (0, &[]),
    deactivate: (1, &[]),
};

/// 21×21 charges 10 apart, `topPadding` 7 (MageArcaneChargesBar.xml:6,125,134).
const ROW: Row = Row {
    cell: (21.0, 21.0),
    spacing: 10.0,
    padding: (7.0, 0.0),
    scale: 1.0,
};

#[derive(Debug)]
pub struct Bar(ActivePoints);

impl Default for Bar {
    fn default() -> Self {
        Self(ActivePoints::new(&MIXIN))
    }
}

impl BarLogic for Bar {
    /// `MagePowerBar:UpdatePower`: `SetActive(i <= charges)`.
    fn power(&mut self, resource: &ClassBarResource, now: f64) {
        self.0.power(resource.current, resource.max, now);
    }

    fn tick(&mut self, now: f64) {
        self.0.tick(now);
    }

    fn view(&self, _: &ClassBarResource, now: f64) -> ClassBarView {
        ROW.view(ClassBar::ArcaneCharges, self.0.visuals(), now)
    }
}
